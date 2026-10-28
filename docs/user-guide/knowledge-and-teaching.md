# Offline knowledge and progressive teaching

Glyphibe's default knowledge is original bilingual text by Glyphibe contributors,
distributed under MPL-2.0. Each of the 34 concepts records its author, license,
redistribution basis and reference-only links. Referenced websites are not copied
or bundled. The registry works without a network, model or API key.

The Concept Registry covers variables, types, scope, functions, parameters,
returns, expressions, branches, loops, objects, interfaces, inheritance, errors,
collections, algorithms, complexity, modules, dependencies, calls, APIs, HTTP,
databases, async work, concurrency, synchronization, recursion, tests, components,
architecture and three design patterns. The Pattern Library associates syntax
node kinds with teaching concepts; it does not certify that a business design
pattern is implemented. The Learning Graph orders prerequisite concepts before
the requested topic and rejects prerequisite cycles.

## Reading an explanation

Select an architecture node and choose a teaching depth from 1 (beginner) to 4
(professional). This setting is independent of architecture level L1–L6.
Explanations use the same graph facts as CLI, MCP and the desktop. They include:

- Entity kind, architecture level and source range where one exists.
- A recorded syntax summary; extracted input/output annotations where available.
- Conditions, iterator input and assignments when represented by syntax tags.
- Literal assignment sites when a binding pattern and scalar literal RHS are
  extracted. The literal is known if that statement is reached; no expression
  evaluation, path propagation or final runtime value is claimed.
- Resolved calls, structural members and an explicit warning about missing facts.
- Original concept explanations, with more detail at greater teaching depth.
- Four lesson steps, each with a question, hint, answer and review prompt.

Both `en` and `zh`/`zh-CN` are supported. No template claims a runtime value,
business purpose, execution order, complete call graph or fault probability.
Absent parameter/type metadata remains unknown. Type annotations describe
declarations, not demonstrated runtime results. Parse diagnostics and historical
or ambiguous identities remain visible in uncertainty notes. User and Agent
proposals retain their own evidence category.

Each live architecture node maps to an explanation at the same level. Its
explanation can also cover explicitly contained members. A member may have
multiple containing aggregates; mappings therefore form a many-to-many graph,
not a mandatory six-level tree. Mapping evidence distinguishes canonical
structural explanation from aggregate scope coverage. Removed nodes' generated
mapping edges are marked stale. Concept associations are inferred from recorded
syntax, rather than inferred business names.

## Importing public GitHub knowledge

The library entry point `import_github(url, destination)` accepts only an HTTPS
repository URL such as `https://github.com/owner/repository`. File/branch URLs,
credentials, alternate hosts, queries and fragments are rejected. Imports use
GitHub's API and raw-content hosts with redirects disabled, pin documents to the
default branch's immutable commit and retain root-license detection provenance.
They never clone a repository, run a script, install a dependency or invoke a
model. Public network requests use no credential or authorization header.

Only ordinary Markdown blobs are eligible. Symlinks, submodules and non-UTF-8
documents are excluded. Limits are 24 files, 256 KiB per file, 2 MiB total text
and 4 MiB per metadata response; timeouts bound individual requests. Truncated
GitHub trees or limit-based omissions appear as warnings. Heading terms are
normalized and duplicate terms retain all source URLs; conflicting text is
preserved as alternate text instead of being declared equivalent.

The importer stores heading sections as **untrusted local data**. Fenced code
and HTML lines are excluded from extraction. Natural-language instructions in
a document remain inert text; callers must display them as text, never HTML or
commands. A repository's root license does not prove every document has the same
license. Every imported bundle remains local-only pending per-document review,
including bundles whose root license is permissive. Unknown licenses never
authorize redistribution. Imported text never automatically enters the default
knowledge package.

Repeating an import at the same commit returns `unchanged: true` without
refetching documents. A changed commit replaces only that repository's existing
validated Glyphibe index using an atomic temporary-file operation. The caller
chooses the destination outside the analyzed project. Existing non-Glyphibe
files and symbolic-link destinations are rejected.

`search_imports(destination, query)` searches an explicitly chosen local catalog,
including normalized headings, text and alternate descriptions. Each hit retains
repository, immutable revision, root-license provenance and untrusted/local-only
flags. It does not merge remote material into the redistributable registry.

## Evidence limits

The registry and templates are covered by offline unit tests, including both
languages, prerequisites, six-level mapping, many-to-many coverage, invalid
controls, malicious Markdown, URL restrictions and response limits. A real
network import and real GUI/Agent teaching interactions require separate
integration evidence. This document describes implemented library behavior;
it does not assert that the v1.0.0 release gates have passed.

On 2026-10-27, the two libraries' 15 offline tests passed. An explicitly enabled
network integration test also imported the public Tree-sitter Python repository
at commit `26855eabccb19c6abf499fbc5b8dc7cc9ab8bc64`: one Markdown file and two
heading concepts, root MIT license detected, local-only status retained, and
unchanged revision recognized on repeat. This is a real import check for that
repository, not a claim of arbitrary-document semantic understanding.
