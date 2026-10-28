# ADR 0002: one local graph and conservative analysis

Date: 2026-10-27. Status: accepted.

Use a modular Rust workspace with core/parser/analysis/knowledge/explain/CLI/
MCP boundaries. Tauri links these libraries directly; there is no additional
daemon, container, database service or duplicate GUI indexer. Store a versioned
JSON graph snapshot and per-file syntax extraction in SQLite. Non-executable
JSON avoids deserialization mechanisms that execute code. SQLite is local,
transactional and requires no user-managed service. Cache paths are external
to repositories and bound to the canonical root.

The attributed graph supports general contains edges and arbitrary many-to-many
cross-plane mappings at six shared levels. Level is a view dimension, not an
enforced parent tree. Observed, inferred, proposed and verified provenance
remain distinct. Agent proposals live in a separate table and do not enter
fact or impact queries automatically.

Use Tree-sitter's official Rust bindings and permissively licensed Python,
Rust, JavaScript and TypeScript/TSX grammars. Reuse syntax trees per file,
cache by content and parser version, and re-resolve relationships globally.
Conservative lexical/import resolution favors precision; reflection and
unproven receiver types remain unresolved. Optional LSP is an extension point,
not a requirement or an enabled default.

Implement deterministic Louvain from its published modularity definition,
with an explicit resolution parameter and ordered processing. Full derived
reclustering after source updates is initially the documented strategy.
Separate directory/module-backed components from statistical communities.

Use Sigma with Graphology for 2D, and react-force-graph/Three.js for 3D. Do not
add React Flow or Excalidraw as redundant renderers. A single React state owns
mode, repository, selection, level, depth, density, filters and impact options.
Native Mica and CSS panel glass have opaque/reduced-effect fallbacks.

Alternatives considered: multiple existing analysis stacks (duplicate facts,
licensing and runtime burden); a strict hierarchy (cannot express many-to-many
mapping); hosted semantic generation (violates offline deterministic core);
DOM cards for all symbols (poor high-density graph fit). The source audit is
recorded in [competitive analysis](../research/competitive-analysis.md).
