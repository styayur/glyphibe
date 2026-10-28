// SPDX-License-Identifier: MPL-2.0
//! Original deterministic implementation of multilevel Louvain modularity optimization.
//! Algorithm reference: Blondel et al. (2008), https://arxiv.org/abs/0803.0476v2.
//! Structural communities do not assert business meaning.
use glyphibe_core::{
    ChangeStatus, Diagnostic, Evidence, Graph, Node, NodeKind, Plane, Relation, fingerprint,
    stable_id,
};
use std::collections::{BTreeMap, BTreeSet};

type Adjacency = Vec<BTreeMap<usize, f64>>;

/// Cyclic strongly connected components, using iterative Kosaraju traversal.
/// Stale edges, removed nodes, Agent proposals and explanatory relationships are excluded.
pub fn cycles(graph: &Graph) -> Vec<Vec<String>> {
    let nodes: Vec<_> = graph
        .nodes
        .values()
        .filter(|n| n.plane == Plane::Architecture && n.change != ChangeStatus::Removed)
        .map(|n| n.id.clone())
        .collect();
    let index: BTreeMap<_, _> = nodes
        .iter()
        .enumerate()
        .map(|(i, id)| (id.clone(), i))
        .collect();
    let mut forward = vec![vec![]; nodes.len()];
    let mut reverse = forward.clone();
    let mut self_loops = BTreeSet::new();
    for edge in graph.edges.iter().filter(|e| {
        !e.stale
            && e.relation.is_dependency()
            && e.evidence.kind != glyphibe_core::EvidenceKind::Proposed
    }) {
        if let (Some(&a), Some(&b)) = (index.get(&edge.source), index.get(&edge.target)) {
            forward[a].push(b);
            reverse[b].push(a);
            if a == b {
                self_loops.insert(a);
            }
        }
    }
    let mut visited = vec![false; nodes.len()];
    let mut order = vec![];
    for start in 0..nodes.len() {
        if visited[start] {
            continue;
        }
        let mut stack = vec![(start, false)];
        while let Some((node, finished)) = stack.pop() {
            if finished {
                order.push(node);
                continue;
            }
            if visited[node] {
                continue;
            }
            visited[node] = true;
            stack.push((node, true));
            for next in forward[node].iter().rev() {
                if !visited[*next] {
                    stack.push((*next, false));
                }
            }
        }
    }
    visited.fill(false);
    let mut result = vec![];
    for start in order.into_iter().rev() {
        if visited[start] {
            continue;
        }
        let mut component = vec![];
        let mut stack = vec![start];
        visited[start] = true;
        while let Some(node) = stack.pop() {
            component.push(node);
            for next in &reverse[node] {
                if !visited[*next] {
                    visited[*next] = true;
                    stack.push(*next);
                }
            }
        }
        if component.len() > 1 || self_loops.contains(&start) {
            let mut members: Vec<_> = component.into_iter().map(|i| nodes[i].clone()).collect();
            members.sort();
            result.push(members);
        }
    }
    result.sort();
    result
}

/// Deterministic Louvain: local modularity moves followed by weighted aggregation.
/// Inputs use symmetric adjacency; diagonal entries count twice a self-loop weight.
pub fn louvain(adjacency: &Adjacency, resolution: f64) -> Vec<usize> {
    let n = adjacency.len();
    if n == 0 {
        return vec![];
    }
    let gamma = if resolution.is_finite() && resolution > 0.0 {
        resolution
    } else {
        1.0
    };
    let mut original_groups: Vec<Vec<usize>> = (0..n).map(|i| vec![i]).collect();
    let mut current = adjacency.clone();
    for _ in 0..64 {
        let local = local_moves(&current, gamma);
        let mut compact = BTreeMap::new();
        for community in &local {
            let next = compact.len();
            compact.entry(*community).or_insert(next);
        }
        let count = compact.len();
        let mut groups = vec![vec![]; count];
        for (index, community) in local.iter().enumerate() {
            groups[compact[community]].extend_from_slice(&original_groups[index]);
        }
        if count == current.len() {
            original_groups = groups;
            break;
        }
        let mut aggregated = vec![BTreeMap::new(); count];
        for (i, neighbors) in current.iter().enumerate() {
            for (j, weight) in neighbors {
                *aggregated[compact[&local[i]]]
                    .entry(compact[&local[*j]])
                    .or_insert(0.0) += weight;
            }
        }
        original_groups = groups;
        current = aggregated;
    }
    // Canonical labels by smallest original node keep IDs independent of aggregation order.
    original_groups.sort_by_key(|g| g.iter().min().copied().unwrap_or(usize::MAX));
    let mut result = vec![0; n];
    for (label, group) in original_groups.iter().enumerate() {
        for index in group {
            result[*index] = label;
        }
    }
    result
}

fn local_moves(adjacency: &Adjacency, gamma: f64) -> Vec<usize> {
    let n = adjacency.len();
    let degrees: Vec<f64> = adjacency
        .iter()
        .map(|neighbors| neighbors.values().sum())
        .collect();
    let mass: f64 = degrees.iter().sum();
    let mut partition: Vec<usize> = (0..n).collect();
    if mass <= 0.0 {
        return partition;
    }
    let mut totals = degrees.clone();
    for _ in 0..100 {
        let mut changed = false;
        for i in 0..n {
            if degrees[i] == 0.0 {
                continue;
            }
            let old = partition[i];
            let mut weights = BTreeMap::<usize, f64>::new();
            for (neighbor, weight) in &adjacency[i] {
                if *neighbor != i {
                    *weights.entry(partition[*neighbor]).or_default() += weight;
                }
            }
            totals[old] -= degrees[i];
            let mut best = old;
            let mut best_score =
                weights.get(&old).copied().unwrap_or(0.0) - gamma * degrees[i] * totals[old] / mass;
            for (community, weight) in weights {
                let score = weight - gamma * degrees[i] * totals[community] / mass;
                if score > best_score + 1e-12 {
                    best = community;
                    best_score = score;
                }
            }
            totals[best] += degrees[i];
            if best != old {
                partition[i] = best;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    partition
}

/// Calculate generalized weighted modularity for validation and reports.
pub fn modularity(adjacency: &Adjacency, partition: &[usize], resolution: f64) -> f64 {
    let degrees: Vec<f64> = adjacency.iter().map(|n| n.values().sum()).collect();
    let mass: f64 = degrees.iter().sum();
    if mass == 0.0 || partition.len() != adjacency.len() {
        return 0.0;
    }
    let mut totals = BTreeMap::<usize, f64>::new();
    let mut internal = 0.0;
    for (i, neighbors) in adjacency.iter().enumerate() {
        *totals.entry(partition[i]).or_default() += degrees[i];
        for (j, weight) in neighbors {
            if partition[i] == partition[*j] {
                internal += weight;
            }
        }
    }
    internal / mass - resolution * totals.values().map(|sum| (sum / mass).powi(2)).sum::<f64>()
}

fn derived_node(
    graph: &Graph,
    id: String,
    label: String,
    entity: (NodeKind, u8),
    summary: String,
    evidence: Evidence,
    tags: Vec<String>,
) -> Node {
    Node {
        id,
        label,
        kind: entity.0,
        level: entity.1,
        plane: Plane::Architecture,
        repository: graph.repository.clone(),
        revision: graph.revision.clone(),
        language: None,
        source: None,
        fingerprint: fingerprint(&summary),
        summary,
        evidence,
        tags,
        knowledge: vec![],
        change: ChangeStatus::Added,
    }
}

/// Full reclustering of the current static graph. Cached parsing is preserved.
/// Components reflect directory evidence; communities reflect connectivity only.
pub fn enrich(graph: &mut Graph, resolution: f64) {
    graph.diagnostics.retain(|d| d.code != "dependency_cycle");
    for node in graph.nodes.values_mut() {
        node.tags.retain(|t| !t.starts_with("cycle:"));
    }
    for members in cycles(graph) {
        let cycle_id = stable_id(&members.iter().map(String::as_str).collect::<Vec<_>>());
        for member in &members {
            if let Some(node) = graph.nodes.get_mut(member) {
                node.tags.push(format!("cycle:{cycle_id}"));
            }
        }
        graph.diagnostics.push(Diagnostic{code:"dependency_cycle".into(),message:format!("Static dependency cycle {cycle_id} contains {} node(s); recursion may be intentional",members.len()),path:None});
    }
    if !resolution.is_finite() || resolution <= 0.0 {
        graph.diagnostics.push(Diagnostic {
            code: "invalid_resolution".into(),
            message: "Louvain resolution must be finite and positive; using 1.0".into(),
            path: None,
        });
    }
    let resolution = if resolution.is_finite() && resolution > 0.0 {
        resolution
    } else {
        1.0
    };
    let generated: BTreeSet<_> = graph
        .nodes
        .values()
        .filter(|n| n.tags.iter().any(|t| t == "glyphibe:derived-structure"))
        .map(|n| n.id.clone())
        .collect();
    graph.nodes.retain(|id, _| !generated.contains(id));
    graph
        .edges
        .retain(|e| !generated.contains(&e.source) && !generated.contains(&e.target));
    graph.reindex_edges();
    let domain_id = stable_id(&[&graph.repository, "repository-domain"]);
    let domain = derived_node(
        graph,
        domain_id.clone(),
        "Repository".into(),
        (NodeKind::Domain, 1),
        "Repository scope; business domain has not been established".into(),
        Evidence::observed("selected repository scope", vec![]),
        vec![
            "glyphibe:derived-structure".into(),
            "business-domain:unverified".into(),
        ],
    );
    graph.add_node(domain);
    let file_nodes: Vec<_> = graph
        .nodes
        .values()
        .filter(|n| n.kind == NodeKind::File && n.change != ChangeStatus::Removed)
        .cloned()
        .collect();
    let mut components = BTreeMap::<String, (String, Vec<String>)>::new();
    for file in &file_nodes {
        let Some(source) = &file.source else {
            continue;
        };
        let directory = source
            .path
            .rsplit_once('/')
            .map(|(p, _)| p)
            .unwrap_or(".")
            .to_owned();
        let component_id = stable_id(&[&graph.repository, "directory-component", &directory]);
        components
            .entry(directory)
            .or_insert_with(|| (component_id, vec![]))
            .1
            .push(file.id.clone());
    }
    for (directory, (id, files)) in &components {
        let evidence = Evidence::observed(
            "directory boundary; not a verified business component",
            files
                .iter()
                .take(64)
                .filter_map(|id| graph.nodes[id].source.clone())
                .collect(),
        );
        graph.add_node(derived_node(
            graph,
            id.clone(),
            format!("Directory {directory}"),
            (NodeKind::Component, 3),
            format!("Directory {directory} groups {} source files", files.len()),
            evidence.clone(),
            vec![
                "glyphibe:derived-structure".into(),
                "component-kind:directory".into(),
            ],
        ));
        graph.add_edge(&domain_id, id, Relation::Contains, evidence.clone());
        for file in files {
            graph.add_edge(
                id,
                file,
                Relation::Contains,
                Evidence::observed(
                    "directory membership",
                    graph.nodes[file].source.clone().into_iter().collect(),
                ),
            );
        }
    }
    let mut selected: Vec<_> = graph
        .nodes
        .values()
        .filter(|n| {
            n.plane == Plane::Architecture
                && n.kind == NodeKind::Function
                && n.change != ChangeStatus::Removed
        })
        .map(|n| n.id.clone())
        .collect();
    if selected.is_empty() {
        selected = file_nodes.iter().map(|n| n.id.clone()).collect();
    }
    selected.sort();
    let index: BTreeMap<_, _> = selected
        .iter()
        .enumerate()
        .map(|(i, id)| (id.clone(), i))
        .collect();
    let mut adjacency = vec![BTreeMap::new(); selected.len()];
    for edge in graph.edges.iter().filter(|e| {
        !e.stale
            && matches!(
                e.relation,
                Relation::Calls | Relation::References | Relation::Imports | Relation::DependsOn
            )
    }) {
        if let (Some(&i), Some(&j)) = (index.get(&edge.source), index.get(&edge.target)) {
            let weight = if edge.relation == Relation::References {
                0.25
            } else {
                1.0
            };
            *adjacency[i].entry(j).or_insert(0.0) += weight;
            *adjacency[j].entry(i).or_insert(0.0) += weight;
        }
    }
    let partition = louvain(&adjacency, resolution);
    let score = modularity(&adjacency, &partition, resolution);
    let mut groups = BTreeMap::<usize, Vec<String>>::new();
    for (i, group) in partition.into_iter().enumerate() {
        groups.entry(group).or_default().push(selected[i].clone());
    }
    for (number, members) in groups {
        let member_key = members.join(":");
        let id = stable_id(&[
            &graph.repository,
            "louvain",
            &resolution.to_string(),
            &member_key,
        ]);
        let sources = members
            .iter()
            .take(64)
            .filter_map(|member| graph.nodes[member].source.clone())
            .collect();
        let evidence = Evidence::inferred(
            format!(
                "deterministic multilevel Louvain; resolution={resolution}; modularity={score:.6}; full recluster; first 64 source spans sampled, complete membership on edges"
            ),
            sources,
        );
        graph.add_node(derived_node(
            graph,
            id.clone(),
            format!("Unnamed cluster {}", number + 1),
            (NodeKind::Cluster, 2),
            format!(
                "{} structurally related nodes; no business label inferred",
                members.len()
            ),
            evidence.clone(),
            vec![
                "glyphibe:derived-structure".into(),
                format!("resolution:{resolution}"),
                format!("modularity:{score:.6}"),
            ],
        ));
        graph.add_edge(&domain_id, &id, Relation::Contains, evidence.clone());
        for member in members {
            let source = graph.nodes[&member].source.clone().into_iter().collect();
            graph.add_edge(
                &id,
                member,
                Relation::Contains,
                Evidence::inferred("Louvain community membership", source),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn graph(n: usize, edges: &[(usize, usize)]) -> Adjacency {
        let mut a = vec![BTreeMap::new(); n];
        for (i, j) in edges {
            *a[*i].entry(*j).or_insert(0.0) += 1.0;
            *a[*j].entry(*i).or_insert(0.0) += 1.0;
        }
        a
    }
    #[test]
    fn separates_cliques_and_retains_isolates() {
        let a = graph(7, &[(0, 1), (1, 2), (2, 0), (3, 4), (4, 5), (5, 3), (2, 3)]);
        let p = louvain(&a, 1.0);
        assert_eq!(p[0], p[1]);
        assert_eq!(p[1], p[2]);
        assert_eq!(p[3], p[4]);
        assert_ne!(p[0], p[3]);
        assert_ne!(p[0], p[6]);
        assert!(modularity(&a, &p, 1.0) > modularity(&a, &(0..7).collect::<Vec<_>>(), 1.0));
        assert_eq!(p, louvain(&a, 1.0));
    }
    #[test]
    fn resolution_changes_partition() {
        let a = graph(3, &[(0, 1), (1, 2), (2, 0)]);
        assert_eq!(louvain(&a, 0.2), vec![0, 0, 0]);
        let p = louvain(&a, 4.0);
        assert_ne!(p[0], p[1]);
        assert_ne!(p[1], p[2]);
    }
    #[test]
    fn empty_and_many_isolates_are_valid() {
        assert!(louvain(&vec![], 1.0).is_empty());
        let p = louvain(&vec![BTreeMap::new(); 10_000], 1.0);
        assert_eq!(p.len(), 10_000);
        assert_eq!(p[9999], 9999);
    }
    #[test]
    fn enrichment_is_repeatable_and_does_not_invent_business_labels() {
        let mut g = Graph::new("fixture", "unversioned");
        enrich(&mut g, 1.0);
        let first = g.nodes.clone();
        enrich(&mut g, 1.0);
        assert_eq!(first, g.nodes);
        g.validate().unwrap();
        assert!(g.nodes.values().all(|n| !n.label.contains("payment")));
    }
    #[test]
    fn iterative_cycles_handle_long_paths_recursion_and_stale_edges() {
        let mut g = Graph::new("fixture", "unversioned");
        for i in 0..10_000 {
            g.add_node(derived_node(
                &g,
                i.to_string(),
                i.to_string(),
                (NodeKind::Function, 5),
                String::new(),
                Evidence::observed("fixture", vec![]),
                vec![],
            ));
        }
        for i in 0..9999 {
            g.add_edge(
                i.to_string(),
                (i + 1).to_string(),
                Relation::Calls,
                Evidence::inferred("fixture", vec![]),
            );
        }
        assert!(cycles(&g).is_empty());
        g.add_edge(
            "9999",
            "9998",
            Relation::Calls,
            Evidence::inferred("fixture", vec![]),
        );
        g.add_edge(
            "0",
            "0",
            Relation::Calls,
            Evidence::inferred("fixture", vec![]),
        );
        let c = cycles(&g);
        assert_eq!(c.len(), 2);
        assert!(c.iter().any(|c| c == &vec!["0".to_owned()]));
        g.edges.last_mut().unwrap().stale = true;
        assert_eq!(cycles(&g).len(), 1);
    }
}
