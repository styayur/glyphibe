// SPDX-License-Identifier: MPL-2.0
use glyphibe_core::*;
fn tempdir() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
}

fn node(id: &str, kind: NodeKind, level: u8) -> Node {
    Node {
        id: id.into(),
        label: id.into(),
        kind,
        level,
        plane: Plane::Architecture,
        repository: "fixture".into(),
        revision: "one".into(),
        language: Some("python".into()),
        source: None,
        summary: String::new(),
        evidence: Evidence::observed("fixture", vec![]),
        fingerprint: fingerprint(id),
        tags: vec![],
        knowledge: vec![],
        change: ChangeStatus::Unchanged,
    }
}
fn graph() -> Graph {
    let mut g = Graph::new("fixture", "one");
    for id in ["a", "b", "c", "isolated"] {
        g.add_node(node(id, NodeKind::Function, 5));
    }
    for (a, b) in [("a", "b"), ("b", "c"), ("c", "a")] {
        g.add_edge(a, b, Relation::Calls, Evidence::inferred("lexical", vec![]));
    }
    g
}

#[test]
fn directional_bounded_paths_and_cycles() {
    let g = graph();
    let downstream = g
        .reachable("a", Direction::Downstream, 2, &[Relation::Calls], 100)
        .unwrap();
    assert_eq!(downstream.paths["c"], ["a", "b", "c"]);
    assert!(!downstream.nodes.iter().any(|n| n == "isolated"));
    let upstream = g
        .reachable("a", Direction::Upstream, 1, &[Relation::Calls], 100)
        .unwrap();
    assert_eq!(upstream.nodes, ["c"]);
    assert!(
        g.reachable("a", Direction::Both, 33, &[Relation::Calls], 100)
            .is_err()
    );
    assert!(
        g.reachable("unknown", Direction::Both, 1, &[Relation::Calls], 100)
            .is_err()
    );
    assert!(
        g.reachable("a", Direction::Both, 4, &[Relation::Calls], 2)
            .unwrap()
            .truncated
    );
}
#[test]
fn stale_and_proposed_edges_do_not_become_impacts() {
    let mut g = graph();
    g.edges[0].stale = true;
    assert!(
        g.impact("a", Direction::Downstream, 3)
            .unwrap()
            .reachability
            .nodes
            .is_empty()
    );
    g.edges[0].stale = false;
    g.edges[0].evidence.kind = EvidenceKind::Proposed;
    assert!(
        g.impact("a", Direction::Downstream, 3)
            .unwrap()
            .reachability
            .nodes
            .is_empty()
    );
}
#[test]
fn many_to_many_cross_plane_and_non_tree_hierarchy() {
    let mut g = graph();
    let mut explanation = node("e", NodeKind::Explanation, 5);
    explanation.plane = Plane::Explanation;
    g.add_node(explanation);
    for id in ["a", "b"] {
        g.add_edge(
            id,
            "e",
            Relation::MapsTo,
            Evidence::inferred("template", vec![]),
        );
    }
    g.add_edge(
        "a",
        "b",
        Relation::Contains,
        Evidence::observed("scope", vec![]),
    );
    g.add_edge(
        "c",
        "b",
        Relation::Contains,
        Evidence::observed("scope", vec![]),
    );
    g.validate().unwrap();
    assert_eq!(g.mappings("e").unwrap().nodes, ["a", "b"]);
    assert_eq!(g.hierarchy("b", true, 1).unwrap().nodes, ["a", "c"]);
    g.add_edge(
        "a",
        "b",
        Relation::MapsTo,
        Evidence::inferred("bad", vec![]),
    );
    assert!(g.validate().is_err());
}
#[test]
fn identity_reconciles_unique_move_and_marks_stale_relations() {
    let old = graph();
    let mut new = Graph::new("fixture", "two");
    let mut moved = node("renamed", NodeKind::Function, 5);
    moved.fingerprint = fingerprint("a");
    new.add_node(moved);
    new.reconcile(&old).unwrap();
    assert_eq!(new.nodes["a"].label, "renamed");
    assert_eq!(new.nodes["a"].change, ChangeStatus::Modified);
    assert_eq!(new.nodes["b"].change, ChangeStatus::Removed);
    assert!(new.edges.iter().all(|e| e.stale));
}
#[test]
fn ambiguous_fingerprints_never_guess_identity() {
    let mut old = Graph::new("fixture", "one");
    for id in ["a", "b"] {
        let mut n = node(id, NodeKind::Function, 5);
        n.fingerprint = "same".into();
        old.add_node(n);
    }
    let mut new = Graph::new("fixture", "two");
    let mut n = node("c", NodeKind::Function, 5);
    n.fingerprint = "same".into();
    new.add_node(n);
    new.reconcile(&old).unwrap();
    assert_eq!(new.nodes["c"].change, ChangeStatus::Ambiguous);
    assert!(
        new.diagnostics
            .iter()
            .any(|d| d.code == "identity_ambiguous")
    );
}
#[test]
fn delimiter_input_and_dangling_edges_are_rejected() {
    assert_ne!(stable_id(&["a", "bc"]), stable_id(&["ab", "c"]));
    let mut g = graph();
    g.add_edge(
        "a",
        "missing",
        Relation::Calls,
        Evidence::observed("bad", vec![]),
    );
    assert!(g.validate().is_err());
}
#[test]
fn sqlite_roundtrip_invalidation_proposal_isolation_and_binding() {
    let dir = tempdir();
    let store = IndexStore::open(&dir.path().join("cache.sqlite")).unwrap();
    store.bind_repository(dir.path()).unwrap();
    let other = tempdir();
    assert!(store.bind_repository(other.path()).is_err());
    let g = graph();
    store.save_graph(&g).unwrap();
    assert_eq!(store.load_graph().unwrap().unwrap().nodes.len(), 4);
    store.save_file("a.py", "v1", "{}").unwrap();
    assert_eq!(store.load_file("a.py", "v1").unwrap(), Some("{}".into()));
    assert_eq!(store.load_file("a.py", "v2").unwrap(), None);
    assert!(
        store
            .propose(&g, "a", "b", Relation::Calls, "label", "reason")
            .is_err()
    );
    let p = store
        .propose(
            &g,
            "a",
            "b",
            Relation::DependsOn,
            "component candidate",
            "user evidence",
        )
        .unwrap();
    assert_eq!(p.evidence, EvidenceKind::Proposed);
    assert_eq!(store.proposals().unwrap().len(), 1);
    assert!(
        store
            .load_graph()
            .unwrap()
            .unwrap()
            .edges
            .iter()
            .all(|e| e.evidence.kind != EvidenceKind::Proposed)
    );
}
#[test]
fn source_boundary_rejects_traversal_and_invalid_range() {
    let dir = tempdir();
    std::fs::write(dir.path().join("safe.py"), "value = 1\n").unwrap();
    let mut span = SourceSpan {
        path: "safe.py".into(),
        start_line: 1,
        start_column: 1,
        end_line: 1,
        end_column: 10,
        start_byte: 0,
        end_byte: 9,
    };
    assert!(
        read_source(dir.path(), &span, 100)
            .unwrap()
            .contains("value")
    );
    assert!(read_source(dir.path(), &span, 2).is_err());
    for path in ["../secret", "C:/secret", "/secret", "a\\b", "a/../secret"] {
        span.path = path.into();
        assert!(read_source(dir.path(), &span, 100).is_err());
    }
}

#[test]
fn proposal_identity_is_revision_and_relation_specific() {
    let dir = tempdir();
    let store = IndexStore::open(&dir.path().join("proposals.sqlite")).unwrap();
    let mut g = graph();
    let mut concept = node("concept", NodeKind::Concept, 6);
    concept.plane = Plane::Explanation;
    g.add_node(concept);
    let dependency = store
        .propose(
            &g,
            "a",
            "concept",
            Relation::DependsOn,
            "candidate",
            "reason",
        )
        .unwrap();
    let mapping = store
        .propose(&g, "a", "concept", Relation::MapsTo, "candidate", "reason")
        .unwrap();
    assert_ne!(dependency.id, mapping.id);
    g.revision = "next".into();
    let newer = store
        .propose(&g, "a", "concept", Relation::MapsTo, "candidate", "reason")
        .unwrap();
    assert_ne!(newer.id, mapping.id);
    assert_eq!(store.proposals().unwrap().len(), 3);
    assert!(
        store
            .propose(
                &g,
                "a",
                "b",
                Relation::HasConcept,
                "invalid",
                "not a concept"
            )
            .is_err()
    );
}

#[cfg(unix)]
#[test]
fn linked_cache_ancestors_and_sources_are_rejected() {
    let dir = tempdir();
    let outside = tempdir();
    std::fs::write(outside.path().join("secret.py"), "secret = 1").unwrap();
    std::os::unix::fs::symlink(outside.path(), dir.path().join("linked")).unwrap();
    assert!(IndexStore::open(&dir.path().join("linked/cache.sqlite")).is_err());
    let span = SourceSpan {
        path: "linked/secret.py".into(),
        start_line: 1,
        start_column: 1,
        end_line: 1,
        end_column: 1,
        start_byte: 0,
        end_byte: 10,
    };
    assert!(read_source(dir.path(), &span, 100).is_err());
}

#[test]
fn duplicate_edges_merge_distinct_callsites_and_preserve_evidence_categories() {
    let mut g = graph();
    let source = |line| SourceSpan {
        path: "main.py".into(),
        start_line: line,
        start_column: 1,
        end_line: line,
        end_column: 2,
        start_byte: line * 4,
        end_byte: line * 4 + 1,
    };
    g.add_edge(
        "a",
        "b",
        Relation::Calls,
        Evidence::inferred("lexical", vec![source(2), source(1), source(2)]),
    );
    g.add_edge(
        "a",
        "b",
        Relation::Calls,
        Evidence::inferred("lexical", vec![source(3), source(1)]),
    );
    let calls: Vec<_> = g
        .edges
        .iter()
        .filter(|e| e.source == "a" && e.target == "b")
        .collect();
    assert_eq!(calls.len(), 1);
    assert_eq!(
        calls[0].evidence.sources,
        vec![source(1), source(2), source(3)]
    );
    g.add_edge(
        "a",
        "b",
        Relation::Calls,
        Evidence::observed("lexical", vec![source(4)]),
    );
    assert_eq!(
        g.edges
            .iter()
            .filter(|e| e.source == "a" && e.target == "b")
            .count(),
        2
    );
    g.validate().unwrap();
}

#[test]
fn identity_remap_reindexes_edges_without_history_or_count_change() {
    let mut previous = Graph::new("fixture", "one");
    previous.add_node(node("a", NodeKind::Function, 5));
    previous.add_node(node("b", NodeKind::Function, 5));
    previous.add_edge(
        "a",
        "b",
        Relation::Calls,
        Evidence::inferred("lexical", vec![]),
    );
    let mut current = Graph::new("fixture", "two");
    for (new, old) in [("new-a", "a"), ("new-b", "b")] {
        let mut value = node(new, NodeKind::Function, 5);
        value.fingerprint = fingerprint(old);
        current.add_node(value);
    }
    current.add_edge(
        "new-a",
        "new-b",
        Relation::Calls,
        Evidence::inferred("lexical", vec![]),
    );
    current.reconcile(&previous).unwrap();
    assert_eq!(current.edges.len(), 1);
    current.add_edge(
        "a",
        "b",
        Relation::Calls,
        Evidence::inferred("lexical", vec![]),
    );
    assert_eq!(current.edges.len(), 1);
    current.validate().unwrap();
    current.edges.reverse();
    current.reindex_edges();
    current.add_edge(
        "a",
        "b",
        Relation::Calls,
        Evidence::inferred("lexical", vec![]),
    );
    assert_eq!(current.edges.len(), 1);
}

#[test]
fn removed_origins_are_explicitly_rejected() {
    let mut g = graph();
    g.nodes.get_mut("a").unwrap().change = ChangeStatus::Removed;
    assert!(
        g.reachable("a", Direction::Both, 2, &[Relation::Calls], 10)
            .unwrap_err()
            .to_string()
            .contains("historical")
    );
    assert!(g.impact("a", Direction::Both, 2).is_err());
    assert!(g.mappings("a").is_err());
}

#[test]
fn foreign_database_is_rejected_before_schema_changes_and_legacy_cache_migrates() {
    let dir = tempdir();
    let path = dir.path().join("foreign.sqlite");
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection.execute_batch("CREATE TABLE unrelated_data(value TEXT);INSERT INTO unrelated_data VALUES('preserve');").unwrap();
    drop(connection);
    assert!(IndexStore::open(&path).is_err());
    let connection = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_schema WHERE type='table'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
    assert_eq!(
        connection
            .query_row("SELECT value FROM unrelated_data", [], |r| r
                .get::<_, String>(0))
            .unwrap(),
        "preserve"
    );
    drop(connection);
    let path = dir.path().join("legacy.sqlite");
    let store = IndexStore::open(&path).unwrap();
    store.save_graph(&graph()).unwrap();
    drop(store);
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection.pragma_update(None, "application_id", 0).unwrap();
    drop(connection);
    assert!(
        IndexStore::open(&path)
            .unwrap()
            .load_graph()
            .unwrap()
            .is_some()
    );
}

#[test]
fn matching_foreign_table_names_do_not_establish_cache_ownership() {
    let dir = tempdir();
    let cases = [
        "CREATE TABLE metadata(key INTEGER PRIMARY KEY, value BLOB);CREATE TABLE graphs(id TEXT, json TEXT);CREATE TABLE files(path TEXT, hash TEXT, json TEXT);CREATE TABLE proposals(id TEXT, json TEXT);",
        "CREATE TABLE metadata(key TEXT PRIMARY KEY,value TEXT NOT NULL);CREATE TABLE graphs(id INTEGER PRIMARY KEY CHECK(id=2),json TEXT NOT NULL);CREATE TABLE files(path TEXT PRIMARY KEY,hash TEXT NOT NULL,json TEXT NOT NULL);CREATE TABLE proposals(id TEXT PRIMARY KEY,json TEXT NOT NULL);",
        "CREATE TABLE metadata(key TEXT PRIMARY KEY,value TEXT NOT NULL);CREATE TABLE graphs(id INTEGER PRIMARY KEY CHECK(id=1),json TEXT NOT NULL);CREATE TABLE files(path TEXT PRIMARY KEY,hash TEXT NOT NULL,json TEXT NOT NULL);CREATE TABLE proposals(id TEXT PRIMARY KEY,json TEXT NOT NULL);CREATE TRIGGER surprise AFTER INSERT ON metadata BEGIN DELETE FROM graphs;END;",
        "CREATE TABLE sqliteX_unrelated(secret TEXT);",
    ];
    for (number, sql) in cases.into_iter().enumerate() {
        let path = dir.path().join(format!("foreign-{number}.sqlite"));
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection.execute_batch(sql).unwrap();
        let schema: String = connection
            .query_row(
                "SELECT group_concat(sql,';') FROM sqlite_schema WHERE sql IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        drop(connection);
        let before = std::fs::read(&path).unwrap();
        assert!(IndexStore::open(&path).is_err());
        assert_eq!(
            std::fs::read(&path).unwrap(),
            before,
            "foreign database bytes must remain untouched"
        );
        let connection = rusqlite::Connection::open(&path).unwrap();
        assert_eq!(
            connection
                .query_row("PRAGMA application_id", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            connection
                .query_row("PRAGMA journal_mode", [], |row| row.get::<_, String>(0))
                .unwrap(),
            "delete"
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT group_concat(sql,';') FROM sqlite_schema WHERE sql IS NOT NULL",
                    [],
                    |row| row.get::<_, String>(0)
                )
                .unwrap(),
            schema
        );
    }
}
