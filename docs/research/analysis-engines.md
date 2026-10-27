# Analysis engine source audit

Research date: **2026-10-27 (Asia/Singapore)**. This is a source inspection, not an execution benchmark or a warranty of third-party correctness. Repository heads below were resolved with the GitHub API; the links pin the inspected revision. No upstream source was copied into Glyphibe. In particular, GitNexus is a restricted-license research reference only.

| Project | Inspected commit | Commit timestamp (UTC) | Manifest version | Root license |
|---|---|---|---|---|
| GitNexus | `3a29876365289ca397e8f7d0fd568a9eeca7b7b5` | 2026-10-27 09:30:26 | 1.6.12 | PolyForm Noncommercial 1.0.0 |
| CodeBoarding | `13adc4a0b793bc472d6b8e583cf824070241c274` | 2026-10-26 23:05:32 | 0.15.3 | MIT |
| Codesight | `6ce2baa932e387928430e9df8a47f802568f6bef` | 2026-04-06 01:43:15 | 0.1.0 | MIT |
| Understand Anything | `4e6d3299017a7432da15957874992837a315c5b4` | 2026-10-26 07:05:22 | core package 0.1.0 | MIT |

## GitNexus

The [pipeline](https://github.com/abhigyanpatwari/GitNexus/blob/3a29876365289ca397e8f7d0fd568a9eeca7b7b5/gitnexus/src/core/ingestion/pipeline.ts) orders scan, structure, parse, cross-file resolution, scope resolution, derived communities and processes. Options separate expensive derived passes from parsing and expose bounded opt-in control/data-flow analysis. Its graph persistence and staged processing are useful design ideas, but are not dependencies of Glyphibe.

The [Python scope adapter](https://github.com/abhigyanpatwari/GitNexus/blob/3a29876365289ca397e8f7d0fd568a9eeca7b7b5/gitnexus/src/core/ingestion/languages/python/scope-resolver.ts) uses language-specific import binding, namespace resolution, C3 method resolution and arity evidence. It also enables explicit fallback heuristics: a discovered edge must therefore retain its derivation rather than being treated as an unconditional runtime fact. Cross-language support needs concrete bridge evidence; sharing a name is insufficient.

The [community processor](https://github.com/abhigyanpatwari/GitNexus/blob/3a29876365289ca397e8f7d0fd568a9eeca7b7b5/gitnexus/src/core/ingestion/community-processor.ts) projects call relationships into seeded Leiden communities. Large-graph filtering excludes designated heuristic edge reasons, and workers have time limits. The [manifest](https://github.com/abhigyanpatwari/GitNexus/blob/3a29876365289ca397e8f7d0fd568a9eeca7b7b5/gitnexus/package.json) declares Tree-sitter grammars, Graphology, LadybugDB and MCP. Embedding infrastructure exists separately; a local structural pipeline need not depend on a hosted LLM. No independent memory, speed or accuracy measurement was made here.

The [license](https://github.com/abhigyanpatwari/GitNexus/blob/3a29876365289ca397e8f7d0fd568a9eeca7b7b5/LICENSE) restricts permitted purposes to noncommercial categories. Glyphibe will **not** copy, translate, vendor or adapt this project's implementation, grammar queries, fixtures, documentation or snippets. Algorithms must be independently implemented from their published general definitions and permissive upstream libraries.

## CodeBoarding

The [call graph builder](https://github.com/CodeBoarding/CodeBoarding/blob/13adc4a0b793bc472d6b8e583cf824070241c274/static_analyzer/engine/call_graph_builder.py) combines LSP document symbols, definitions, references and hierarchy evidence. [LSP handling](https://github.com/CodeBoarding/CodeBoarding/blob/13adc4a0b793bc472d6b8e583cf824070241c274/static_analyzer/engine/lsp_client.py) includes batched definition requests, server-readiness handling and UTF-16 position conversion. This is stronger evidence than global name matching, at the cost of per-language server installation and workspace initialization. Such servers require separate safety review before use with untrusted workspaces.

The [incremental updater](https://github.com/CodeBoarding/CodeBoarding/blob/13adc4a0b793bc472d6b8e583cf824070241c274/static_analyzer/incremental_orchestrator.py) invalidates changed-file facts, resolves fresh callers against retained declarations and re-proves cached inbound edges. This demonstrates why unchanged caller text does not imply unchanged relations. Its [cache](https://github.com/CodeBoarding/CodeBoarding/blob/13adc4a0b793bc472d6b8e583cf824070241c274/static_analyzer/analysis_cache.py) uses versioned SHA-tagged pickle artifacts; Glyphibe should prefer non-executable data serialization and external cache directories.

Current [clustering](https://github.com/CodeBoarding/CodeBoarding/blob/13adc4a0b793bc472d6b8e583cf824070241c274/static_analyzer/clustering/service.py) builds a replayable qualified-name hierarchy over file leaves, with calls and type/inheritance relations as file affinity evidence. It should not be described simply as a Leiden engine: this revision explicitly distinguishes name partitions from communities. [Dependencies](https://github.com/CodeBoarding/CodeBoarding/blob/13adc4a0b793bc472d6b8e583cf824070241c274/pyproject.toml) include Tree-sitter, NetworkX, language servers and multiple LangChain model integrations. Deterministic graph construction is separable conceptually from narrative agents, but Glyphibe will use a single original Rust pipeline rather than embed this stack.

The [root MIT license](https://github.com/CodeBoarding/CodeBoarding/blob/13adc4a0b793bc472d6b8e583cf824070241c274/LICENSE) permits reuse with its notice conditions; transitive dependencies and bundled servers still need independent review. No source reuse is planned. No execution performance or cross-language accuracy claim was verified in this audit.

## Codesight

The [parser](https://github.com/spirituslab/codesight/blob/6ce2baa932e387928430e9df8a47f802568f6bef/src/analyzer/parser.mjs) reuses one native Tree-sitter parser per language and extracts symbols/imports from one syntax tree. [Analysis orchestration](https://github.com/spirituslab/codesight/blob/6ce2baa932e387928430e9df8a47f802568f6bef/src/analyzer/index.mjs) batches work and derives project metadata from text rather than executing manifests. The [manifest](https://github.com/spirituslab/codesight/blob/6ce2baa932e387928430e9df8a47f802568f6bef/package.json) confirms a no-LLM structural stack and MCP SDK.

The [call graph resolver](https://github.com/spirituslab/codesight/blob/6ce2baa932e387928430e9df8a47f802568f6bef/src/analyzer/callgraph.mjs) indexes paths and symbols, prefers local/import evidence and includes exported-name fallbacks marked inferred. These fallbacks can over-connect unrelated code. Glyphibe must preserve unresolved dynamic calls instead of promoting a unique global spelling to fact. [Module grouping](https://github.com/spirituslab/codesight/blob/6ce2baa932e387928430e9df8a47f802568f6bef/src/analyzer/modules.mjs) follows the first directory component and is not community detection or verified business architecture.

The [cache](https://github.com/spirituslab/codesight/blob/6ce2baa932e387928430e9df8a47f802568f6bef/src/analyzer/cache.mjs) hashes file contents, checks tsconfig changes and prunes deleted entries. It stores JSON under the analyzed project; Glyphibe's cache must instead stay outside it. [License](https://github.com/spirituslab/codesight/blob/6ce2baa932e387928430e9df8a47f802568f6bef/LICENSE): MIT, subject to notices and separate dependency review. No engine, UI frame-rate or precision/recall benchmark was run.

## Understand Anything

The [Tree-sitter plugin](https://github.com/Egonex-AI/Understand-Anything/blob/4e6d3299017a7432da15957874992837a315c5b4/understand-anything-plugin/packages/core/src/plugins/tree-sitter-plugin.ts) reuses WASM parsers and offers a combined structure/call extraction pass. It distinguishes unavailable/failed structural evidence from a successfully empty file. Its [manifest](https://github.com/Egonex-AI/Understand-Anything/blob/4e6d3299017a7432da15957874992837a315c5b4/understand-anything-plugin/packages/core/package.json) declares Web Tree-sitter, grammars, search and schema libraries.

The [scanner](https://github.com/Egonex-AI/Understand-Anything/blob/4e6d3299017a7432da15957874992837a315c5b4/understand-anything-plugin/skills/understand/scan-project.mjs) deterministically enumerates and classifies files while explicitly retaining LLM responsibility for narrative project interpretation. The [LLM analyzer](https://github.com/Egonex-AI/Understand-Anything/blob/4e6d3299017a7432da15957874992837a315c5b4/understand-anything-plugin/packages/core/src/analyzer/llm-analyzer.ts) constructs semantic-analysis prompts and parses their outputs. Structural extraction is separable; narrative domains and teaching must not be assumed deterministic just because parsing is.

The [schema](https://github.com/Egonex-AI/Understand-Anything/blob/4e6d3299017a7432da15957874992837a315c5b4/understand-anything-plugin/packages/core/src/schema.ts) models many structural, semantic and knowledge relationships and sanitizes model-generated aliases. Glyphibe needs a stronger independent distinction between observed, inferred, proposed and verified evidence. The [incremental preparation](https://github.com/Egonex-AI/Understand-Anything/blob/4e6d3299017a7432da15957874992837a315c5b4/understand-anything-plugin/skills/understand/prepare-incremental.mjs) uses fingerprints, Git diffs, symbol baselines and parameterized Git arguments with NUL-delimited paths.

The [root license](https://github.com/Egonex-AI/Understand-Anything/blob/4e6d3299017a7432da15957874992837a315c5b4/LICENSE) is MIT. Bundled WASM grammars and other assets retain their own attribution requirements. No complete domain/community generation execution or external performance claim was independently validated.

## Consequences for Glyphibe

Use one bounded Rust parsing pipeline and one shared attributed graph. Keep source ranges and derivation provenance; do not equate inferred call targets, communities and business meaning. Re-resolve relationships after file changes even when only one file needed parsing. Reconcile stable identities by qualified symbol identity and fingerprints, with ambiguity explicit. Cache only data outside analyzed roots. Keep optional LSP enrichment behind explicit adapters and workspace-execution safeguards. Borrow ideas, not restricted implementation text. Run Glyphibe's own accuracy and performance measurements before making product claims.
