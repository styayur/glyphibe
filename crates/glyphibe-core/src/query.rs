// SPDX-License-Identifier: MPL-2.0
use crate::*;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Upstream,
    Downstream,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reachability {
    pub origin: String,
    pub direction: Direction,
    pub hops: u8,
    pub nodes: Vec<String>,
    pub paths: BTreeMap<String, Vec<String>>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Impact {
    pub reachability: Reachability,
    pub scope: ImpactScope,
    pub components: usize,
    pub functions: usize,
    pub tests: usize,
    pub interfaces: usize,
    pub limitation: String,
}

/// Structural scope is disclosed separately from dependency paths. These steps
/// never consume dependency hops and never fabricate call/dependency edges.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactScope {
    pub requested: String,
    pub architecture_origins: Vec<String>,
    pub dependency_seeds: Vec<String>,
    pub structural_nodes: Vec<String>,
    pub basis: Vec<ImpactScopeBasis>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactScopeBasis {
    pub edge_id: String,
    pub source: String,
    pub target: String,
    pub relation: Relation,
    pub evidence: EvidenceKind,
    pub method: String,
    pub step: String,
}

const IMPACT_LIMIT: usize = 50_000;

struct ImpactResolver<'a> {
    graph: &'a Graph,
    children: BTreeMap<&'a str, Vec<&'a Edge>>,
    parents: BTreeMap<&'a str, Vec<&'a Edge>>,
    seen: BTreeSet<String>,
    structural: BTreeSet<String>,
    origins: BTreeSet<String>,
    seeds: BTreeSet<String>,
    basis: BTreeMap<(String, String), ImpactScopeBasis>,
    truncated: bool,
}

impl<'a> ImpactResolver<'a> {
    fn new(graph: &'a Graph) -> Self {
        let mut resolver = Self {
            graph,
            children: BTreeMap::new(),
            parents: BTreeMap::new(),
            seen: BTreeSet::new(),
            structural: BTreeSet::new(),
            origins: BTreeSet::new(),
            seeds: BTreeSet::new(),
            basis: BTreeMap::new(),
            truncated: false,
        };
        for edge in &graph.edges {
            if edge.relation == Relation::Contains
                && resolver.active_edge(edge)
                && graph.nodes[&edge.source].plane == Plane::Architecture
                && graph.nodes[&edge.target].plane == Plane::Architecture
            {
                resolver
                    .children
                    .entry(&edge.source)
                    .or_default()
                    .push(edge);
                resolver.parents.entry(&edge.target).or_default().push(edge);
            }
        }
        for edges in resolver.children.values_mut() {
            edges.sort_unstable_by(|a, b| (&a.target, &a.id).cmp(&(&b.target, &b.id)));
        }
        for edges in resolver.parents.values_mut() {
            edges.sort_unstable_by(|a, b| (&a.source, &a.id).cmp(&(&b.source, &b.id)));
        }
        resolver
    }

    fn active_node(&self, id: &str) -> bool {
        self.graph.nodes.get(id).is_some_and(|node| {
            node.change != ChangeStatus::Removed && node.evidence.kind != EvidenceKind::Proposed
        })
    }

    fn active_edge(&self, edge: &Edge) -> bool {
        !edge.stale
            && edge.evidence.kind != EvidenceKind::Proposed
            && self.active_node(&edge.source)
            && self.active_node(&edge.target)
    }

    fn admit(&mut self, id: &str, structural: bool) -> bool {
        if !self.active_node(id) {
            return false;
        }
        if !self.seen.contains(id) {
            if self.seen.len() >= IMPACT_LIMIT {
                self.truncated = true;
                return false;
            }
            self.seen.insert(id.to_owned());
        }
        if structural {
            self.structural.insert(id.to_owned());
        }
        true
    }

    fn record(&mut self, edge: &Edge, step: &str) {
        let key = (step.to_owned(), edge.id.clone());
        if self.basis.contains_key(&key) {
            return;
        }
        if self.basis.len() >= IMPACT_LIMIT {
            self.truncated = true;
            return;
        }
        self.basis.insert(
            key,
            ImpactScopeBasis {
                edge_id: edge.id.clone(),
                source: edge.source.clone(),
                target: edge.target.clone(),
                relation: edge.relation,
                evidence: edge.evidence.kind,
                method: edge.evidence.method.clone(),
                step: step.to_owned(),
            },
        );
    }

    fn resolve_origins(&mut self, requested: &str) {
        self.admit(requested, true);
        let selection = &self.graph.nodes[requested];
        if selection.plane == Plane::Architecture {
            self.origins.insert(requested.to_owned());
            return;
        }
        let candidates = self.graph.edges.iter().filter_map(|edge| {
            if edge.relation != Relation::MapsTo || !self.active_edge(edge) {
                return None;
            }
            let other = if edge.source == requested {
                &edge.target
            } else if edge.target == requested {
                &edge.source
            } else {
                return None;
            };
            let node = &self.graph.nodes[other];
            if node.plane != Plane::Architecture {
                return None;
            }
            let exact = selection
                .source
                .as_ref()
                .is_some_and(|span| node.source.as_ref() == Some(span));
            let score = u8::from(exact) * 2 + u8::from(node.level == selection.level);
            Some((score, other.clone(), edge))
        });
        // Score first without collecting an unbounded set of mapping candidates.
        let best = candidates.clone().map(|(score, _, _)| score).max();
        let mut selected = BTreeMap::new();
        let mut mappings_truncated = false;
        for (_, target, edge) in candidates.filter(|(score, _, _)| Some(*score) == best) {
            selected.insert((target, edge.id.clone()), edge);
            if selected.len() > IMPACT_LIMIT {
                selected.pop_last();
                mappings_truncated = true;
            }
        }
        self.truncated |= mappings_truncated;
        for ((target, _), edge) in selected {
            if self.admit(&target, true) {
                self.origins.insert(target);
                self.record(edge, "explanation_mapping");
            }
        }
    }

    fn resolve_code_scope(&mut self) {
        let mut descendants = VecDeque::new();
        let mut owners = VecDeque::new();
        for id in &self.origins {
            match self.graph.nodes[id].kind {
                NodeKind::Domain
                | NodeKind::Cluster
                | NodeKind::Component
                | NodeKind::File
                | NodeKind::Class => descendants.push_back(id.clone()),
                NodeKind::Function => {
                    self.seeds.insert(id.clone());
                }
                _ => owners.push_back(id.clone()),
            }
        }
        let mut scheduled: BTreeSet<_> = descendants.iter().cloned().collect();
        while let Some(id) = descendants.pop_front() {
            match self.graph.nodes[&id].kind {
                NodeKind::Domain
                | NodeKind::Cluster
                | NodeKind::Component
                | NodeKind::File
                | NodeKind::Class
                | NodeKind::Function => {
                    self.seeds.insert(id.clone());
                }
                _ => {}
            }
            let edges = self.children.get(id.as_str()).cloned().unwrap_or_default();
            for edge in edges {
                if self.admit(&edge.target, true) {
                    self.record(edge, "descendant_scope");
                    if scheduled.insert(edge.target.clone()) {
                        descendants.push_back(edge.target.clone());
                    }
                }
            }
        }
        // A shared multi-source walk visits each owner region once. Stop each
        // branch at its nearest function or file, retaining all valid parents.
        let mut scheduled: BTreeSet<_> = owners.iter().cloned().collect();
        while let Some(id) = owners.pop_front() {
            if matches!(
                self.graph.nodes[&id].kind,
                NodeKind::Function | NodeKind::File
            ) {
                self.seeds.insert(id);
                continue;
            }
            let edges = self.parents.get(id.as_str()).cloned().unwrap_or_default();
            for edge in edges {
                if self.admit(&edge.source, true) {
                    self.record(edge, "owner_scope");
                    if scheduled.insert(edge.source.clone()) {
                        owners.push_back(edge.source.clone());
                    }
                }
            }
        }
    }

    fn dependency_paths(
        &mut self,
        requested: &str,
        direction: Direction,
        hops: u8,
    ) -> Reachability {
        let mut adjacency: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for edge in &self.graph.edges {
            if !edge.relation.is_dependency()
                || !self.active_edge(edge)
                || self.graph.nodes[&edge.source].plane != Plane::Architecture
                || self.graph.nodes[&edge.target].plane != Plane::Architecture
            {
                continue;
            }
            if matches!(direction, Direction::Downstream | Direction::Both) {
                adjacency
                    .entry(&edge.source)
                    .or_default()
                    .push(&edge.target);
            }
            if matches!(direction, Direction::Upstream | Direction::Both) {
                adjacency
                    .entry(&edge.target)
                    .or_default()
                    .push(&edge.source);
            }
        }
        for values in adjacency.values_mut() {
            values.sort_unstable();
            values.dedup();
        }
        let mut paths: BTreeMap<_, _> = self
            .seeds
            .iter()
            .map(|id| (id.clone(), vec![id.clone()]))
            .collect();
        let mut queue: VecDeque<_> = self.seeds.iter().map(|id| (id.clone(), 0u8)).collect();
        while let Some((id, depth)) = queue.pop_front() {
            if depth >= hops {
                continue;
            }
            if let Some(neighbors) = adjacency.get(id.as_str()) {
                for next in neighbors {
                    if paths.contains_key(*next) {
                        continue;
                    }
                    if self.admit(next, false) {
                        let mut path = paths[&id].clone();
                        path.push((*next).to_owned());
                        paths.insert((*next).to_owned(), path);
                        queue.push_back(((*next).to_owned(), depth + 1));
                    }
                }
            }
        }
        Reachability {
            origin: requested.into(),
            direction,
            hops,
            nodes: paths
                .keys()
                .filter(|id| id.as_str() != requested && !self.origins.contains(*id))
                .cloned()
                .collect(),
            paths,
            truncated: self.truncated,
        }
    }

    fn component_ancestors(&mut self, paths: &BTreeMap<String, Vec<String>>) -> BTreeSet<String> {
        let mut ancestors: BTreeSet<_> = paths
            .keys()
            .cloned()
            .chain(self.origins.iter().cloned())
            .collect();
        let mut queue: VecDeque<_> = ancestors.iter().cloned().collect();
        while let Some(id) = queue.pop_front() {
            let edges = self.parents.get(id.as_str()).cloned().unwrap_or_default();
            for edge in edges {
                if self.admit(&edge.source, true) {
                    self.record(edge, "component_ancestor");
                    if ancestors.insert(edge.source.clone()) {
                        queue.push_back(edge.source.clone());
                    }
                }
            }
        }
        ancestors
    }
}

impl Graph {
    pub fn search(&self, query: &str, limit: usize) -> Vec<&Node> {
        let query = query.to_lowercase();
        self.nodes
            .values()
            .filter(|n| {
                n.change != ChangeStatus::Removed
                    && (n.label.to_lowercase().contains(&query)
                        || n.tags.iter().any(|t| t.to_lowercase().contains(&query))
                        || n.source
                            .as_ref()
                            .is_some_and(|s| s.path.to_lowercase().contains(&query)))
            })
            .take(limit.min(10_000))
            .collect()
    }

    pub fn reachable(
        &self,
        origin: &str,
        direction: Direction,
        hops: u8,
        relations: &[Relation],
        limit: usize,
    ) -> Result<Reachability> {
        if !self.nodes.contains_key(origin) {
            bail!("unknown node");
        }
        if self.nodes[origin].change == ChangeStatus::Removed {
            bail!("node is historical and removed; select a current node");
        }
        if hops > 32 || limit == 0 || limit > 100_000 {
            bail!("hops must be 0..32 and node limit 1..100000");
        }
        let mut adjacency: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for edge in &self.edges {
            if edge.stale
                || !relations.contains(&edge.relation)
                || edge.evidence.kind == EvidenceKind::Proposed
            {
                continue;
            }
            if self
                .nodes
                .get(&edge.source)
                .is_some_and(|n| n.change == ChangeStatus::Removed)
                || self
                    .nodes
                    .get(&edge.target)
                    .is_some_and(|n| n.change == ChangeStatus::Removed)
            {
                continue;
            }
            if matches!(direction, Direction::Downstream | Direction::Both) {
                adjacency
                    .entry(&edge.source)
                    .or_default()
                    .push(&edge.target);
            }
            if matches!(direction, Direction::Upstream | Direction::Both) {
                adjacency
                    .entry(&edge.target)
                    .or_default()
                    .push(&edge.source);
            }
        }
        for targets in adjacency.values_mut() {
            targets.sort_unstable();
            targets.dedup();
        }
        let mut paths = BTreeMap::from([(origin.to_string(), vec![origin.to_string()])]);
        let mut queue = VecDeque::from([(origin.to_string(), 0u8)]);
        let mut truncated = false;
        while let Some((id, depth)) = queue.pop_front() {
            if depth >= hops {
                continue;
            }
            if let Some(neighbors) = adjacency.get(id.as_str()) {
                for next in neighbors {
                    if paths.contains_key(*next) {
                        continue;
                    }
                    if paths.len() >= limit {
                        truncated = true;
                        continue;
                    }
                    let mut path = paths[&id].clone();
                    path.push((*next).to_string());
                    paths.insert((*next).to_string(), path);
                    queue.push_back(((*next).to_string(), depth + 1));
                }
            }
        }
        let nodes = paths
            .keys()
            .filter(|id| id.as_str() != origin)
            .cloned()
            .collect();
        Ok(Reachability {
            origin: origin.into(),
            direction,
            hops,
            nodes,
            paths,
            truncated,
        })
    }
    pub fn impact(&self, origin: &str, direction: Direction, hops: u8) -> Result<Impact> {
        let Some(selection) = self.nodes.get(origin) else {
            bail!("unknown node");
        };
        if selection.change == ChangeStatus::Removed {
            bail!("node is historical and removed; select a current node");
        }
        if selection.evidence.kind == EvidenceKind::Proposed {
            bail!("proposed nodes do not establish a static impact scope");
        }
        if hops > 32 {
            bail!("hops must be 0..32");
        }
        let mut resolver = ImpactResolver::new(self);
        resolver.resolve_origins(origin);
        resolver.resolve_code_scope();
        let mut reachability = resolver.dependency_paths(origin, direction, hops);
        let nodes: Vec<_> = reachability
            .nodes
            .iter()
            .filter_map(|id| self.nodes.get(id))
            .collect();
        let contained = resolver.component_ancestors(&reachability.paths);
        reachability.truncated |= resolver.truncated;
        let unresolved = resolver.seeds.is_empty();
        let scope = ImpactScope {
            requested: origin.into(),
            architecture_origins: resolver.origins.into_iter().collect(),
            dependency_seeds: resolver.seeds.into_iter().collect(),
            structural_nodes: resolver.structural.into_iter().collect(),
            basis: resolver.basis.into_values().collect(),
            truncated: resolver.truncated,
        };
        Ok(Impact {
            components: contained
                .iter()
                .filter_map(|id| self.nodes.get(id))
                .filter(|n| n.kind == NodeKind::Component)
                .count(),
            functions: nodes
                .iter()
                .filter(|n| n.kind == NodeKind::Function)
                .count(),
            tests: nodes
                .iter()
                .filter(|n| n.tags.iter().any(|t| t == "test"))
                .count(),
            interfaces: nodes
                .iter()
                .filter(|n| {
                    n.tags
                        .iter()
                        .any(|t| t == "interface" || t == "export" || t == "exported")
                })
                .count(),
            reachability,
            scope,
            limitation: if unresolved {
                "No active evidenced architecture/owning-code scope was resolved. An empty result does not establish absence of impact.".into()
            } else {
                "Structural scope expansion is disclosed separately from dependency paths. Paths begin at dependency_seeds; only dependency edges consume hops. Static reachability is not failure probability or guaranteed runtime impact. Unresolved dynamic calls are absent. Nodes and structural evidence are bounded at 50000; inspect truncated flags.".into()
            },
        })
    }
    /// Structural ancestors or descendants are graph queries, never a rigid level tree.
    pub fn hierarchy(&self, origin: &str, upward: bool, hops: u8) -> Result<Reachability> {
        self.reachable(
            origin,
            if upward {
                Direction::Upstream
            } else {
                Direction::Downstream
            },
            hops,
            &[Relation::Contains],
            50_000,
        )
    }
    pub fn mappings(&self, origin: &str) -> Result<Reachability> {
        self.reachable(
            origin,
            Direction::Both,
            1,
            &[Relation::MapsTo, Relation::HasConcept],
            50_000,
        )
    }
}
