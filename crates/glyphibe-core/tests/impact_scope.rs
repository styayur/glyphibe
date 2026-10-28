// SPDX-License-Identifier: MPL-2.0
//! Scope fixtures state exact evidence-bearing relationships; no project code runs.
use glyphibe_core::*;
use std::collections::BTreeSet;

fn node(id: &str, kind: NodeKind, level: u8) -> Node {
    Node {
        id: id.into(),
        label: id.into(),
        kind,
        level,
        plane: Plane::Architecture,
        repository: "impact-fixture".into(),
        revision: "working".into(),
        language: Some("python".into()),
        source: Some(SourceSpan {
            path: format!("{id}.py"),
            start_line: 1,
            start_column: 1,
            end_line: 4,
            end_column: 1,
            start_byte: 0,
            end_byte: 80,
        }),
        summary: "fixture declaration".into(),
        evidence: Evidence::observed("fixture syntax", vec![]),
        fingerprint: String::new(),
        tags: vec![],
        knowledge: vec![],
        change: ChangeStatus::Unchanged,
    }
}
fn edge(graph: &mut Graph, from: &str, to: &str, relation: Relation) {
    graph.add_edge(
        from,
        to,
        relation,
        Evidence::observed("fixture relationship", vec![]),
    );
}
fn base() -> Graph {
    let mut graph = Graph::new("impact-fixture", "working");
    for (id, kind, level) in [
        ("domain", NodeKind::Domain, 1),
        ("cluster", NodeKind::Cluster, 2),
        ("component", NodeKind::Component, 3),
        ("external", NodeKind::Component, 3),
        ("file", NodeKind::File, 4),
        ("other-file", NodeKind::File, 4),
        ("function", NodeKind::Function, 5),
        ("caller", NodeKind::Function, 5),
        ("test", NodeKind::Function, 5),
        ("statement", NodeKind::Statement, 6),
    ] {
        graph.add_node(node(id, kind, level));
    }
    graph
        .nodes
        .get_mut("test")
        .unwrap()
        .tags
        .push("test".into());
    graph
        .nodes
        .get_mut("caller")
        .unwrap()
        .tags
        .push("exported".into());
    for (from, to) in [
        ("domain", "cluster"),
        ("cluster", "component"),
        ("component", "file"),
        ("file", "function"),
        ("function", "statement"),
        ("external", "other-file"),
        ("other-file", "caller"),
        ("other-file", "test"),
    ] {
        edge(&mut graph, from, to, Relation::Contains);
    }
    edge(&mut graph, "caller", "function", Relation::Calls);
    edge(&mut graph, "test", "caller", Relation::Calls);
    graph
}
fn set(values: &[String]) -> BTreeSet<&str> {
    values.iter().map(String::as_str).collect()
}

#[test]
fn aggregates_find_cross_file_callers_and_tests_without_spending_hops_on_containment() {
    let graph = base();
    for origin in ["domain", "cluster", "component", "file"] {
        let direct = graph.impact(origin, Direction::Upstream, 1).unwrap();
        assert!(
            direct.reachability.nodes.iter().any(|id| id == "caller"),
            "{origin}"
        );
        assert!(
            !direct.reachability.nodes.iter().any(|id| id == "test"),
            "{origin}"
        );
        assert_eq!(direct.reachability.paths["caller"], ["function", "caller"]);
        let transitive = graph.impact(origin, Direction::Upstream, 2).unwrap();
        assert!(transitive.reachability.nodes.iter().any(|id| id == "test"));
        assert_eq!(
            transitive.reachability.paths["test"],
            ["function", "caller", "test"]
        );
        assert_eq!(transitive.tests, 1);
        assert_eq!(transitive.interfaces, 1);
        assert_eq!(transitive.components, 2);
        assert_eq!(transitive.functions, 3);
        assert!(
            transitive
                .scope
                .basis
                .iter()
                .any(|basis| basis.step == "descendant_scope")
        );
    }
}

#[test]
fn statements_and_deep_blocks_reach_the_nearest_function_without_a_six_level_depth_limit() {
    let mut graph = base();
    graph
        .edges
        .retain(|edge| !(edge.source == "function" && edge.target == "statement"));
    graph.reindex_edges();
    let mut parent = "function".to_owned();
    for index in 0..24 {
        let id = format!("block-{index}");
        graph.add_node(node(&id, NodeKind::Block, 5));
        edge(&mut graph, &parent, &id, Relation::Contains);
        parent = id;
    }
    edge(&mut graph, &parent, "statement", Relation::Contains);
    for origin in ["statement", "block-23"] {
        let impact = graph.impact(origin, Direction::Upstream, 1).unwrap();
        assert_eq!(
            set(&impact.scope.dependency_seeds),
            BTreeSet::from(["function"])
        );
        assert!(set(&impact.reachability.nodes).contains("caller"));
        assert_eq!(impact.reachability.paths["caller"], ["function", "caller"]);
        assert_eq!(impact.components, 2);
        assert!(
            impact
                .scope
                .basis
                .iter()
                .filter(|basis| basis.step == "owner_scope")
                .count()
                > 6
        );
    }
    assert!(
        graph
            .reachable("statement", Direction::Upstream, 3, &[Relation::Calls], 100)
            .unwrap()
            .nodes
            .is_empty()
    );
}

#[test]
fn function_impact_keeps_existing_call_paths_and_does_not_expand_its_whole_file() {
    let graph = base();
    let impact = graph.impact("function", Direction::Upstream, 2).unwrap();
    let trace = graph
        .reachable(
            "function",
            Direction::Upstream,
            2,
            &[Relation::Calls],
            50_000,
        )
        .unwrap();
    assert_eq!(impact.reachability.nodes, trace.nodes);
    assert_eq!(impact.reachability.paths, trace.paths);
    assert_eq!(impact.functions, 2);
    assert_eq!(
        set(&impact.scope.dependency_seeds),
        BTreeSet::from(["function"])
    );
    assert!(
        !impact
            .scope
            .basis
            .iter()
            .any(|basis| basis.step == "owner_scope" || basis.step == "descendant_scope")
    );
}

#[test]
fn many_parent_ownership_and_containment_cycles_are_finite_and_keep_all_branch_owners() {
    let mut graph = base();
    graph.add_node(node("other-function", NodeKind::Function, 5));
    graph.add_node(node("loop-a", NodeKind::Block, 5));
    graph.add_node(node("loop-b", NodeKind::Block, 5));
    for (from, to) in [
        ("function", "loop-a"),
        ("loop-a", "loop-b"),
        ("loop-b", "loop-a"),
        ("loop-b", "statement"),
        ("other-function", "statement"),
        ("other-file", "statement"),
    ] {
        edge(&mut graph, from, to, Relation::Contains);
    }
    let impact = graph.impact("statement", Direction::Upstream, 1).unwrap();
    assert_eq!(
        set(&impact.scope.dependency_seeds),
        BTreeSet::from(["function", "other-function", "other-file"])
    );
    assert!(!impact.scope.truncated);
    assert!(impact.scope.structural_nodes.len() <= graph.nodes.len());
    assert!(set(&impact.reachability.nodes).contains("caller"));
}

#[test]
fn explanation_mapping_prefers_exact_source_and_level_over_broad_member_coverage() {
    let mut graph = base();
    let mut explanation = graph.nodes["function"].clone();
    explanation.id = "explanation".into();
    explanation.kind = NodeKind::Explanation;
    explanation.plane = Plane::Explanation;
    graph.add_node(explanation);
    let mut block = node("same-level-block", NodeKind::Block, 5);
    block.source = graph.nodes["statement"].source.clone();
    graph.add_node(block);
    let function_source = graph.nodes["function"].source.clone();
    let exact_file = graph.nodes.get_mut("file").unwrap();
    exact_file.source = function_source;
    for origin in ["function", "statement", "file", "same-level-block"] {
        edge(&mut graph, origin, "explanation", Relation::MapsTo);
    }
    let impact = graph.impact("explanation", Direction::Upstream, 2).unwrap();
    assert_eq!(impact.scope.architecture_origins, ["function"]);
    assert_eq!(impact.scope.dependency_seeds, ["function"]);
    assert_eq!(
        impact.reachability.nodes,
        graph
            .impact("function", Direction::Upstream, 2)
            .unwrap()
            .reachability
            .nodes
    );
    assert_eq!(
        impact
            .scope
            .basis
            .iter()
            .filter(|basis| basis.step == "explanation_mapping")
            .count(),
        1
    );
}

#[test]
fn equally_specific_explanation_mappings_remain_many_to_many() {
    let mut graph = base();
    let mut alias = graph.nodes["function"].clone();
    alias.id = "alias".into();
    graph.add_node(alias);
    let mut explanation = graph.nodes["function"].clone();
    explanation.id = "explanation".into();
    explanation.kind = NodeKind::Explanation;
    explanation.plane = Plane::Explanation;
    graph.add_node(explanation);
    edge(&mut graph, "function", "explanation", Relation::MapsTo);
    edge(&mut graph, "alias", "explanation", Relation::MapsTo);
    let impact = graph.impact("explanation", Direction::Upstream, 1).unwrap();
    assert_eq!(
        set(&impact.scope.architecture_origins),
        BTreeSet::from(["alias", "function"])
    );
    assert_eq!(impact.scope.dependency_seeds.len(), 2);
}

#[test]
fn stale_removed_and_proposed_nodes_edges_and_mapping_candidates_never_expand_scope() {
    let mut graph = base();
    for id in ["stale", "removed", "proposed", "proposed-edge"] {
        graph.add_node(node(id, NodeKind::Function, 5));
    }
    graph.nodes.get_mut("removed").unwrap().change = ChangeStatus::Removed;
    graph.nodes.get_mut("proposed").unwrap().evidence.kind = EvidenceKind::Proposed;
    for id in ["stale", "removed", "proposed", "proposed-edge"] {
        edge(&mut graph, "component", id, Relation::Contains);
    }
    graph
        .edges
        .iter_mut()
        .find(|edge| edge.target == "stale")
        .unwrap()
        .stale = true;
    graph
        .edges
        .iter_mut()
        .find(|edge| edge.target == "proposed-edge")
        .unwrap()
        .evidence
        .kind = EvidenceKind::Proposed;
    let mut evidence = Evidence::inferred("unverified agent claim", vec![]);
    evidence.kind = EvidenceKind::Proposed;
    graph.add_edge("function", "proposed-edge", Relation::Calls, evidence);
    let impact = graph.impact("component", Direction::Downstream, 3).unwrap();
    for id in ["stale", "removed", "proposed", "proposed-edge"] {
        assert!(!set(&impact.scope.structural_nodes).contains(id));
        assert!(!set(&impact.reachability.nodes).contains(id));
    }
    assert!(
        impact
            .scope
            .basis
            .iter()
            .all(|basis| basis.evidence != EvidenceKind::Proposed)
    );
    assert!(graph.impact("removed", Direction::Both, 1).is_err());
    assert!(graph.impact("proposed", Direction::Both, 1).is_err());
}

#[test]
fn absent_mapping_is_explicitly_unresolved_and_bad_controls_are_rejected() {
    let mut graph = base();
    let mut explanation = node("unmapped", NodeKind::Explanation, 5);
    explanation.plane = Plane::Explanation;
    graph.add_node(explanation);
    let impact = graph.impact("unmapped", Direction::Both, 2).unwrap();
    assert!(impact.scope.architecture_origins.is_empty());
    assert!(impact.reachability.nodes.is_empty());
    assert!(impact.limitation.contains("No active evidenced"));
    assert!(graph.impact("function", Direction::Both, 33).is_err());
    assert!(graph.impact("missing", Direction::Both, 1).is_err());
}

#[test]
fn structural_and_dependency_results_disclose_the_fifty_thousand_node_cap() {
    let mut graph = Graph::new("impact-fixture", "working");
    graph.add_node(node("component", NodeKind::Component, 3));
    for index in 0..50_005 {
        let id = format!("fn-{index:05}");
        graph.add_node(node(&id, NodeKind::Function, 5));
        edge(&mut graph, "component", &id, Relation::Contains);
    }
    let impact = graph.impact("component", Direction::Upstream, 1).unwrap();
    assert!(impact.scope.truncated && impact.reachability.truncated);
    assert!(impact.scope.structural_nodes.len() <= 50_000);
    assert!(impact.scope.basis.len() <= 50_000);
    assert!(impact.reachability.paths.len() <= 50_000);
    assert!(impact.functions < 50_005);
}
