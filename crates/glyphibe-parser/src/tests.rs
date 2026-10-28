// SPDX-License-Identifier: MPL-2.0
use super::*;
use glyphibe_core::EvidenceKind;
fn tempdir() -> std::io::Result<tempfile::TempDir> {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize()?)
}

fn fixture(language: &str) -> Graph {
    analyze(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures")
            .join(language),
        &AnalyzeOptions::default(),
    )
    .unwrap()
}
fn pairs(graph: &Graph) -> BTreeSet<(String, String)> {
    graph
        .edges
        .iter()
        .filter(|e| e.relation == Relation::Calls && !e.stale)
        .map(|e| {
            (
                graph.nodes[&e.source].label.clone(),
                graph.nodes[&e.target].label.clone(),
            )
        })
        .collect()
}
fn expected(language: &str) -> BTreeSet<(String, String)> {
    serde_json::from_str(
        &fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures")
                .join(language)
                .join("calls.expected.json"),
        )
        .unwrap(),
    )
    .unwrap()
}
fn write(root: &Path, name: &str, source: &str) {
    fs::write(root.join(name), source).unwrap();
}

#[test]
fn annotated_python_calls_classes_blocks_and_uncertainty() {
    let g = fixture("python");
    g.validate().unwrap();
    assert_eq!(pairs(&g), expected("python"));
    assert!(
        g.nodes
            .values()
            .any(|n| n.kind == NodeKind::Class && n.label == "Counter")
    );
    assert!(g.nodes.values().any(|n| n.kind == NodeKind::Block));
    assert!(g.nodes.values().any(|n| n.level == 6));
    assert!(
        g.diagnostics
            .iter()
            .any(|d| d.code == "unresolved_call" && d.message.contains("callback"))
    );
    assert!(
        g.edges
            .iter()
            .filter(|e| e.relation == Relation::Calls)
            .all(|e| e.evidence.kind == EvidenceKind::Inferred)
    );
}
#[test]
fn annotated_rust_import_alias_call() {
    let g = fixture("rust");
    assert_eq!(pairs(&g), expected("rust"));
    assert!(g.edges.iter().any(|e| e.relation == Relation::Imports));
}
#[test]
fn annotated_javascript_arrow_and_alias() {
    let g = fixture("javascript");
    assert_eq!(pairs(&g), expected("javascript"));
    assert!(
        g.nodes
            .values()
            .any(|n| n.label == "calculate" && n.kind == NodeKind::Function)
    );
}
#[test]
fn annotated_typescript_tsx_cross_file() {
    let g = fixture("typescript");
    assert_eq!(pairs(&g), expected("typescript"));
    assert!(
        g.nodes
            .values()
            .any(|n| n.language.as_deref() == Some("tsx"))
    );
    assert!(
        g.nodes
            .values()
            .find(|n| n.label == "calculate")
            .unwrap()
            .tags
            .iter()
            .any(|t| t.starts_with("input:"))
    );
}
#[test]
fn no_global_name_guess_or_parameter_shadow_edge() {
    let dir = tempdir().unwrap();
    write(dir.path(), "a.py", "def target():\n    return 1\n");
    write(
        dir.path(),
        "b.py",
        "def caller(target):\n    return target()\n\ndef unknown():\n    return target()\n",
    );
    let g = analyze(dir.path(), &AnalyzeOptions::default()).unwrap();
    assert!(pairs(&g).is_empty());
    assert_eq!(
        g.diagnostics
            .iter()
            .filter(|d| d.code == "unresolved_call")
            .count(),
        2
    );
}
#[test]
fn local_assignment_shadows_import_and_dynamic_member_is_unresolved() {
    let dir = tempdir().unwrap();
    write(dir.path(), "a.py", "def target():\n    return 1\n");
    write(
        dir.path(),
        "b.py",
        "from a import target\ndef caller(obj):\n    target = obj\n    return target() + obj.target()\n",
    );
    let g = analyze(dir.path(), &AnalyzeOptions::default()).unwrap();
    assert!(pairs(&g).is_empty());
}
#[test]
fn cache_invalidation_move_and_rebuild_calls() {
    let dir = tempdir().unwrap();
    let cache = tempdir().unwrap();
    let options = AnalyzeOptions {
        cache: Some(cache.path().join("index.sqlite")),
        ..Default::default()
    };
    write(dir.path(), "a.py", "def target():\n    return 7\n");
    write(
        dir.path(),
        "main.py",
        "from a import target\ndef caller():\n    return target()\n",
    );
    let first = analyze(dir.path(), &options).unwrap();
    let target = first
        .nodes
        .values()
        .find(|n| n.label == "target")
        .unwrap()
        .id
        .clone();
    let second = analyze(dir.path(), &options).unwrap();
    assert_eq!(second.stats.cached_files, 2);
    assert_eq!(second.stats.parsed_files, 0);
    assert_eq!(pairs(&first), pairs(&second));
    fs::rename(dir.path().join("a.py"), dir.path().join("renamed.py")).unwrap();
    let moved = analyze(dir.path(), &options).unwrap();
    assert!(
        moved
            .nodes
            .get(&target)
            .is_some_and(|n| n.source.as_ref().unwrap().path == "renamed.py")
    );
    assert!(pairs(&moved).is_empty());
    assert!(
        moved
            .edges
            .iter()
            .any(|e| e.relation == Relation::Calls && e.stale)
    );
    write(
        dir.path(),
        "main.py",
        "from renamed import target\ndef caller():\n    return target()\n",
    );
    let updated = analyze(dir.path(), &options).unwrap();
    assert_eq!(updated.stats.parsed_files, 1);
    assert_eq!(pairs(&updated), pairs(&first));
}
#[test]
fn function_rename_preserves_unique_body_identity() {
    let dir = tempdir().unwrap();
    let cache = tempdir().unwrap();
    let options = AnalyzeOptions {
        cache: Some(cache.path().join("index.sqlite")),
        ..Default::default()
    };
    write(dir.path(), "a.py", "def original():\n    return 713\n");
    let a = analyze(dir.path(), &options).unwrap();
    let id = a
        .nodes
        .values()
        .find(|n| n.label == "original")
        .unwrap()
        .id
        .clone();
    write(dir.path(), "a.py", "def renamed():\n    return 713\n");
    let b = analyze(dir.path(), &options).unwrap();
    assert_eq!(b.nodes[&id].label, "renamed");
}
#[test]
fn malformed_binary_oversized_ignored_and_empty() {
    let dir = tempdir().unwrap();
    let empty = analyze(dir.path(), &AnalyzeOptions::default()).unwrap();
    assert!(empty.nodes.is_empty());
    write(dir.path(), "broken.py", "def broken(:\n    nope(\n");
    write(dir.path(), ".gitignore", "ignored.py\n");
    write(dir.path(), "ignored.py", "def hidden(): pass\n");
    fs::write(dir.path().join("binary.rs"), [0xff, 0xfe]).unwrap();
    let g = analyze(dir.path(), &AnalyzeOptions::default()).unwrap();
    assert_eq!(g.stats.discovered_files, 2);
    assert!(g.diagnostics.iter().any(|d| d.code == "syntax_error"));
    assert!(g.diagnostics.iter().any(|d| d.code == "invalid_utf8"));
    let bounded = analyze(
        dir.path(),
        &AnalyzeOptions {
            max_file_bytes: 4,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(bounded.stats.skipped_files, 2);
}
#[test]
fn cancellation_limits_and_cache_pollution_rejected() {
    let dir = tempdir().unwrap();
    write(dir.path(), "a.py", "pass");
    assert!(
        analyze(
            dir.path(),
            &AnalyzeOptions {
                cancel: Some(Arc::new(AtomicBool::new(true))),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        analyze(
            dir.path(),
            &AnalyzeOptions {
                cache: Some(dir.path().join("polluting.sqlite")),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        analyze(
            dir.path(),
            &AnalyzeOptions {
                max_files: 0,
                ..Default::default()
            }
        )
        .is_err()
    );
}
#[test]
fn progress_is_real_and_never_reparses_for_queries() {
    let dir = tempdir().unwrap();
    write(dir.path(), "a.py", "def f():\n    return 1\n");
    let events = Arc::new(std::sync::Mutex::new(vec![]));
    let output = events.clone();
    let options = AnalyzeOptions {
        progress: Some(Arc::new(move |event| output.lock().unwrap().push(event))),
        ..Default::default()
    };
    let g = analyze(dir.path(), &options).unwrap();
    let events = events.lock().unwrap();
    assert_eq!(events[0].phase, "discovered");
    assert_eq!(events.last().unwrap().completed, 1);
    assert_eq!(g.stats.parsed_files, 1);
}

#[test]
fn cumulative_and_extraction_memory_budgets_fail_explicitly() {
    let dir = tempdir().unwrap();
    write(dir.path(), "a.py", "def a():\n    return 1\n");
    write(dir.path(), "b.py", "def b():\n    return 2\n");
    let error = analyze(
        dir.path(),
        &AnalyzeOptions {
            max_total_bytes: 30,
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(error.to_string().contains("cumulative"));
    let error = analyze(
        dir.path(),
        &AnalyzeOptions {
            max_graph_nodes: 2,
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(error.to_string().contains("budget"));
}

#[test]
fn fingerprints_preserve_literal_whitespace_but_ignore_formatting() {
    let a = parse_source(
        "a.py",
        "def a():\n    return 'a b'\n",
        &AnalyzeOptions::default(),
    )
    .unwrap();
    let b = parse_source(
        "a.py",
        "def b():\n    return 'ab'\n",
        &AnalyzeOptions::default(),
    )
    .unwrap();
    let c = parse_source(
        "a.py",
        "def c():\n    return    'a b'   \n",
        &AnalyzeOptions::default(),
    )
    .unwrap();
    assert_ne!(a.facts[0].body, b.facts[0].body);
    assert_eq!(a.facts[0].body, c.facts[0].body);
}

#[test]
fn deeply_nested_malformed_source_uses_iterative_traversal() {
    let source = format!(
        "def nested():\n    return {}1{}\n",
        "(".repeat(1_000),
        ")".repeat(999)
    );
    let result = parse_source("deep.py", &source, &AnalyzeOptions::default());
    assert!(
        result.is_err()
            || result
                .unwrap()
                .diagnostics
                .iter()
                .any(|d| d.code == "syntax_error")
    );
}

#[test]
fn cyclic_imports_remain_real_edges() {
    let dir = tempdir().unwrap();
    write(dir.path(), "a.py", "import b\ndef a(): return 1\n");
    write(dir.path(), "b.py", "import a\ndef b(): return 2\n");
    let g = analyze(dir.path(), &AnalyzeOptions::default()).unwrap();
    assert_eq!(
        g.edges
            .iter()
            .filter(|e| e.relation == Relation::Imports)
            .count(),
        2
    );
}

#[test]
fn class_members_do_not_become_bare_lexical_bindings() {
    let dir = tempdir().unwrap();
    write(
        dir.path(),
        "a.py",
        "class Counter:\n    def value(self): return 1\n    def bad(self): return value()\n    def good(self): return self.value()\n",
    );
    let g = analyze(dir.path(), &AnalyzeOptions::default()).unwrap();
    assert_eq!(pairs(&g), BTreeSet::from([("good".into(), "value".into())]));
}

#[test]
fn javascript_import_requires_export_and_python_attributes_are_not_global_references() {
    let dir = tempdir().unwrap();
    write(dir.path(), "helper.js", "function hidden() { return 1; }\n");
    write(
        dir.path(),
        "main.js",
        "import {hidden} from './helper';\nfunction caller() { return hidden(); }\n",
    );
    let g = analyze(dir.path(), &AnalyzeOptions::default()).unwrap();
    assert!(pairs(&g).is_empty());
    let python = tempdir().unwrap();
    write(
        python.path(),
        "a.py",
        "def target(): return 1\ndef caller(obj): return obj.target()\n",
    );
    let g = analyze(python.path(), &AnalyzeOptions::default()).unwrap();
    let caller = g.nodes.values().find(|n| n.label == "caller").unwrap();
    let target = g.nodes.values().find(|n| n.label == "target").unwrap();
    assert!(
        !g.edges
            .iter()
            .any(|e| e.source == caller.id && e.target == target.id && e.relation.is_dependency())
    );
}

#[test]
fn explicit_rust_trait_and_typescript_interface_implementations() {
    let dir = tempdir().unwrap();
    write(
        dir.path(),
        "lib.rs",
        "pub trait Value { fn value(&self)->i32; }\npub struct Counter;\nimpl Value for Counter {fn value(&self)->i32 {2}}\n",
    );
    let g = analyze(dir.path(), &AnalyzeOptions::default()).unwrap();
    let implementations: Vec<_> = g
        .edges
        .iter()
        .filter(|e| e.relation == Relation::Implements)
        .collect();
    assert_eq!(implementations.len(), 1);
    assert_eq!(g.nodes[&implementations[0].source].label, "Counter");
    assert_eq!(g.nodes[&implementations[0].target].label, "Value");
    assert_eq!(implementations[0].evidence.kind, EvidenceKind::Observed);
    let ts = tempdir().unwrap();
    write(
        ts.path(),
        "types.ts",
        "interface Value { value():number; }\nclass Base {}\nclass Counter extends Base implements Value {value():number{return 2;}}\n",
    );
    let g = analyze(ts.path(), &AnalyzeOptions::default()).unwrap();
    assert!(g.edges.iter().any(|e| e.relation == Relation::Implements
        && g.nodes[&e.source].label == "Counter"
        && g.nodes[&e.target].label == "Value"));
    assert!(g.edges.iter().any(|e| e.relation == Relation::References
        && g.nodes[&e.source].label == "Counter"
        && g.nodes[&e.target].label == "Base"));
}

#[test]
fn python_base_class_is_evidence_and_dynamic_base_is_unresolved() {
    let dir = tempdir().unwrap();
    write(
        dir.path(),
        "a.py",
        "class Base: pass\nclass Child(Base): pass\nclass Dynamic(factory()): pass\n",
    );
    let g = analyze(dir.path(), &AnalyzeOptions::default()).unwrap();
    assert!(g.edges.iter().any(|e| e.relation == Relation::References
        && g.nodes[&e.source].label == "Child"
        && g.nodes[&e.target].label == "Base"));
    assert!(
        g.diagnostics
            .iter()
            .any(|d| d.code == "unresolved_type_relation")
    );
}

#[test]
fn nested_rust_module_paths_do_not_resolve_to_sibling_files() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("outer")).unwrap();
    write(dir.path(), "lib.rs", "mod outer;\n");
    write(
        dir.path(),
        "outer.rs",
        "mod child;\nfn caller()->i32 {child::target()}\n",
    );
    write(dir.path(), "outer/child.rs", "pub fn target()->i32 {1}\n");
    write(dir.path(), "child.rs", "pub fn unrelated()->i32 {9}\n");
    let g = analyze(dir.path(), &AnalyzeOptions::default()).unwrap();
    assert_eq!(
        pairs(&g),
        BTreeSet::from([("caller".into(), "target".into())])
    );
}

#[test]
fn rust_crate_paths_are_bound_to_nearest_package_in_workspace() {
    let dir = tempdir().unwrap();
    for package in ["one", "two"] {
        let root = dir.path().join("crates").join(package);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            format!("[package]\nname='{package}'\nversion='0.1.0'\n"),
        )
        .unwrap();
        fs::write(
            root.join("src/helper.rs"),
            format!("pub fn {package}()->i32{{1}}\n"),
        )
        .unwrap();
        fs::write(
            root.join("src/lib.rs"),
            format!("mod helper;\nfn caller_{package}()->i32{{crate::helper::{package}()}}\n"),
        )
        .unwrap();
    }
    let g = analyze(dir.path(), &AnalyzeOptions::default()).unwrap();
    assert_eq!(
        pairs(&g),
        BTreeSet::from([
            ("caller_one".into(), "one".into()),
            ("caller_two".into(), "two".into())
        ])
    );
}

#[test]
fn imported_typescript_implements_and_javascript_base_relations() {
    let dir = tempdir().unwrap();
    write(
        dir.path(),
        "types.ts",
        "export interface Value {value():number;}\n",
    );
    write(
        dir.path(),
        "main.ts",
        "import {Value as Contract} from './types';\nexport class Counter implements Contract {value():number{return 2;}}\n",
    );
    let g = analyze(dir.path(), &AnalyzeOptions::default()).unwrap();
    assert!(
        g.edges
            .iter()
            .any(|e| e.relation == Relation::Implements && g.nodes[&e.target].label == "Value")
    );
    let js = tempdir().unwrap();
    write(
        js.path(),
        "main.js",
        "class Base {}\nclass Child extends Base {}\n",
    );
    let g = analyze(js.path(), &AnalyzeOptions::default()).unwrap();
    assert!(g.edges.iter().any(|e| e.relation == Relation::References
        && g.nodes[&e.source].label == "Child"
        && g.nodes[&e.target].label == "Base"));
}

#[test]
fn bounded_reader_never_reads_past_limit_plus_one_and_supports_cancellation() {
    use std::io::{Cursor, Read};
    let mut input = Cursor::new(vec![b'x'; 100_000]);
    assert!(bounded_read(&mut input, 19, &AnalyzeOptions::default()).is_err());
    assert_eq!(input.position(), 20);
    let mut input = Cursor::new(vec![b'x'; 100_000]);
    let options = AnalyzeOptions {
        cancel: Some(Arc::new(AtomicBool::new(true))),
        ..Default::default()
    };
    assert!(bounded_read(&mut input, 100, &options).is_err());
    assert_eq!(input.position(), 0);
    assert_eq!(
        bounded_read(Cursor::new(b"abc"), 3, &AnalyzeOptions::default()).unwrap(),
        b"abc"
    );
    let mut buffer = [0u8; 1];
    assert_eq!(input.read(&mut buffer).unwrap(), 1);
}

#[test]
fn nested_declarations_fail_explicitly_at_scope_budget() {
    let mut source = String::new();
    for i in 0..260 {
        source.push_str(&format!("function f{i}() {{\n"));
    }
    source.push_str("return 1;\n");
    source.push_str(&"}\n".repeat(260));
    let error = parse_source("nested.js", &source, &AnalyzeOptions::default())
        .err()
        .expect("nested declaration scope must be bounded");
    assert!(error.to_string().contains("scope safety budget"));
}
