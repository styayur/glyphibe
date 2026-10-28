# Hosted release candidate: analysis measurements

These are actual measurements of the **CI candidate** from source
`0f1e4b64acf46752349029bbba4bf265efb13c6f`, not final tagged or published Release assets.
The older [development results](analysis.md) remain separate; their timings must
not be substituted for this binary. [Raw samples](release-candidate-analysis.json)
include every successful cold, hot, increment and query measurement.

## Binary and environment

CLI: `glyphibe 1.0.0`, Windows x64 MSVC, SHA-256
`c85d4b38a785cdf885f5610ea18d40c3573ed80215974b5e285d1ff18fd55386`.
It was built by [GitHub Actions run 40683111999](https://github.com/styayur/glyphibe/actions/runs/40683111999)
on `win25-vs2026` image `20261021.278.1`, Rust 1.98.1, stable Visual Studio
Enterprise 2026 channel `VisualStudio.18.Release`, toolset `14.51.36231`.
The Cargo release profile uses default optimization level 3, thin LTO, stripping
and one codegen unit. The report records the sanitized build provenance and its
exact asset SHA-256; this is not a reproducible-build certification.

Measured on 2026-10-28: Windows 11 build 26200, AMD Ryzen 7 H 255,
16 logical processors, 24,445,165,568 bytes physical memory, Python 3.14.7.
There was no intentionally concurrent compile or renderer benchmark. An idle
native desktop window was present at the beginning and closed after the
1,000-function case. OS background activity was not isolated and file caches
were not flushed. Observed differences from older binaries are not a controlled
causal regression analysis.

## Method and results

The unchanged `scripts/benchmark.py` generated actual Python source in a fresh
external work directory, with 100 functions per file and a statically resolvable
call chain. It parsed that source through the selected executable and never
executed the workload or installed dependencies. These are synthetic **source
workloads**, not fabricated graph facts or representative business architectures.

Cold means a fresh SQLite index. Hot means five separate CLI processes reusing
unchanged file parse caches. Incremental changes one source literal in one file.
Full relationships, clusters, explanations and persistence are rebuilt in both
hot and incremental cases. End-to-end timings include process startup, cache
reads, reconciliation, enrichment, JSON and SQLite writes. Parser-only
`stats.elapsed_ms` is separately available in the raw report.

Peak memory below is the highest Windows `PeakWorkingSetSize` sample among the
cold, five hot and incremental indexing processes. It excludes the Python
harness and short-lived Git subprocess; query/export peak memory is not retained
in this table. Each subprocess was bounded to 180 seconds and 2 GiB. Search
latency is the median of five new CLI processes, including graph deserialization.

| Functions | Source files / bytes | Cold s | Hot median s | One-file increment s | Peak indexing MiB | SQLite MiB | Search median ms |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 1,000 | 10 / 50,868 | 7.007 | 9.680 | 9.453 | 96.4 | 46.5 | 1224.7 |
| 5,000 | 50 / 263,426 | 32.224 | 52.286 | 53.252 | 423.4 | 231.7 | 5571.0 |
| 10,000 | 100 / 529,126 | 71.199 | 103.121 | 98.758 | 837.9 | 461.7 | 13735.9 |

All three cases completed with **zero harness failures**. Every hot scan parsed
zero files; every increment parsed exactly one. Cold plus five unchanged hot
scans retained stable graph node totals of 8,099 / 40,261 / 80,407. These totals
include statements, expressions, concepts and derived mappings, rather than only
functions. Function-only graph exports contained exactly 1,000 / 5,000 / 10,000
parsed functions and 999 / 4,999 / 9,999 resolved calls, respectively.

Hot scans were slower than cold scans, and the 10,000-function search median was
13.74 seconds. Large JSON snapshots, rebuilding derived relations and whole-graph
loading remain substantial performance limitations. A cached parse is not a
fast full-index update. These observations do not substantiate compact storage,
streaming graph queries or instant repository-wide incremental analysis.

The 47 manually annotated distinct caller/callee pairs produced 47 true
positives, zero false positives and zero false negatives: precision and recall
100% **on that finite fixture set only** (Python 11, Rust 11, JavaScript 12,
TypeScript/TSX 13). This does not estimate real-project call or business-mapping
accuracy. Callback/dynamic dispatch remains explicitly unresolved.

This backend run does not measure desktop GPU FPS, graph drawing cadence,
selection latency or native window startup. Rendering recommendations must be
assessed against their own measured environment; unmeasured native GPU FPS is
unknown, not zero. This candidate's measured latency and storage limitations
remain part of release-gate review rather than being hidden behind older results.

## Reproduction

Verify the selected executable SHA and CI provenance, then use a new external
cache and a new report name:

```text
python scripts/benchmark.py --binary CANDIDATE_CLI --work-dir NEW_EXTERNAL_CACHE --output NEW_REPORT.json --sizes 1000 5000 10000
```

No existing benchmark report was overwritten by this run. The paired
[public project checks](release-candidate-public-projects.md) and
[MCP measurements](release-candidate-mcp.md) use the same candidate executable.
