# Deterministic analysis pipeline

This describes the implementation in `glyphibe-parser` and `glyphibe-analysis`,
not a claim that all release gates have passed. The same libraries are called by
the CLI and desktop adapter. MCP reads the resulting repository-scoped index.

## Stages and ownership

1. **Bind a repository and external cache.** Canonicalize the selected directory,
   derive a hashed local repository identity, and read Git's HEAD when available.
   SQLite is bound to that canonical repository. An index inside the analyzed
   directory is rejected. A plain directory without Git is supported.
2. **Discover regular source files.** The `ignore` walker respects applicable
   `.gitignore`/Git exclusion rules without requiring a Git checkout. It does not
   follow symlinks. Dependency, build and cache directories such as `node_modules`,
   `target`, `.venv`, `dist` and `.git` are excluded. Paths are sorted for repeatable
   traversal. The focused grammars cover Python, Rust, JavaScript/JSX and
   TypeScript/TSX.
3. **Read and reuse parsing facts.** Check source containment, regular-file type,
   UTF-8 and byte limits. A cache key combines a parser-format version, exact
   content hash and applicable conventional Rust crate-root metadata. Unchanged
   files reuse serialized extraction facts. No syntax trees are serialized or
   executable objects deserialized.
4. **Parse once per changed file.** A native Tree-sitter parser extracts
   declarations, scopes, imports, call sites, structural references, class/trait
   relations, control blocks, selected statements and expressions. Iterative
   traversal avoids Rust recursion on deeply nested syntax. A separate bottom-up
   syntax digest pass computes subtree fingerprints without retaining overlapping
   copies of complete function/block source text.
5. **Resolve against the complete current declaration set.** Indexed lexical
   declarations and import bindings resolve supported local targets. Relationships
   are rebuilt even when a caller's own parse result was cached: changing or
   deleting a callee can invalidate an unchanged caller. Bare names do not become
   global cross-file guesses. Parameters and recorded assignments can shadow
   declarations/imports. Class/impl members are not inherited as bare lexical
   names inside method bodies.
6. **Reconcile identities.** Compare the fresh graph with the previous snapshot.
   Exact identities are retained. A missing/new pair can match by a unique
   structural fingerprint with compatible entity kind and language. Ambiguous
   candidates remain explicitly ambiguous. Removed nodes and invalidated edges
   remain identifiable as removed/stale history. Deleted parse-cache entries are
   pruned after a successful scan.
7. **Derive structure.** Iterative Kosaraju traversal identifies cyclic dependency
   components, including self recursion. Deterministic multilevel Louvain groups
   current symbols by structural connectivity. Directory-derived components are
   stored separately. A repository scope is an L1 entry point; it does not assert
   a discovered business domain.
8. **Map explanations and knowledge.** `glyphibe-explain` adds canonical
   explanations, direct scope coverage and concept associations. These mappings
   are derived templates with explicit provenance. A final validation precedes
   persistence of the complete enriched snapshot.

## Call and type evidence

Resolved calls are **Inferred** static targets with a note that runtime dispatch
is not guaranteed. Dynamic callbacks, computed receivers, reflection, complex
generics and unavailable targets remain unresolved diagnostics. Syntax errors
are reported; calls in a malformed file are conservatively left unresolved.

Local module import edges are **Observed** syntax with a resolved local file.
External or ambiguous imports are diagnostics. Explicit Rust trait
implementations and TypeScript `implements` clauses link known class/trait
declarations through `implements`. Python/JavaScript/TypeScript base-type clauses
use `references` with a declared-base method. Dynamic base expressions do not
create a guessed edge. These are syntactic facts, not compiler or runtime tests.

Supported resolution includes Python local/relative imports and aliases,
JavaScript/TypeScript relative named/default/namespace imports, and basic Rust
module/import paths. Conventional Rust `src` roots are associated with the
nearest ancestor `Cargo.toml` inside the scan boundary; no Cargo command or
manifest script is run. Multiple packages can contain same-named modules without
sharing a guessed crate root.

Current limitations include wildcard and re-export expansion, custom Cargo
source roots, TypeScript path aliases, package-manager resolution, full
block-sensitive data flow, overload/type inference and general dynamic dispatch.
There is no active LSP enrichment implementation. Unsupported cases must be
visible rather than treated as absent runtime behavior.

## Communities and components

The Louvain implementation is original Rust code based on the published
[Blondel et al. algorithm](https://arxiv.org/abs/0803.0476v2). It performs local
modularity-improving moves followed by weighted graph aggregation. Sorted node
order and deterministic tie handling replace randomized traversal. Resolution
is explicit. The projection weights calls/imports/dependencies at 1 and
structural references at 0.25 when both endpoints are in the projection.

The projection uses function nodes when any exist, otherwise files. Isolated
nodes remain separate communities. The current strategy fully reclusters the
current graph after relationship reconstruction; it does not claim incremental
Louvain optimization or stable communities after arbitrary edits. Community IDs
include membership and resolution. Names remain `Unnamed cluster N`.

Components are explicitly labeled `Directory …`, with directory membership as
evidence. This is an organizational boundary, not proof of a business component.
Cluster nodes retain at most 64 sampled source spans; complete member provenance
is available on individual membership edges. This avoids quadratic replication
of all member source spans on every edge.

## Resource and execution boundaries

Defaults are 20,000 focused source files, 2 MiB per file, 256 MiB cumulative
source bytes, 250,000 extraction items and a 150,000-node syntax budget per file.
Declaration nesting is limited to 256 scopes and a qualified scope to 16 KiB.
The extraction budget counts nodes plus auxiliary calls/references, so the final
architecture-node count may be below that limit. Exceeding a global budget fails
explicitly. Oversized/non-UTF-8 files are skipped with diagnostics. Extraction is
currently single-threaded, bounding concurrent parser/file memory. Cancellation
is checked during discovery, syntax processing and relation construction.
Actual reads use a `take(limit + 1)` bounded reader with cancellation checks;
concurrent file growth cannot cause an unbounded `read_to_end` allocation before
the configured per-file/cumulative limit is checked.

Progress reports discovered totals and completed files through a callback; CLI
progress goes to stderr through best-effort writes. Shared libraries do not print
to stdout/stderr, so a detached GUI or closed progress pipe cannot panic source
analysis. Parameter and graph-mode changes use the loaded graph
and do not invoke this pipeline. Project code, package scripts, tests, dependency
installers and remote knowledge scripts are never run by source analysis.

The SQLite snapshot is currently serialized graph JSON plus per-file extraction
JSON. Queries load the complete graph; this is not yet a row-level, streaming
graph-query engine. Hot analysis still reads/hashes source and rebuilds relations,
clusters and mappings. Process startup, reconciliation, serialization and SQLite
writes can dominate small parsing gains. See [measured results](../benchmarks/analysis.md).
