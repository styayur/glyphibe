# Shared implementation contract

Status: pre-implementation contract, 2026-10-27. This is not a feature claim.

All interfaces use one serializable Rust graph model in `glyphibe-core`. UI,
CLI, and MCP consume the same graph and query functions. Repository source is
untrusted input; analysis never executes project code or installs dependencies.

## Data model

`Graph { schema_version: u32, repository: String, revision: String,
nodes: BTreeMap<String, Node>, edges: Vec<Edge>, diagnostics: Vec<Diagnostic>,
stats: ScanStats }`.

`Node { id, label, kind, level: u8 (1..6), plane, repository, revision,
language: Option<String>, source: Option<SourceSpan>, summary, evidence,
fingerprint, tags: Vec<String>, knowledge: Vec<String>, change }`.

`SourceSpan { path: String (repository-relative, forward slashes), start_line,
start_column, end_line, end_column, start_byte, end_byte }`. Lines and columns
are one-based. Bytes use UTF-8 offsets. `Plane` = `Architecture | Explanation`.
`NodeKind` = `Domain | Cluster | Component | File | Class | Function | Block |
Statement | Expression | Concept | Explanation`.

`Evidence { kind: EvidenceKind, method: String, sources: Vec<SourceSpan>,
note: String }`. `EvidenceKind` = `Observed | Inferred | Proposed | Verified`.
`ChangeStatus` = `Unchanged | Added | Modified | Removed | Ambiguous`.

`Edge { id, source, target, relation, evidence, stale: bool }`.
`Relation` = `Contains | Calls | Imports | References | DependsOn | Implements |
MapsTo | HasConcept | VerifiedBy`. JSON enum names use snake_case.

`Diagnostic { code, message, path: Option<String> }`.
`ScanStats { discovered_files, parsed_files, cached_files, skipped_files,
source_bytes, elapsed_ms }` (integer fields).

`Graph::new(repository, revision)`, `Graph::add_node(Node)`,
`Graph::add_edge(source, target, Relation, Evidence)`, `Graph::validate()`;
`stable_id(parts: &[&str]) -> String`, `fingerprint(text: &str) -> String`;
`Evidence::observed(method, sources)` / `Evidence::inferred(method, sources)`.

Graph query APIs are owned by core. Storage is a local SQLite cache owned by
core: `IndexStore::open(path)`, `load_graph()`, `save_graph(&Graph)`,
`load_file(path, hash) -> Option<String>`, `save_file(path, hash, json)`.
Caches live outside scanned projects and are bound to a canonical repository.

## Crate ownership and interfaces

- `glyphibe-parser`: Tree-sitter extraction, bounded discovery, cache-aware
  scanning. `analyze(root: &Path, options: &AnalyzeOptions) -> Result<Graph>`;
  `AnalyzeOptions { cache: Option<PathBuf>, max_file_bytes: u64,
  max_files: usize, cancel: Option<Arc<AtomicBool>> }` with `Default`.
  Progress goes to stderr or a callback, never corrupts JSON stdout.
- `glyphibe-analysis`: `enrich(graph: &mut Graph, resolution: f64)`;
  deterministic Louvain structural clusters; components keep their own
  evidence. No business labels invented from graph connectivity.
- `glyphibe-knowledge`: original bilingual concept registry, prerequisite
  learning graph, safe GitHub URL import with license/version provenance.
- `glyphibe-explain`: `explain(graph: &Graph, id: &str, depth: u8,
  language: &str) -> Result<Explanation>` and `map_explanations(&mut Graph)`.
  `Explanation` serializes title, facts, uncertainty, concepts, lesson steps.
- `glyphibe-cli`: all command routing, stdio MCP via standard SDK.
- `glyphibe-mcp`: SDK server adapter sharing core queries and proposal store.
- `glyphibe-desktop`: Tauri 2 adapter. Only registered analysis, query and
  native-dialog commands; no arbitrary shell or unrestricted filesystem IPC.
- `glyphibe-ui`: React/TypeScript graph canvas, same graph JSON, shared mode
  state, all graph modes using the same loaded data.

## Verification rules

Record exact commands and actual results. Failed, pending, and unmeasured
checks remain visible. No Stable tag or release before every required gate
has independent evidence. Dependency and knowledge license inventories are
generated from the resolved dependency graphs, not guessed from package names.
