# Evidence-bearing graph model

`glyphibe-core` owns the versioned Rust/JSON model used by the parser, analysis,
knowledge/explanation engines, CLI, MCP and desktop. Frontend types mirror this
schema. There is one attributed graph, not one index per view.

## Two planes and six granularities

| Level | Architecture entities | Explanation coverage |
|---|---|---|
| L1 | Domain/repository scope | Scope goal and established or missing domain evidence |
| L2 | Structural community/cluster | How recorded members relate |
| L3 | Component | Evidenced boundaries and dependencies; current automatic components are directories |
| L4 | File | File organization and source membership |
| L5 | Function, class or control/implementation block | Declared inputs/output syntax, calls and control structure |
| L6 | Selected statement/expression | Syntax facts, literal bindings and prerequisite concepts |

`level` is an attribute from 1 through 6, not a restriction that parents must be
exactly one level above children. `contains` relationships form a graph. A symbol
can belong to a file, directory component and structural community. A directory
component can be reached directly from the repository scope without requiring an
invented intervening cluster. The current L1 repository entry deliberately marks
its business domain as unverified.

Each current architecture node has a canonical explanation node at the same
level. Additional `maps_to` edges connect members to the explanations of their
direct containing scopes. Thus one architecture member can map to several
explanations, and one aggregate explanation can cover several members. Mapping
queries follow edges in either direction; hierarchy queries are graph traversals.
Concept nodes live in the explanation plane and connect through `has_concept`.

## Nodes, edges and provenance

A node records `id`, `kind`, `label`, `level`, `plane`, hashed repository identity,
Git revision anchor, optional language/source range, structural summary,
fingerprint, tags, concept IDs, change state and evidence. Source paths are
repository-relative with forward slashes. Lines and columns are one-based;
Tree-sitter columns and `start_byte`/`end_byte` are UTF-8 byte positions, not UTF-16
or grapheme offsets. Reading source requires the selected root and validates the
relative path and canonical containment.

An edge records its own identity, endpoints, relation, evidence and explicit
`stale` flag. Evidence contains a category, method, source-span list and note.
Repeated relationships with the same endpoints, relation, method and evidence
category merge distinct source locations rather than dropping later call sites.
Different evidence categories remain separate. A non-serialized edge index is
rebuilt after identity reconciliation and bulk edge mutation.
The graph schema supports:

| Relation | Current use |
|---|---|
| `contains` | Syntax membership, directory components and inferred community membership |
| `calls` | Conservatively resolved static call targets |
| `imports` | Resolved local module/file import syntax |
| `references` | Lexical symbol references and explicit base-type clauses |
| `depends_on` | General dependency queries and isolated semantic proposals |
| `implements` | Explicit Rust trait and TypeScript interface implementation clauses |
| `maps_to` | Cross-plane canonical and scope explanations |
| `has_concept` | Syntax-associated knowledge concepts |
| `verified_by` | Reserved for applicable independent verification evidence; scans do not invent verification |

The categories are intentionally independent of relation type:

- **Observed:** directly extracted syntax, positions, explicit declarations or
  original knowledge records.
- **Inferred:** lexical call targets, structural communities, concept associations
  and deterministic explanation templates. Static targets do not guarantee a
  particular runtime dispatch.
- **Proposed:** a user/Agent hypothesis, stored in a separate SQLite proposal
  table. It does not silently alter observed graph facts.
- **Verified:** requires supporting evidence metadata. Merely passing unrelated
  unit tests does not make every analyzed relationship verified.

Validation checks node identities/levels/repository membership, safe source
paths/ranges, edge uniqueness, existing endpoints, cross-plane `maps_to` and
concept targets for `has_concept`. Evidence validation is structural; it cannot
prove a supplied independent test was sufficient for an arbitrary semantic claim.

## Identity and change tracking

IDs are SHA-256 hashes of length-prefixed inputs. Local repository identity is
derived from its canonical root; clones at different locations therefore have
different repository identities. Symbol identity combines that repository,
relative file, qualified structural scope, kind and occurrence within that
scope. It does not use source line numbers as its sole identity.

Exact IDs are compared first. New/missing symbols can be reconciled by a unique
compatible kind/language/fingerprint pair. Function fingerprints use the body
syntax. Bottom-up token/subtree hashing ignores layout and comments while
preserving string-literal whitespace and token structure. File fingerprints
hash exact UTF-8 content. Rename/move reconciliation is attempted, never promised
for ambiguous or structurally changed entities. Recursive renaming can change a
body fingerprint; repeated identical bodies can prevent a unique match.

Nodes distinguish unchanged, added, modified, removed and ambiguous states.
Old relations absent from the new graph remain stale history. Normal queries
exclude removed nodes and stale/proposed relationships from dependency traversal.
Source ranges and fingerprints reflect the working tree that was read; Git HEAD
is a revision anchor, not proof that all scanned bytes equal a committed snapshot.
MCP source retrieval verifies the current file fingerprint. Removed historical
nodes return metadata with source unavailable; their old byte ranges are never
applied to current file contents. Traversal from a removed origin is rejected.

## Queries and storage

Breadth-first traversal powers upstream/downstream/both-direction reachability,
bounded call traces, structural ancestors/descendants, mappings and modification
impact. Results retain paths, configured hop count and truncation state. Impact
summaries count affected functions, containing components, recognized test
candidates and known interface/export tags. Reachability is not a risk percentage
and does not assert a future runtime failure.

SQLite stores graph JSON, per-file hash/extraction JSON, repository metadata and
separate validated proposals. It uses an external user cache, WAL mode,
parameterized SQL and `trusted_schema=OFF`. Current query processes deserialize
the complete graph. A SQLite application ID identifies new caches. Compatible
legacy Glyphibe caches can migrate; nonempty foreign databases are rejected
before Glyphibe schema creation. Existing symlink/reparse-point cache ancestors
are rejected; on systems with symlinked temporary roots, use their canonical
paths.
Historical retention is not a compact event log with a
configured retention policy; large refactors can grow the snapshot and should
be measured. Five unchanged benchmark rescans check graph-count stability but do
not establish a long-term history bound.

Known limits and evidence of actual scale are recorded in the
[analysis benchmark report](../benchmarks/analysis.md). Changes to schema or
identity semantics require migration/version review and corresponding tests.
