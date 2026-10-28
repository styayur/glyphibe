# Evidence-backed modification scope

Impact queries first identify source scope, then traverse dependency edges.
Aggregate domains, clusters, components, files and types expand through active
`contains` evidence. A statement or block walks all ownership branches to its
nearest function or file; this walk is not limited to six nesting edges.
Explanation nodes resolve active architectural mappings, preferring matching
source ranges and display levels while retaining equally specific mappings.

`Impact.scope` records architectural origins, dependency seeds, structural
nodes and each structural/mapping edge used as basis. These steps do not spend
dependency hops. Paths begin at the actual seed; they never invent calls from
a component or expression. Ordinary function call paths retain their meaning.
Unresolved scope is explicit and an empty result does not establish no impact.

Removed and Proposed nodes, stale edges and Proposed relationships do not
establish scope. Cycles and multiple parents are handled with visited sets.
Scope nodes, basis and dependency results are bounded to 50,000 with explicit
truncation. Counts include contained code scopes and discovered dependencies;
they are not runtime failure predictions. Native GUI, CLI and MCP use this Rust
query. Browser-import mode exposes a raw dependency projection and labels its
reduced scope separately.

Regression evidence: `crates/glyphibe-core/tests/impact_scope.rs`, including
aggregate/fine/mapped scopes, deep nesting, multiple parents, cycles, exclusion
of stale/proposed facts and the safety cap.
