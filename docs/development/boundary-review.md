# Core and Agent boundary review

Review date: 2026-10-28 (Asia/Singapore). Scope: graph identity/evidence,
SQLite ownership, source reads, historical queries, CLI/MCP validation and the
external Agent acceptance harness. This is a targeted engineering review, not
an assertion that every security or v1.0.0 release gate has passed.

| Finding | Correction | Verification |
|---|---|---|
| Library progress printing could panic when a detached desktop had a closed stderr pipe | Parser progress uses only its callback; CLI progress/error writes ignore disconnected stderr, and stdout writes propagate errors | `disconnected_progress_stderr_never_panics_the_scanner` runs the real CLI with its stderr pipe closed |
| Repeated calls to the same target lost all but the first source position | Matching edge evidence merges sorted distinct spans; different evidence categories remain separate | `duplicate_edges_merge_distinct_callsites_and_preserve_evidence_categories` |
| Identity reconciliation changed edge IDs without refreshing the private index | Rebuild the ID-to-position index after reconciliation and known bulk mutations | `identity_remap_reindexes_edges_without_history_or_count_change` |
| Historical removed nodes could apply old byte offsets to current source | MCP returns removed metadata with source unavailable; current source requires an active file fingerprint; removed traversal origins are rejected | Real SDK stdio integration changes source, verifies stale refusal, rescans a deletion, then verifies historical metadata and trace refusal |
| Metadata-only size checks did not bound a file that grew before reading | Both parser and core source reads use a limit-plus-one reader and reject excess bytes | Bounded-reader unit tests verify consumption stops at limit plus one, invalid UTF-8 is rejected, and parser cancellation is honored |
| Opening an unrelated SQLite file could create Glyphibe tables before repository binding failed | Check application ID, exact table set, columns/types/defaults/primary keys and normalized original CREATE SQL before DDL; reject unexpected views/triggers/indexes | Foreign-schema regressions verify bytes, journal mode, application ID and schema remain unchanged, including misleading four-table names and changed CHECK constraints; valid legacy cache migration still succeeds |
| CLI search silently accepted zero/oversized limits | Validate 1..10,000 results and a 1,000-byte query bound | CLI integration verifies invalid limits fail without JSON stdout |
| Agent acceptance counted any eight completed MCP events | Require all eight distinct successful Glyphibe tool names, reject other operation types, verify fixture hashes, impose a 180-second timeout and kill only the started process tree | Syntax checked; actual Codex CLI 0.159.2 run passed all eight tools, zero forbidden operations, unchanged fixture; see `agent-interoperability.json` |

At the initial review checkpoint, six relevant Rust packages passed 53 tests:
core 14, parser 24, analysis 5, explanation 7 and CLI integration 3. The MCP crate
is exercised by the CLI's real SDK stdio integration. Strict Clippy passed for
all six packages. Native application installation/startup must still be retested
with the rebuilt package; the disconnected-pipe regression is not a substitute
for that lifecycle check.

The additional strict legacy-schema regression subsequently passed, as did the
existing legacy migration test and strict core Clippy. It tests wrong column/PK
definitions, a changed graph-row CHECK constraint, an injected trigger and a
foreign table whose name resembles SQLite's internal prefix. No foreign database
bytes or persistence PRAGMAs changed on rejection.

Cache and source symlink/reparse-point checks remain strict. Tests canonicalize
their own temporary roots so a platform's conventional symlinked temporary
directory does not itself masquerade as untrusted input. Actual macOS/Linux CI
results must be reported independently of the Windows results above.

The external client uses existing authentication with an ephemeral Codex CLI
invocation and a read-only sandbox. Its only test repository is the authored
public Python fixture. Raw JSONL and stderr stay in an external local cache;
public evidence includes sanitized tool names/outcomes and executable hashes.
Online Agent availability is separate from Glyphibe's offline analysis core.

## Deterministic teaching and receiver audit

A later source-freeze review found two reproducible analysis errors in the
then-built 1.0.0 development executable. A Python assignment `x = 1` incorrectly
carried `iterator:1`, causing its lesson to say “Loop input: 1”. Generic right
operands no longer acquire loop meaning. Python and JavaScript/TypeScript for
loops use their actual iterable field; Rust for-expressions use their value
field. JavaScript/TypeScript for-in/of now produces a real loop block. The
existing knowledge pattern already recognized that syntax kind.

The same review constructed valid Python with a global `self = Other()` and
`Counter.report(this)` calling `self.value()`. The former convention-only rule
incorrectly selected `Counter.value`. Python receiver inference now requires
the actual first positional `self`/`cls`/`this` parameter in the enclosing method,
with local assignment, unpacking, loop rebinding, import aliases and nested
parameter shadowing conservatively unresolved. Static or unknown decorators do
not establish a receiver. Recognized classmethod/property syntax is rejected
when those decorator names are shadowed. JavaScript's global `self` is distinct
from its class `this`; Rust module `self::name` is distinct from `self.name`.

Cache format `parser-9` forces extraction refresh. All 28 parser tests passed,
including the 47 annotated call pairs and the new four-language loop and
receiver regressions. Strict parser Clippy and formatting passed. These tests
do not establish complete runtime dispatch or general type inference. A final
binary/package rebuild and the release gate checks remain separate obligations.
