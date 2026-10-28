# Analysis benchmark report

Status: **measured development build; not a v1.0.0 release claim**.
The final `glyphibe 1.0.0` executable was additionally checked on
[three pinned public projects](public-projects.md) and against the same 47
annotated call pairs. [MCP latency](mcp.md) is measured separately.
Recorded 2026-10-27 on Windows 11 build 26200, AMD Ryzen 7 H 255 with 16 logical
processors, 24,445,165,568 bytes usable physical memory and Python 3.14.7. See the machine-readable
[raw report](analysis-results.json) for every sample and measurement method.

The measured executable was `glyphibe 0.1.0-dev`, release profile (optimization 3,
thin LTO, stripped), SHA-256
`9951241d7014aa25a2e1e2814c1becdb96aba3464e64b6cb354ca277fd7363f6`.
It includes parser cache version 7, explicit type relations, conventional Rust
workspace roots, bounded reads and declaration-scope safety limits. Its release
build finished before timing began. The rendering benchmark owner held final
timed rendering samples during this run. Background OS activity was not isolated.
An earlier parser-version-4 run overlapped desktop compilation and is preserved
as [preliminary raw data](analysis-preliminary-results.json); its timings are not
combined with the table below.

## Method

`scripts/benchmark.py` generates real Python source in an external temporary
directory: 100 functions per file, a static call chain across files, and no
third-party dependencies. It invokes the actual CLI and never executes the
generated project. Graph exports for UI tests contain only the actual parsed
function nodes and resolved call edges. These are synthetic **source workloads**,
not fabricated graph facts or representative product architectures.

Cold means a new SQLite index; the OS filesystem cache is not flushed. Hot means
five new CLI processes with unchanged per-file parse caches. A single-file
increment edits a real literal in one source file. Relations, communities and
explanation mappings are rebuilt in both hot and incremental runs. End-to-end
time includes process startup, parsing/cache reads, reconciliation, enrichment,
mapping, JSON serialization and SQLite persistence. `stats.elapsed_ms` in the raw
report measures the parser phase, not all these operations.

Windows `GetProcessMemoryInfo.PeakWorkingSetSize` is polled for the CLI process;
the Python harness and short-lived Git metadata subprocess are excluded. Query
latency is five CLI searches including startup and full graph deserialization.
Every subprocess has a 180-second/2-GiB safety budget. Errors are retained in the
raw report rather than converted to successful measurements.

## Results

| Functions | Source files / bytes | Cold seconds | Hot median seconds | One-file increment seconds | Highest measured CLI working set MiB | SQLite MiB | Search median ms |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 1,000 | 10 / 50,868 | 1.696 | 1.948 | 1.554 | 93.9 | 46.5 | 189.5 |
| 5,000 | 50 / 263,426 | 5.452 | 8.023 | 10.080 | 408.9 | 232.0 | 893.9 |
| 10,000 | 100 / 529,126 | 10.625 | 16.896 | 16.100 | 816.4 | 462.2 | 1,724.5 |

All five hot scans parsed zero files. Each increment parsed exactly one file.
Graph node counts remained stable across cold plus five unchanged hot runs:
8,093, 40,261 and 80,401 respectively. These totals include source structure,
clusters, components, explanations and concepts; they are larger than the
function-only exports used for UI measurements.

Hot scans were **not faster end to end** in this run. Reconciliation, rebuilding
derived graphs and rewriting/deserializing large JSON snapshots dominate the
saved parse work. Index size and second-scale 10,000-function CLI searches are
known performance limitations. The current architecture is not yet evidence of
efficient streaming graph queries or compact persistent graph storage.

## Accuracy and unresolved behavior

The published authored fixtures contain 47 manually annotated distinct
caller/callee pairs: Python 11, Rust 11, JavaScript 12, TypeScript/TSX 13. The
actual CLI produced 47 true positives, zero false positives and zero false
negatives on this set: precision 100%, recall 100%. These figures describe only
these small fixtures. They do not estimate accuracy on arbitrary repositories,
dynamic languages in general, inheritance dispatch or real business mappings.
Callback calls intentionally remain unresolved and are tested separately.

No independently annotated real-project call corpus or business-domain mapping
accuracy measurement has been completed. Cluster modularity is an internal
structural objective, not proof of correct business architecture.
UI drawing cadence/FPS, native desktop startup and MCP client latency require
their own measurements and must not be inferred from this backend table.

## Authorized real-project robustness scans

Two existing local projects were analyzed read-only with new external indices.
No project code, scripts, tests or dependency installation was executed. The
public report deliberately identifies cases by number and omits source paths,
symbol labels, source text, Git revision and diagnostic messages.

| Case | Parsed files / source bytes | Nodes / edges | End-to-end seconds | Peak CLI working set MiB | SQLite MiB |
|---:|---:|---:|---:|---:|---:|
| 1 | 87 / 483,153 | 21,509 / 76,186 | 4.303 | 232.4 | 90.3 |
| 2 | 142 / 590,359 | 38,765 / 128,075 | 7.001 | 386.4 | 153.0 |

Both completed within the harness limits with zero skipped source files. They
produced 2 and 3 syntax-error diagnostics, respectively. Case 1 recorded 295
unresolved imports, 4,697 unresolved calls and 18 unresolved type relationships;
case 2 recorded 574, 5,933 and 54. Case 1 also reported 21 static dependency
cycles. These diagnostics are limitations and findings, not automatically
confirmed bugs in those projects. Calls in malformed files are conservatively
unresolved. External-library and dynamic calls also remain unresolved.

Completing these scans establishes bounded robustness on these two working
trees; it does **not** establish real-project call precision, recall, business
mapping correctness or successful builds of the analyzed software. The complete
run recorded zero harness failures across three synthetic and two real cases.

## Reproduction

Build the selected source revision with `cargo build --release -p glyphibe-cli`.
Then run the harness with explicit external output/cache locations:

```powershell
python scripts/benchmark.py --binary <release-glyphibe.exe> `
  --work-dir <external-benchmark-directory> `
  --output docs/benchmarks/analysis-results.json `
  --sizes 1000 5000 10000
```

Optional `--real <authorized-repository> …` arguments perform read-only source
scans and publish aggregate counts only. Never publish private symbol names,
source text, absolute paths or full untrusted diagnostic messages. Record the
binary SHA and workload conditions with each rerun; timings from different
binaries must not be silently combined.
