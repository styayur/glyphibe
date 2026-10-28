// SPDX-License-Identifier: MPL-2.0
//! Bounded, non-executing Tree-sitter analysis. Unresolved calls stay diagnostics.
use anyhow::{Context, Result, bail};
use glyphibe_core::{
    ChangeStatus, Diagnostic, Evidence, Graph, IndexStore, Node, NodeKind, Plane, Relation,
    SourceSpan, fingerprint, stable_id,
};
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    ops::ControlFlow,
    path::{Component, Path, PathBuf},
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};
use tree_sitter::{Language, Node as Syntax, ParseOptions, Parser};

const CACHE_VERSION: &str = "parser-9";
const AST_BUDGET: usize = 150_000;
const MAX_SCOPE_BYTES: usize = 16 * 1024;
const MAX_DECLARATION_DEPTH: usize = 256;

#[derive(Clone)]
pub struct AnalyzeOptions {
    pub cache: Option<PathBuf>,
    pub max_file_bytes: u64,
    pub max_files: usize,
    pub max_total_bytes: u64,
    pub max_graph_nodes: usize,
    pub cancel: Option<Arc<AtomicBool>>,
    pub progress: Option<Arc<dyn Fn(ScanProgress) + Send + Sync>>,
}
#[derive(Clone, Serialize)]
pub struct ScanProgress {
    pub phase: String,
    pub completed: usize,
    pub total: usize,
    pub path: Option<String>,
}
impl Default for AnalyzeOptions {
    fn default() -> Self {
        Self {
            cache: None,
            max_file_bytes: 2 * 1024 * 1024,
            max_files: 20_000,
            max_total_bytes: 256 * 1024 * 1024,
            max_graph_nodes: 250_000,
            cancel: None,
            progress: None,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Fact {
    key: String,
    label: String,
    kind: String,
    parent: String,
    span: SourceSpan,
    summary: String,
    body: String,
    scope: String,
    tags: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Import {
    module: String,
    bindings: Vec<(String, String)>,
    scope: String,
    span: SourceSpan,
}
#[derive(Clone, Serialize, Deserialize)]
struct Call {
    owner: String,
    scope: String,
    spelling: String,
    span: SourceSpan,
}
#[derive(Clone, Serialize, Deserialize)]
struct Reference {
    owner: String,
    scope: String,
    name: String,
    span: SourceSpan,
}
#[derive(Clone, Serialize, Deserialize)]
struct TypeRelation {
    source_key: String,
    source_name: String,
    target_name: String,
    scope: String,
    span: SourceSpan,
    implements: bool,
}
#[derive(Clone, Serialize, Deserialize)]
struct ParsedFile {
    path: String,
    language: String,
    facts: Vec<Fact>,
    imports: Vec<Import>,
    calls: Vec<Call>,
    references: Vec<Reference>,
    type_relations: Vec<TypeRelation>,
    shadows: BTreeMap<String, BTreeSet<String>>,
    rebindings: BTreeMap<String, BTreeSet<String>>,
    diagnostics: Vec<Diagnostic>,
    file_span: SourceSpan,
    source_hash: String,
    crate_root: Option<String>,
}
fn conventional_crate_root(root: &Path, path: &Path) -> Option<String> {
    for parent in path.parent()?.ancestors() {
        if !parent.starts_with(root) {
            break;
        }
        let manifest = parent.join("Cargo.toml");
        if fs::symlink_metadata(manifest).is_ok_and(|m| m.is_file() && !m.file_type().is_symlink())
        {
            let source = parent.join("src");
            if path.starts_with(&source) {
                return Some(
                    source
                        .strip_prefix(root)
                        .ok()?
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
            return None;
        }
    }
    None
}

fn cancelled(options: &AnalyzeOptions) -> bool {
    options
        .cancel
        .as_ref()
        .is_some_and(|x| x.load(Ordering::Relaxed))
}
fn bounded_read(
    reader: impl std::io::Read,
    limit: u64,
    options: &AnalyzeOptions,
) -> Result<Vec<u8>> {
    use std::io::Read;
    let mut reader = reader.take(limit.saturating_add(1));
    let mut output = vec![];
    let mut buffer = [0u8; 8192];
    loop {
        if cancelled(options) {
            bail!("analysis cancelled");
        }
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        output.extend_from_slice(&buffer[..read]);
        if output.len() as u64 > limit {
            bail!("source file or cumulative byte budget exceeded while reading");
        }
    }
    Ok(output)
}
fn language(path: &str) -> Option<(&'static str, Language)> {
    match Path::new(path).extension()?.to_str()? {
        "py" => Some(("python", tree_sitter_python::LANGUAGE.into())),
        "rs" => Some(("rust", tree_sitter_rust::LANGUAGE.into())),
        "js" | "mjs" | "cjs" | "jsx" => {
            Some(("javascript", tree_sitter_javascript::LANGUAGE.into()))
        }
        "ts" | "mts" | "cts" => Some((
            "typescript",
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        )),
        "tsx" => Some(("tsx", tree_sitter_typescript::LANGUAGE_TSX.into())),
        _ => None,
    }
}
fn text<'a>(node: Syntax<'_>, source: &'a str) -> &'a str {
    source.get(node.byte_range()).unwrap_or("")
}
fn span(node: Syntax<'_>, path: &str) -> SourceSpan {
    let start = node.start_position();
    let end = node.end_position();
    SourceSpan {
        path: path.to_owned(),
        start_line: start.row + 1,
        start_column: start.column + 1,
        end_line: end.row + 1,
        end_column: end.column + 1,
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
    }
}
fn compact(value: &str) -> String {
    value
        .split_whitespace()
        .flat_map(|word| word.chars().chain(std::iter::once(' ')))
        .take(240)
        .collect::<String>()
        .trim_end()
        .to_owned()
}
fn syntax_fingerprints(
    root: Syntax<'_>,
    source: &str,
    options: &AnalyzeOptions,
) -> Result<BTreeMap<usize, String>> {
    let mut hashes = BTreeMap::new();
    let mut stack = vec![(root, false)];
    let mut visited = 0usize;
    while let Some((node, ready)) = stack.pop() {
        if cancelled(options) {
            bail!("analysis cancelled");
        }
        if ready {
            let mut cursor = node.walk();
            let children: Vec<_> = node
                .children(&mut cursor)
                .filter(|n| !matches!(n.kind(), "comment" | "line_comment" | "block_comment"))
                .collect();
            let hash = if children.is_empty() {
                stable_id(&[node.kind(), text(node, source)])
            } else {
                let parts: Vec<_> = std::iter::once(node.kind())
                    .chain(
                        children
                            .iter()
                            .filter_map(|child| hashes.get(&child.id()).map(String::as_str)),
                    )
                    .collect();
                stable_id(&parts)
            };
            hashes.insert(node.id(), hash);
        } else {
            visited += 1;
            if visited > AST_BUDGET {
                bail!("AST safety budget exceeded ({AST_BUDGET} nodes)");
            }
            stack.push((node, true));
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                stack.push((child, false));
            }
        }
    }
    Ok(hashes)
}
fn export_context(node: Syntax<'_>) -> Option<Syntax<'_>> {
    let mut parent = node.parent();
    for _ in 0..4 {
        let value = parent?;
        if value.kind() == "export_statement" {
            return Some(value);
        }
        if matches!(
            value.kind(),
            "program" | "statement_block" | "function_definition" | "class_body"
        ) {
            return None;
        }
        parent = value.parent();
    }
    None
}
fn node_kind(kind: &str) -> NodeKind {
    match kind {
        "class" => NodeKind::Class,
        "function" => NodeKind::Function,
        "block" => NodeKind::Block,
        "statement" => NodeKind::Statement,
        _ => NodeKind::Expression,
    }
}
struct NodeDraft {
    id: String,
    label: String,
    kind: NodeKind,
    level: u8,
    source: Option<SourceSpan>,
    summary: String,
    hash: String,
    language: Option<String>,
    tags: Vec<String>,
}
fn new_node(graph: &Graph, draft: NodeDraft) -> Node {
    let NodeDraft {
        id,
        label,
        kind,
        level,
        source,
        summary,
        hash,
        language,
        tags,
    } = draft;
    let sources = source.iter().cloned().collect();
    Node {
        id,
        label,
        kind,
        level,
        plane: Plane::Architecture,
        repository: graph.repository.clone(),
        revision: graph.revision.clone(),
        language,
        source,
        summary,
        evidence: Evidence::observed("tree-sitter syntax", sources),
        fingerprint: hash,
        tags,
        knowledge: vec![],
        change: ChangeStatus::Added,
    }
}

/// Analyze regular files only. Never follows symlinks, runs project code or writes inside root.
pub fn analyze(root: &Path, options: &AnalyzeOptions) -> Result<Graph> {
    let started = Instant::now();
    let root = root
        .canonicalize()
        .context("analysis root does not exist")?;
    if !root.is_dir() {
        bail!("analysis root must be a directory");
    }
    if options.max_files == 0
        || options.max_file_bytes == 0
        || options.max_total_bytes == 0
        || options.max_graph_nodes == 0
    {
        bail!("scan limits must be positive");
    }
    if let Some(cache) = &options.cache {
        let absolute = if cache.is_absolute() {
            cache.clone()
        } else {
            std::env::current_dir()?.join(cache)
        };
        let mut existing = absolute.as_path();
        while !existing.exists() {
            existing = existing
                .parent()
                .context("cache path has no existing ancestor")?;
        }
        if existing.canonicalize()?.starts_with(&root) {
            bail!("index cache must be outside analyzed project");
        }
    }
    let revision = installed_git(&root, std::env::var_os("PATH").as_deref())
        .and_then(|git| {
            Command::new(&git)
                .current_dir(git.parent()?)
                .args(["--no-optional-locks", "-C"])
                .arg(&root)
                .args(["rev-parse", "--verify", "HEAD"])
                .output()
                .ok()
        })
        .filter(|x| x.status.success())
        .map(|x| String::from_utf8_lossy(&x.stdout).trim().to_owned())
        .filter(|revision| {
            matches!(revision.len(), 40 | 64) && revision.bytes().all(|b| b.is_ascii_hexdigit())
        })
        .unwrap_or_else(|| "unversioned".into());
    // Never publish absolute user paths: repository identity is hashed canonical path.
    let repository = stable_id(&["repository", &root.to_string_lossy()]);
    let mut graph = Graph::new(repository.clone(), revision);
    let mut store = options
        .cache
        .as_ref()
        .map(|p| IndexStore::open(p))
        .transpose()?;
    if let Some(store) = store.as_mut() {
        store.bind_repository(&root)?;
    }
    let previous = store
        .as_ref()
        .map(|s| s.load_graph())
        .transpose()?
        .flatten()
        .filter(|g| g.repository == repository);
    let mut walk = WalkBuilder::new(&root);
    walk.follow_links(false)
        .hidden(false)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(true)
        .require_git(false);
    walk.filter_entry(|entry| {
        !matches!(
            entry.file_name().to_str(),
            Some(
                ".git"
                    | "node_modules"
                    | "target"
                    | "dist"
                    | "build"
                    | "venv"
                    | ".venv"
                    | "__pycache__"
                    | ".cache"
                    | ".next"
                    | "coverage"
            )
        )
    });
    let mut files = vec![];
    for entry in walk.build() {
        if cancelled(options) {
            bail!("analysis cancelled");
        }
        match entry {
            Ok(entry) if entry.file_type().is_some_and(|t| t.is_file()) => {
                let path = entry.path();
                let relative = path
                    .strip_prefix(&root)?
                    .to_string_lossy()
                    .replace('\\', "/");
                if language(&relative).is_some() {
                    files.push((relative, path.to_owned()));
                }
                if files.len() > options.max_files {
                    bail!("file limit exceeded ({})", options.max_files);
                }
            }
            Ok(_) => {}
            Err(_) => graph.diagnostics.push(Diagnostic {
                code: "discovery_error".into(),
                message: "A directory entry could not be read".into(),
                path: None,
            }),
        }
    }
    files.sort();
    graph.stats.discovered_files = files.len();
    if let Some(progress) = &options.progress {
        progress(ScanProgress {
            phase: "discovered".into(),
            completed: 0,
            total: files.len(),
            path: None,
        });
    }
    let mut parsed = vec![];
    let mut extracted_items = 0usize;
    let mut cached_paths = BTreeSet::new();
    for (index, (relative, path)) in files.into_iter().enumerate() {
        if cancelled(options) {
            bail!("analysis cancelled");
        }
        // Re-check canonical containment and regular type immediately before opening.
        let metadata = fs::symlink_metadata(&path)?;
        if !metadata.is_file()
            || !path.canonicalize()?.starts_with(&root)
            || metadata.len() > options.max_file_bytes
        {
            graph.stats.skipped_files += 1;
            graph.diagnostics.push(Diagnostic {
                code: "file_skipped".into(),
                message: "File is outside scan safety or size limits".into(),
                path: Some(relative),
            });
            continue;
        }
        if graph.stats.source_bytes.saturating_add(metadata.len()) > options.max_total_bytes {
            bail!(
                "cumulative source byte budget exceeded ({})",
                options.max_total_bytes
            );
        }
        let remaining = options
            .max_total_bytes
            .saturating_sub(graph.stats.source_bytes);
        let bytes = bounded_read(
            fs::File::open(&path)?,
            options.max_file_bytes.min(remaining),
            options,
        )?;
        if bytes.len() as u64 > options.max_file_bytes {
            bail!("file grew beyond scan size limit");
        }
        graph.stats.source_bytes += bytes.len() as u64;
        if graph.stats.source_bytes > options.max_total_bytes {
            bail!("cumulative source byte budget exceeded");
        }
        let source = match std::str::from_utf8(&bytes) {
            Ok(s) => s,
            Err(_) => {
                graph.stats.skipped_files += 1;
                graph.diagnostics.push(Diagnostic {
                    code: "invalid_utf8".into(),
                    message: "Source is not UTF-8".into(),
                    path: Some(relative),
                });
                continue;
            }
        };
        let crate_root = if relative.ends_with(".rs") {
            conventional_crate_root(&root, &path)
        } else {
            None
        };
        let hash = fingerprint(&format!(
            "{CACHE_VERSION}\0{}\0{source}",
            crate_root.as_deref().unwrap_or("")
        ));
        let cache_key = format!("{repository}/{relative}");
        cached_paths.insert(cache_key.clone());
        let cached = store
            .as_ref()
            .map(|s| s.load_file(&cache_key, &hash))
            .transpose()?
            .flatten();
        let result =
            if let Some(data) = cached.and_then(|s| serde_json::from_str::<ParsedFile>(&s).ok()) {
                graph.stats.cached_files += 1;
                data
            } else {
                let mut data = parse_source(&relative, source, options)?;
                data.crate_root = crate_root;
                graph.stats.parsed_files += 1;
                if let Some(store) = store.as_mut() {
                    store.save_file(&cache_key, &hash, &serde_json::to_string(&data)?)?;
                }
                data
            };
        graph.diagnostics.extend(result.diagnostics.clone());
        extracted_items += 1 + result.facts.len() + result.calls.len() + result.references.len();
        if extracted_items > options.max_graph_nodes {
            bail!(
                "graph and extraction item budget exceeded ({})",
                options.max_graph_nodes
            );
        }
        parsed.push(result);
        if let Some(progress) = &options.progress {
            progress(ScanProgress {
                phase: "parse".into(),
                completed: index + 1,
                total: graph.stats.discovered_files,
                path: Some(relative),
            });
        }
    }
    build_graph(&mut graph, &parsed, options)?;
    graph.stats.elapsed_ms = started.elapsed().as_millis() as u64;
    if let Some(previous) = previous {
        graph.reconcile(&previous)?;
    }
    graph.validate()?;
    if let Some(store) = store.as_mut() {
        store.prune_files(&cached_paths)?;
        store.save_graph(&graph)?;
    }
    Ok(graph)
}

/// Resolve an installed executable without Windows' implicit current-directory
/// search. Relative PATH entries and executables inside the scanned root are
/// untrusted. Git is optional: absence leaves ordinary directory analysis usable.
fn installed_git(root: &Path, path: Option<&std::ffi::OsStr>) -> Option<PathBuf> {
    let root = root.canonicalize().ok()?;
    let executable = if cfg!(windows) { "git.exe" } else { "git" };
    for directory in std::env::split_paths(path?) {
        if !directory.is_absolute() {
            continue;
        }
        let Ok(candidate) = directory.join(executable).canonicalize() else {
            continue;
        };
        if candidate.is_file() && !candidate.starts_with(&root) {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod executable_boundary_tests {
    use super::*;
    #[test]
    fn project_and_relative_path_git_shadows_are_never_executed() {
        let project = tempfile::tempdir().unwrap();
        let installed = tempfile::tempdir().unwrap();
        let name = if cfg!(windows) { "git.exe" } else { "git" };
        fs::write(
            project.path().join(name),
            "untrusted executable placeholder",
        )
        .unwrap();
        fs::write(
            installed.path().join(name),
            "trusted path selection fixture; never executed",
        )
        .unwrap();
        let only_project = std::env::join_paths([project.path(), Path::new(".")]).unwrap();
        assert!(installed_git(project.path(), Some(&only_project)).is_none());
        let mixed = std::env::join_paths([project.path(), installed.path()]).unwrap();
        assert_eq!(
            installed_git(project.path(), Some(&mixed)),
            Some(installed.path().join(name).canonicalize().unwrap())
        );
    }
}

fn parse_source(path: &str, source: &str, options: &AnalyzeOptions) -> Result<ParsedFile> {
    let (lang, grammar) = language(path).context("unsupported language")?;
    let mut parser = Parser::new();
    parser.set_language(&grammar)?;
    let mut progress = |_: &tree_sitter::ParseState| {
        if cancelled(options) {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    };
    let tree = parser
        .parse_with_options(
            &mut |offset, _| &source.as_bytes()[offset..],
            None,
            Some(ParseOptions::new().progress_callback(&mut progress)),
        )
        .context("parsing cancelled")?;
    let mut out = ParsedFile {
        path: path.into(),
        language: lang.into(),
        facts: vec![],
        imports: vec![],
        calls: vec![],
        references: vec![],
        type_relations: vec![],
        shadows: BTreeMap::new(),
        rebindings: BTreeMap::new(),
        diagnostics: vec![],
        file_span: span(tree.root_node(), path),
        source_hash: fingerprint(source),
        crate_root: None,
    };
    let hashes = syntax_fingerprints(tree.root_node(), source, options)?;
    if tree.root_node().has_error() {
        out.diagnostics.push(Diagnostic {
            code: "syntax_error".into(),
            message: "Tree-sitter recovered malformed syntax; affected calls remain uncertain"
                .into(),
            path: Some(path.into()),
        });
    }
    let mut stack = vec![(
        tree.root_node(),
        String::new(),
        String::new(),
        String::new(),
        false,
    )];
    let mut occurrences: BTreeMap<String, usize> = BTreeMap::new();
    let mut visited = 0;
    while let Some((node, mut parent, mut scope, mut owner, in_parameters)) = stack.pop() {
        if scope.len() > MAX_SCOPE_BYTES
            || scope.match_indices("::").count() >= MAX_DECLARATION_DEPTH
        {
            bail!("declaration scope safety budget exceeded");
        }
        visited += 1;
        if visited > AST_BUDGET {
            out.diagnostics.push(Diagnostic {
                code: "ast_limit".into(),
                message: "AST budget exceeded; remaining syntax omitted".into(),
                path: Some(path.into()),
            });
            break;
        }
        if visited % 256 == 0 && cancelled(options) {
            bail!("analysis cancelled");
        }
        let kind = node.kind();
        if matches!(
            kind,
            "import_statement" | "import_from_statement" | "use_declaration" | "mod_item"
        ) {
            extract_imports(node, source, path, &scope, lang, &mut out.imports);
        }
        let declaration = match kind {
            "function_definition"
            | "function_item"
            | "function_signature_item"
            | "function_declaration"
            | "generator_function_declaration"
            | "method_definition"
            | "method_signature" => Some("function"),
            "class_definition"
            | "class_declaration"
            | "struct_item"
            | "enum_item"
            | "trait_item"
            | "interface_declaration"
            | "type_alias_declaration" => Some("class"),
            "arrow_function" | "function_expression" | "generator_function" => Some("function"),
            _ => None,
        };
        let mut declaration_name = node
            .child_by_field_name("name")
            .map(|n| text(n, source).to_owned());
        if declaration.is_some() && declaration_name.is_none() {
            declaration_name = node
                .parent()
                .and_then(|p| p.child_by_field_name("name"))
                .map(|n| text(n, source).to_owned());
        }
        let is_block = matches!(
            kind,
            "if_statement"
                | "if_expression"
                | "for_statement"
                | "for_expression"
                | "for_in_statement"
                | "while_statement"
                | "while_expression"
                | "loop_expression"
                | "try_statement"
                | "except_clause"
                | "catch_clause"
                | "match_expression"
                | "impl_item"
        );
        let is_statement = matches!(
            kind,
            "return_statement"
                | "return_expression"
                | "let_declaration"
                | "assignment"
                | "variable_declarator"
                | "expression_statement"
                | "throw_statement"
                | "raise_statement"
                | "break_statement"
                | "continue_statement"
        );
        let is_expression = matches!(
            kind,
            "binary_expression"
                | "binary_operator"
                | "comparison_operator"
                | "boolean_operator"
                | "conditional_expression"
                | "await_expression"
                | "call"
                | "call_expression"
                | "new_expression"
        );
        if declaration.is_some() || is_block || is_statement || is_expression {
            let entity = declaration.unwrap_or(if is_block {
                "block"
            } else if is_statement {
                "statement"
            } else {
                "expression"
            });
            let label = declaration_name.unwrap_or_else(|| {
                if kind == "impl_item" {
                    node.child_by_field_name("type")
                        .map(|n| format!("impl {}", text(n, source)))
                        .unwrap_or_else(|| "impl".into())
                } else {
                    compact(text(node, source))
                }
            });
            let base = format!(
                "{scope}/{entity}/{}",
                if declaration.is_some() {
                    label.as_str()
                } else {
                    kind
                }
            );
            let occurrence = occurrences.entry(base.clone()).or_default();
            let key = format!("{base}#{occurrence}");
            *occurrence += 1;
            let body_node = node.child_by_field_name("body").unwrap_or(node);
            let body = hashes.get(&body_node.id()).cloned().unwrap_or_default();
            let mut tags = vec![format!("syntax:{kind}")];
            if lang == "python" && kind == "function_definition" {
                tags.extend(python_receiver_tags(node, source));
            }
            if declaration.is_some() {
                out.rebindings
                    .entry(scope.clone())
                    .or_default()
                    .insert(label.clone());
            }
            if declaration.is_some()
                && (named_children(node)
                    .iter()
                    .any(|n| n.kind() == "visibility_modifier")
                    || export_context(node).is_some())
            {
                tags.push("exported".into());
            }
            if matches!(kind, "interface_declaration" | "trait_item") {
                tags.push("interface".into());
            }
            if entity == "function"
                && (label.starts_with("test_")
                    || node.prev_named_sibling().is_some_and(|p| {
                        p.kind() == "attribute_item"
                            && (text(p, source).contains("#[test]")
                                || text(p, source).contains("#[tokio::test"))
                    }))
            {
                tags.push("test".into());
            }
            if export_context(node)
                .is_some_and(|p| text(p, source).trim_start().starts_with("export default"))
            {
                tags.push("export-default".into());
            }
            for (field, tag) in [
                ("parameters", "input"),
                ("return_type", "output"),
                ("condition", "condition"),
                ("value", "value"),
            ] {
                if let Some(value) = node.child_by_field_name(field) {
                    tags.push(format!("{tag}:{}", compact(text(value, source))));
                }
            }
            if matches!(
                kind,
                "for_statement" | "for_expression" | "for_in_statement"
            ) {
                let iterator = if kind == "for_expression" {
                    node.child_by_field_name("value")
                } else {
                    node.child_by_field_name("right")
                };
                if let Some(iterator) = iterator {
                    tags.push(format!("iterator:{}", compact(text(iterator, source))));
                }
            }
            if matches!(
                kind,
                "assignment" | "let_declaration" | "variable_declarator"
            ) {
                if let Some(left) = node
                    .child_by_field_name("left")
                    .or_else(|| node.child_by_field_name("name"))
                    .or_else(|| node.child_by_field_name("pattern"))
                {
                    tags.push(format!("binding:{}", compact(text(left, source))));
                }
                if let Some(right) = node.child_by_field_name("right") {
                    tags.push(format!("value:{}", compact(text(right, source))));
                }
            }
            if node.has_error() {
                tags.push("syntax-error".into());
            }
            out.facts.push(Fact {
                key: key.clone(),
                label: label.clone(),
                kind: entity.into(),
                parent: parent.clone(),
                span: span(node, path),
                summary: compact(text(node, source)),
                body,
                scope: scope.clone(),
                tags,
            });
            if entity == "class" {
                if let Some(bases) = node.child_by_field_name("superclasses") {
                    for base in named_children(bases) {
                        out.type_relations.push(TypeRelation {
                            source_key: key.clone(),
                            source_name: String::new(),
                            target_name: type_name(base, source),
                            scope: scope.clone(),
                            span: span(base, path),
                            implements: false,
                        });
                    }
                }
                for heritage in named_children(node)
                    .into_iter()
                    .filter(|n| matches!(n.kind(), "class_heritage" | "extends_type_clause"))
                {
                    for clause in named_children(heritage) {
                        let is_clause =
                            matches!(clause.kind(), "extends_clause" | "implements_clause");
                        let targets = if is_clause {
                            named_children(clause)
                        } else {
                            vec![clause]
                        };
                        for target in targets.into_iter().filter(|n| n.kind() != "type_arguments") {
                            out.type_relations.push(TypeRelation {
                                source_key: key.clone(),
                                source_name: String::new(),
                                target_name: type_name(target, source),
                                scope: scope.clone(),
                                span: span(target, path),
                                implements: clause.kind() == "implements_clause",
                            });
                        }
                    }
                }
            }
            if kind == "impl_item"
                && let (Some(trait_node), Some(type_node)) = (
                    node.child_by_field_name("trait"),
                    node.child_by_field_name("type"),
                )
            {
                out.type_relations.push(TypeRelation {
                    source_key: String::new(),
                    source_name: type_name(type_node, source),
                    target_name: type_name(trait_node, source),
                    scope: scope.clone(),
                    span: span(node, path),
                    implements: true,
                });
            }
            parent = key.clone();
            if declaration.is_some() || kind == "impl_item" {
                scope = if scope.is_empty() {
                    label.clone()
                } else {
                    format!("{scope}::{label}")
                };
                if scope.len() > MAX_SCOPE_BYTES
                    || scope.match_indices("::").count() >= MAX_DECLARATION_DEPTH
                {
                    bail!("declaration scope safety budget exceeded");
                }
                if entity == "function" {
                    owner = key;
                }
            }
        }
        if matches!(kind, "call" | "call_expression" | "new_expression")
            && let Some(function) = node
                .child_by_field_name("function")
                .or_else(|| node.child_by_field_name("constructor"))
        {
            out.calls.push(Call {
                owner: owner.clone(),
                scope: scope.clone(),
                spelling: text(function, source).to_owned(),
                span: span(node, path),
            });
        }
        let params = in_parameters || matches!(kind, "parameters" | "formal_parameters");
        if matches!(
            kind,
            "assignment"
                | "augmented_assignment"
                | "named_expression"
                | "for_statement"
                | "let_declaration"
                | "variable_declarator"
        ) && let Some(target) = node
            .child_by_field_name("left")
            .or_else(|| node.child_by_field_name("pattern"))
            .or_else(|| node.child_by_field_name("name"))
        {
            let mut targets = vec![target];
            while let Some(target) = targets.pop() {
                if target.kind() == "identifier" {
                    out.rebindings
                        .entry(scope.clone())
                        .or_default()
                        .insert(text(target, source).into());
                } else if matches!(
                    target.kind(),
                    "pattern_list" | "tuple_pattern" | "list_pattern" | "tuple" | "list"
                ) {
                    targets.extend(named_children(target));
                }
            }
        }
        if matches!(
            kind,
            "assignment" | "let_declaration" | "variable_declarator"
        ) {
            let left = node
                .child_by_field_name("left")
                .or_else(|| node.child_by_field_name("pattern"))
                .or_else(|| node.child_by_field_name("name"));
            let right = node
                .child_by_field_name("right")
                .or_else(|| node.child_by_field_name("value"));
            if !right.is_some_and(|r| {
                matches!(
                    r.kind(),
                    "arrow_function" | "function_expression" | "generator_function"
                )
            }) && let Some(left) = left.filter(|n| n.kind() == "identifier")
            {
                out.shadows
                    .entry(scope.clone())
                    .or_default()
                    .insert(text(left, source).into());
            }
        }
        if params && matches!(kind, "identifier" | "shorthand_property_identifier_pattern") {
            // A parameter shadows any same-named global declaration/import.
            out.shadows
                .entry(scope.clone())
                .or_default()
                .insert(text(node, source).into());
        }
        if matches!(kind, "identifier" | "type_identifier") && !owner.is_empty() && !params {
            let is_member_name = node.parent().is_some_and(|p| {
                (p.kind() == "attribute" && p.child_by_field_name("attribute") == Some(node))
                    || (p.kind() == "keyword_argument"
                        && p.child_by_field_name("name") == Some(node))
            });
            if !is_member_name {
                out.references.push(Reference {
                    owner: owner.clone(),
                    scope: scope.clone(),
                    name: text(node, source).into(),
                    span: span(node, path),
                });
            }
        }
        let mut cursor = node.walk();
        let children: Vec<_> = node.named_children(&mut cursor).collect();
        for child in children.into_iter().rev() {
            stack.push((child, parent.clone(), scope.clone(), owner.clone(), params));
        }
    }
    Ok(out)
}

fn named_children(node: Syntax<'_>) -> Vec<Syntax<'_>> {
    let mut c = node.walk();
    node.named_children(&mut c).collect()
}
fn python_receiver_tags(node: Syntax<'_>, source: &str) -> Vec<String> {
    let Some(parameters) = node.child_by_field_name("parameters") else {
        return vec![];
    };
    let Some(first) = named_children(parameters).first().copied() else {
        return vec![];
    };
    if !matches!(
        first.kind(),
        "identifier" | "typed_parameter" | "default_parameter" | "typed_default_parameter"
    ) {
        return vec![];
    }
    let name = if first.kind() == "identifier" {
        Some(first)
    } else {
        first
            .child_by_field_name("name")
            .filter(|n| n.kind() == "identifier")
            .or_else(|| {
                named_children(first).into_iter().find(|n| {
                    n.kind() == "identifier" && Some(*n) != first.child_by_field_name("type")
                })
            })
    };
    let Some(name) = name
        .map(|n| text(n, source))
        .filter(|name| matches!(*name, "self" | "cls" | "this"))
    else {
        return vec![];
    };
    let mut tags = vec![format!("receiver_parameter:{name}")];
    if let Some(parent) = node.parent().filter(|p| p.kind() == "decorated_definition") {
        let decorators: Vec<_> = named_children(parent)
            .into_iter()
            .filter(|n| n.kind() == "decorator")
            .collect();
        if decorators.len() != 1 {
            return vec![];
        }
        let decorator = text(decorators[0], source).trim().trim_start_matches('@');
        if !matches!(decorator, "classmethod" | "property") {
            return vec![];
        }
        tags.push(format!("receiver_decorator:{decorator}"));
    }
    tags
}
fn type_name(node: Syntax<'_>, source: &str) -> String {
    let value = if node.kind() == "generic_type" {
        node.child_by_field_name("type")
            .or_else(|| node.child_by_field_name("name"))
            .unwrap_or(node)
    } else {
        node
    };
    compact(text(value, source))
}
fn clean_string(value: &str) -> String {
    value.trim_matches(['\'', '"', '`']).to_owned()
}
fn extract_imports(
    node: Syntax<'_>,
    source: &str,
    path: &str,
    scope: &str,
    lang: &str,
    imports: &mut Vec<Import>,
) {
    let at = span(node, path);
    if lang == "python" {
        if node.kind() == "import_from_statement" {
            let module = node
                .child_by_field_name("module_name")
                .map(|n| text(n, source).to_owned())
                .unwrap_or_default();
            let mut bindings = vec![];
            for child in named_children(node) {
                if Some(child) == node.child_by_field_name("module_name") {
                    continue;
                }
                match child.kind() {
                    "dotted_name" => {
                        let name = text(child, source).to_owned();
                        bindings.push((name.clone(), name));
                    }
                    "aliased_import" => {
                        if let Some(name) = child.child_by_field_name("name") {
                            let original = text(name, source).to_owned();
                            let alias = child
                                .child_by_field_name("alias")
                                .map(|n| text(n, source).to_owned())
                                .unwrap_or_else(|| original.clone());
                            bindings.push((alias, original));
                        }
                    }
                    _ => {}
                }
            }
            imports.push(Import {
                module,
                bindings,
                scope: scope.into(),
                span: at,
            });
        } else if node.kind() == "import_statement" {
            for child in named_children(node) {
                let name = child.child_by_field_name("name").unwrap_or(child);
                let module = text(name, source).to_owned();
                let alias = child
                    .child_by_field_name("alias")
                    .map(|n| text(n, source).to_owned())
                    .unwrap_or_else(|| module.split('.').next().unwrap_or("").into());
                imports.push(Import {
                    module,
                    bindings: vec![(alias, "*".into())],
                    scope: scope.into(),
                    span: at.clone(),
                });
            }
        }
    } else if lang == "rust" {
        if node.kind() == "mod_item" {
            if node.child_by_field_name("body").is_none()
                && let Some(name) = node.child_by_field_name("name")
            {
                imports.push(Import {
                    module: text(name, source).into(),
                    bindings: vec![(text(name, source).into(), "*".into())],
                    scope: scope.into(),
                    span: at,
                });
            }
        } else {
            // Parse the use tree; grouped imports retain the prefix on each leaf.
            if let Some(argument) = node.child_by_field_name("argument") {
                rust_use(argument, "", source, scope, &at, imports);
            }
        }
    } else if node.kind() == "import_statement" {
        let module = node
            .child_by_field_name("source")
            .map(|n| clean_string(text(n, source)))
            .unwrap_or_default();
        let mut bindings = vec![];
        let mut todo = named_children(node);
        while let Some(child) = todo.pop() {
            match child.kind() {
                "import_specifier" => {
                    if let Some(name) = child.child_by_field_name("name") {
                        let original = text(name, source).to_owned();
                        let alias = child
                            .child_by_field_name("alias")
                            .map(|n| text(n, source).to_owned())
                            .unwrap_or_else(|| original.clone());
                        bindings.push((alias, original));
                    }
                }
                "namespace_import" => {
                    if let Some(name) = named_children(child).last() {
                        bindings.push((text(*name, source).into(), "*".into()));
                    }
                }
                "identifier" if child.parent().is_some_and(|p| p.kind() == "import_clause") => {
                    bindings.push((text(child, source).into(), "default".into()))
                }
                _ => todo.extend(named_children(child)),
            }
        }
        imports.push(Import {
            module,
            bindings,
            scope: scope.into(),
            span: at,
        });
    }
}
fn rust_use(
    node: Syntax<'_>,
    prefix: &str,
    source: &str,
    scope: &str,
    at: &SourceSpan,
    out: &mut Vec<Import>,
) {
    match node.kind() {
        "scoped_use_list" => {
            let path = node
                .child_by_field_name("path")
                .map(|n| text(n, source))
                .unwrap_or("");
            let next = if prefix.is_empty() {
                path.to_owned()
            } else {
                format!("{prefix}::{path}")
            };
            if let Some(list) = node.child_by_field_name("list") {
                for child in named_children(list) {
                    rust_use(child, &next, source, scope, at, out);
                }
            }
        }
        "use_list" => {
            for child in named_children(node) {
                rust_use(child, prefix, source, scope, at, out);
            }
        }
        _ => {
            let raw = node
                .child_by_field_name("path")
                .map(|n| text(n, source))
                .unwrap_or_else(|| text(node, source));
            let full = if prefix.is_empty() {
                raw.to_owned()
            } else {
                format!("{prefix}::{raw}")
            };
            let mut parts: Vec<_> = full.split("::").collect();
            let name = parts.pop().unwrap_or("");
            let alias = node
                .child_by_field_name("alias")
                .map(|n| text(n, source))
                .unwrap_or(name);
            out.push(Import {
                module: parts.join("::"),
                bindings: vec![(alias.into(), name.into())],
                scope: scope.into(),
                span: at.clone(),
            });
        }
    }
}

fn normalized(path: &Path) -> Option<String> {
    let mut parts = vec![];
    for c in path.components() {
        match c {
            Component::Normal(p) => parts.push(p.to_string_lossy().to_string()),
            Component::ParentDir => {
                parts.pop()?;
            }
            Component::CurDir => {}
            _ => return None,
        }
    }
    Some(parts.join("/"))
}
fn resolve_module(file: &ParsedFile, module: &str, paths: &BTreeSet<String>) -> Option<String> {
    let base = Path::new(&file.path).parent().unwrap_or(Path::new(""));
    let candidate = if file.language == "python" {
        let dots = module.chars().take_while(|c| *c == '.').count();
        let mut parent = if dots == 0 {
            PathBuf::new()
        } else {
            base.to_path_buf()
        };
        for _ in 1..dots {
            if !parent.pop() {
                return None;
            }
        }
        parent.join(module.trim_start_matches('.').replace('.', "/"))
    } else if file.language == "rust" {
        let parts: Vec<_> = module.split("::").collect();
        let mut parent = if parts.first() == Some(&"crate") {
            file.crate_root
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    if file.path.starts_with("src/") {
                        PathBuf::from("src")
                    } else {
                        PathBuf::new()
                    }
                })
        } else {
            let stem = Path::new(&file.path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            if matches!(stem, "lib" | "main" | "mod") {
                base.to_path_buf()
            } else {
                base.join(stem)
            }
        };
        for part in parts {
            match part {
                "crate" | "self" | "" => {}
                "super" => {
                    parent.pop();
                }
                p => parent.push(p),
            }
        }
        parent
    } else {
        if !module.starts_with('.') {
            return None;
        }
        base.join(module)
    };
    let key = normalized(&candidate)?;
    let extensions: &[&str] = if file.language == "python" {
        &["py"]
    } else if file.language == "rust" {
        &["rs"]
    } else {
        &["ts", "tsx", "js", "jsx", "mjs", "cjs"]
    };
    if paths.contains(&key) {
        return Some(key);
    }
    let stem = if file.language != "python" && file.language != "rust" {
        Path::new(&key)
            .with_extension("")
            .to_string_lossy()
            .to_string()
    } else {
        key.clone()
    };
    let mut matches = BTreeSet::new();
    for ext in extensions {
        for candidate in [
            format!("{stem}.{ext}"),
            format!("{key}/index.{ext}"),
            if file.language == "python" {
                format!("{key}/__init__.py")
            } else {
                format!("{key}/mod.{ext}")
            },
        ] {
            if paths.contains(&candidate) {
                matches.insert(candidate);
            }
        }
    }
    if matches.len() == 1 {
        matches.into_iter().next()
    } else {
        None
    }
}

type BindingKey<'a> = (&'a str, &'a str, &'a str);
type ImportBindings<'a> = BTreeMap<BindingKey<'a>, Vec<(&'a Import, &'a str)>>;
struct Resolver<'a> {
    declarations: BTreeMap<(&'a str, &'a str, &'a str), Vec<&'a Fact>>,
    imports: ImportBindings<'a>,
    class_scopes: BTreeSet<(String, String)>,
    paths: BTreeSet<String>,
}
impl<'a> Resolver<'a> {
    fn new(files: &'a [ParsedFile]) -> Self {
        let mut resolver = Self {
            declarations: BTreeMap::new(),
            imports: BTreeMap::new(),
            class_scopes: BTreeSet::new(),
            paths: files.iter().map(|f| f.path.clone()).collect(),
        };
        for file in files {
            for fact in &file.facts {
                if matches!(fact.kind.as_str(), "function" | "class") {
                    resolver
                        .declarations
                        .entry((&file.path, &fact.scope, &fact.label))
                        .or_default()
                        .push(fact);
                    if fact.tags.iter().any(|t| t == "export-default") && fact.label != "default" {
                        resolver
                            .declarations
                            .entry((&file.path, &fact.scope, "default"))
                            .or_default()
                            .push(fact);
                    }
                }
                if fact.kind == "class" || fact.tags.iter().any(|t| t == "syntax:impl_item") {
                    let scope = if fact.scope.is_empty() {
                        fact.label.clone()
                    } else {
                        format!("{}::{}", fact.scope, fact.label)
                    };
                    resolver.class_scopes.insert((file.path.clone(), scope));
                }
            }
            for import in &file.imports {
                for (local, imported) in &import.bindings {
                    resolver
                        .imports
                        .entry((&file.path, &import.scope, local))
                        .or_default()
                        .push((import, imported));
                }
            }
        }
        resolver
    }
    fn declaration(&self, path: &str, scope: &str, name: &str, exported: bool) -> Option<&'a Fact> {
        let declarations = self.declarations.get(&(path, scope, name))?;
        if declarations.len() != 1 {
            return None;
        }
        let result = declarations[0];
        if exported && !result.tags.iter().any(|t| t == "exported") {
            return None;
        }
        Some(result)
    }
    fn resolve(&self, file: &ParsedFile, scope: &str, name: &str) -> Option<(String, String)> {
        if name
            .chars()
            .any(|c| !c.is_alphanumeric() && !matches!(c, '_' | '.' | ':'))
        {
            return None;
        }
        let parts: Vec<_> = name.split(['.', ':']).filter(|x| !x.is_empty()).collect();
        let first = *parts.first()?;
        let receiver_syntax = match file.language.as_str() {
            "python" => matches!(first, "self" | "cls" | "this") && name.contains('.'),
            "javascript" | "typescript" | "tsx" => first == "this" && name.starts_with("this."),
            "rust" => {
                (first == "self" && name.starts_with("self."))
                    || (first == "Self" && name.starts_with("Self::"))
            }
            _ => false,
        };
        if parts.len() == 2 && receiver_syntax {
            let class_scope = scope.rsplit_once("::").map(|(s, _)| s)?;
            if self
                .class_scopes
                .contains(&(file.path.clone(), class_scope.to_owned()))
            {
                let method = scope.rsplit_once("::")?.1;
                let declaration = self.declaration(&file.path, class_scope, method, false)?;
                if file.language == "python" {
                    if !declaration
                        .tags
                        .contains(&format!("receiver_parameter:{first}"))
                        || file
                            .rebindings
                            .get(scope)
                            .is_some_and(|names| names.contains(first))
                        || self
                            .imports
                            .contains_key(&(file.path.as_str(), scope, first))
                    {
                        return None;
                    }
                    for decorator in declaration
                        .tags
                        .iter()
                        .filter_map(|tag| tag.strip_prefix("receiver_decorator:"))
                    {
                        if ["", class_scope].into_iter().any(|scope| {
                            file.shadows
                                .get(scope)
                                .is_some_and(|names| names.contains(decorator))
                                || self.declarations.contains_key(&(
                                    file.path.as_str(),
                                    scope,
                                    decorator,
                                ))
                                || self.imports.contains_key(&(
                                    file.path.as_str(),
                                    scope,
                                    decorator,
                                ))
                        }) {
                            return None;
                        }
                    }
                } else if matches!(file.language.as_str(), "javascript" | "typescript" | "tsx")
                    && !declaration
                        .tags
                        .contains(&"syntax:method_definition".into())
                    && !declaration.tags.contains(&"syntax:arrow_function".into())
                {
                    return None;
                }
                let target = self.declaration(&file.path, class_scope, parts[1], false)?;
                return Some((file.path.clone(), target.key.clone()));
            }
        }
        let mut current = scope.to_owned();
        loop {
            // Method bodies do not lexically inherit class/impl member bindings.
            if !self
                .class_scopes
                .contains(&(file.path.clone(), current.clone()))
            {
                if file
                    .shadows
                    .get(&current)
                    .is_some_and(|s| s.contains(first))
                {
                    return None;
                }
                if parts.len() == 1
                    && let Some(declarations) =
                        self.declarations
                            .get(&(file.path.as_str(), current.as_str(), first))
                {
                    if declarations.len() != 1 {
                        return None;
                    }
                    return Some((file.path.clone(), declarations[0].key.clone()));
                }
                if let Some(bindings) =
                    self.imports
                        .get(&(file.path.as_str(), current.as_str(), first))
                {
                    if bindings.len() != 1 {
                        return None;
                    }
                    let (import, imported) = bindings[0];
                    let target = resolve_module(file, &import.module, &self.paths)?;
                    let requested = if imported == "*" {
                        if parts.len() != 2 {
                            return None;
                        }
                        parts[1]
                    } else {
                        if parts.len() != 1 {
                            return None;
                        }
                        imported
                    };
                    let declaration = self.declaration(
                        &target,
                        "",
                        requested,
                        !matches!(file.language.as_str(), "python" | "rust"),
                    )?;
                    return Some((target, declaration.key.clone()));
                }
            }
            if current.is_empty() {
                break;
            }
            current = current
                .rsplit_once("::")
                .map(|(s, _)| s.to_owned())
                .unwrap_or_default();
        }
        if file.language == "rust"
            && parts.len() >= 2
            && matches!(parts[0], "crate" | "self" | "super")
        {
            let module = parts[..parts.len() - 1].join("::");
            let target = resolve_module(file, &module, &self.paths)?;
            let declaration = self.declaration(&target, "", parts.last()?, false)?;
            return Some((target, declaration.key.clone()));
        }
        None
    }
}

fn build_graph(graph: &mut Graph, files: &[ParsedFile], options: &AnalyzeOptions) -> Result<()> {
    let mut ids = BTreeMap::new();
    let mut file_ids = BTreeMap::new();
    let resolver = Resolver::new(files);
    let paths = &resolver.paths;
    for file in files {
        if cancelled(options) {
            bail!("analysis cancelled");
        }
        let file_id = stable_id(&[&graph.repository, "file", &file.path]);
        graph.add_node(new_node(
            graph,
            NodeDraft {
                id: file_id.clone(),
                label: file.path.clone(),
                kind: NodeKind::File,
                level: 4,
                source: Some(file.file_span.clone()),
                summary: format!("{} source file", file.language),
                hash: file.source_hash.clone(),
                language: Some(file.language.clone()),
                tags: vec![],
            },
        ));
        file_ids.insert(file.path.clone(), file_id.clone());
        for fact in &file.facts {
            let id = stable_id(&[&graph.repository, &file.path, &fact.key]);
            ids.insert((file.path.clone(), fact.key.clone()), id.clone());
            graph.add_node(new_node(
                graph,
                NodeDraft {
                    id,
                    label: fact.label.clone(),
                    kind: node_kind(&fact.kind),
                    level: if matches!(fact.kind.as_str(), "statement" | "expression") {
                        6
                    } else {
                        5
                    },
                    source: Some(fact.span.clone()),
                    summary: fact.summary.clone(),
                    hash: fact.body.clone(),
                    language: Some(file.language.clone()),
                    tags: fact.tags.clone(),
                },
            ));
        }
    }
    for file in files {
        if cancelled(options) {
            bail!("analysis cancelled");
        }
        let file_id = &file_ids[&file.path];
        for fact in &file.facts {
            let id = &ids[&(file.path.clone(), fact.key.clone())];
            let parent = ids
                .get(&(file.path.clone(), fact.parent.clone()))
                .unwrap_or(file_id);
            graph.add_edge(
                parent,
                id,
                Relation::Contains,
                Evidence::observed("syntax containment", vec![fact.span.clone()]),
            );
        }
        for import in &file.imports {
            if let Some(target) =
                resolve_module(file, &import.module, paths).and_then(|p| file_ids.get(&p))
            {
                graph.add_edge(
                    file_id,
                    target,
                    Relation::Imports,
                    Evidence::observed("local module import", vec![import.span.clone()]),
                );
            } else {
                graph.diagnostics.push(Diagnostic {
                    code: "unresolved_import".into(),
                    message: format!(
                        "Import {} is external, ambiguous, or unsupported",
                        compact(&import.module)
                    ),
                    path: Some(file.path.clone()),
                });
            }
        }
        for call in &file.calls {
            if cancelled(options) {
                bail!("analysis cancelled");
            }
            let caller = ids
                .get(&(file.path.clone(), call.owner.clone()))
                .unwrap_or(file_id);
            let target = if file.diagnostics.iter().any(|d| d.code == "syntax_error") {
                None
            } else {
                resolver.resolve(file, &call.scope, &call.spelling)
            };
            if let Some(target) = target.and_then(|key| ids.get(&key)) {
                graph.add_edge(
                    caller,
                    target,
                    Relation::Calls,
                    Evidence::inferred(
                        "lexical/import binding; static target, runtime dispatch not guaranteed",
                        vec![call.span.clone()],
                    ),
                );
            } else {
                graph.diagnostics.push(Diagnostic {
                    code: "unresolved_call".into(),
                    message: format!(
                        "{} at line {} has no proven static target",
                        compact(&call.spelling),
                        call.span.start_line
                    ),
                    path: Some(file.path.clone()),
                });
            }
        }
        for reference in &file.references {
            if cancelled(options) {
                bail!("analysis cancelled");
            }
            if let Some(target) = resolver
                .resolve(file, &reference.scope, &reference.name)
                .and_then(|key| ids.get(&key))
                && let Some(owner) = ids.get(&(file.path.clone(), reference.owner.clone()))
                && owner != target
            {
                graph.add_edge(
                    owner,
                    target,
                    Relation::References,
                    Evidence::inferred("lexical symbol reference", vec![reference.span.clone()]),
                );
            }
        }
        for relationship in &file.type_relations {
            let source = if relationship.source_key.is_empty() {
                resolver
                    .resolve(file, &relationship.scope, &relationship.source_name)
                    .and_then(|key| ids.get(&key))
            } else {
                ids.get(&(file.path.clone(), relationship.source_key.clone()))
            };
            let target = resolver
                .resolve(file, &relationship.scope, &relationship.target_name)
                .and_then(|key| ids.get(&key));
            if let (Some(source), Some(target)) = (source, target)
                && graph.nodes[source].kind == NodeKind::Class
                && graph.nodes[target].kind == NodeKind::Class
            {
                graph.add_edge(
                    source,
                    target,
                    if relationship.implements {
                        Relation::Implements
                    } else {
                        Relation::References
                    },
                    Evidence::observed(
                        if relationship.implements {
                            "declared implementation; lexical type binding"
                        } else {
                            "declared base type; lexical type binding"
                        },
                        vec![relationship.span.clone()],
                    ),
                );
            } else {
                graph.diagnostics.push(Diagnostic {
                    code: "unresolved_type_relation".into(),
                    message: format!(
                        "Declared type relation to {} has no unambiguous local target",
                        compact(&relationship.target_name)
                    ),
                    path: Some(file.path.clone()),
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
