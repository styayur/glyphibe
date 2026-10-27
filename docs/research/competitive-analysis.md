# Competitive analysis and source audit

Research date: **2026-10-27, Asia/Singapore**. Scope: current public source,
documentation, manifests and per-project licenses of all twelve requested
references. The linked reports record exact commits and source permalinks.
This audit did not execute the reference products or benchmark them.

| Area | Inspected projects | Source report |
| --- | --- | --- |
| Indexing and analysis | GitNexus, CodeBoarding, Codesight, Understand Anything | [Analysis engines](analysis-engines.md) |
| Graphs and interaction | GitDiagram, Sigma.js, React Flow, react-force-graph, Excalidraw | [Graph engines](graph-engines.md) |
| Desktop and glass | Kando, Tauri, window-vibrancy | [Desktop platform](desktop-platform.md) |

## Engineering decisions

1. Use one original Rust analyzer. Tree-sitter syntax extraction is separated
   from conservative symbol resolution and derived community detection. A
   function spelling alone cannot establish a cross-file or cross-language
   call. Dynamic and ambiguous calls remain explicit diagnostics.
2. Persist a shared attributed graph and non-executable per-file cache outside
   scanned roots. Changed files invalidate derived relationships, including
   inbound edges from unchanged callers. Do not execute repository scripts.
3. Separate structural communities from business components. Seeded or ordered
   graph algorithms provide repeatable partitions; names require independent
   directory, symbol, documentation or user evidence. A community is not a
   discovered business domain.
4. Use Sigma/Graphology for two-dimensional graph rendering and one Three.js
   graph adapter for Galaxy. Mode switches reuse data and selection. React
   Flow and Excalidraw are useful interaction references; their large DOM or
   whiteboard object models are unnecessary for the v1 shared graph canvas.
5. Use Tauri 2 for a constrained local host, native Mica when available, and
   opaque fallback. Graph rendering measurements must be collected on the
   actual app; upstream engine choices are not performance evidence.
6. Use official rmcp for stdio MCP. Protocol compatibility is validated with
   a real SDK client; JSON-RPC message shapes are not invented locally.
7. Keep the core free of LLM and cloud dependencies. Explain syntax, source
   evidence and original concept knowledge through bilingual templates;
   explicitly disclose missing semantic evidence.

## Reuse boundary and limits

No implementation, grammar query, artwork, fixture, documentation fragment or
code snippet from the twelve references has been copied. GitNexus's PolyForm
Noncommercial license makes it unsuitable for source reuse in this project.
Kando has per-file licensing and cannot be described as uniformly MIT. A root
license does not certify transitive dependencies or knowledge redistribution.
Resolved dependency inventories and per-entry knowledge provenance remain
release gates.

Unmeasured: reference runtime memory, cache size, call precision/recall and
rendering FPS. Optional LSP enrichment requires language-server-specific
workspace safety analysis; it is not silently started on untrusted projects.

See [ADR 0001](../adr/0001-license-and-clean-implementation.md) and
[ADR 0002](../adr/0002-shared-local-graph.md).
