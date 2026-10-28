# Hosted release candidate: public project checks

Three separately acquired public checkouts were scanned read-only on 2026-10-28
using the GitHub-hosted Windows CLI candidate from source `0f1e4b64acf46752349029bbba4bf265efb13c6f`.
CLI SHA-256: `c85d4b38a785cdf885f5610ea18d40c3573ed80215974b5e285d1ff18fd55386`. This is a CI candidate measurement, not a claim about
final main/tag builds or published Release assets. [Raw results](release-candidate-public-projects.json)
retain exact revisions, license source links and hashes, counts and timings.
The [candidate analysis report](release-candidate-analysis.md) records the build
provenance, environment and matching synthetic benchmark. Earlier local-binary
[public results](public-projects.md) are retained as historical evidence.

## Measured results

| Project / revision | Root license | Parsed files / bytes | Nodes / edges | End-to-end s | Peak CLI MiB | SQLite MiB | Tracked regular files checked |
|---|---|---:|---:|---:|---:|---:|---:|
| [pallets/click](https://github.com/pallets/click/tree/2247b35ea1c47c727d7a06e51fa280e12a863ff6) / `2247b35` | BSD-3-Clause | 91 / 1,003,633 | 47,279 / 159,534 | 41.065 | 461.3 | 179.0 | 178 |
| [BurntSushi/ripgrep](https://github.com/BurntSushi/ripgrep/tree/91f5d22ef48bde459d90a8c7dbeab416e97d0f2b) / `91f5d22` | Unlicense OR MIT | 102 / 1,879,988 | 63,753 / 222,844 | 64.501 | 644.7 | 255.8 | 226 |
| [jacomyal/sigma.js](https://github.com/jacomyal/sigma.js/tree/1db338580abb10c75144b3131ba8718550cb177a) / `1db3385` | MIT | 257 / 1,598,447 | 63,557 / 214,947 | 66.725 | 646.9 | 285.9 | 579 |

All three retained identical hashes for every tracked regular file and a clean
Git status before and after analysis. The harness verified each public remote
identity. It did not update the checkouts, install dependencies, execute project
code, run tests/build scripts or copy their source into Glyphibe. Index/cache
files were outside those checkouts. There were **zero harness failures**.

The run used Windows 11 build 26200, Ryzen 7 H 255, 16 logical processors and
24,445,165,568 bytes physical memory. No build, rendering benchmark or native
Glyphibe window ran intentionally in parallel. Cases were serial; OS background
activity was not isolated and file caches were not flushed. The raw report
explicitly corrects the harness's stale default condition text; no numerical
measurements were changed. Cold analysis, JSON serialization and SQLite writes
are included. Peak CLI working set excludes the Python harness and Git
subprocesses. Each CLI process had a 180-second / 2-GiB safety budget.

## Unresolved relations and limitations

| Project | Unresolved imports | Unresolved calls | Unresolved type relations | Static dependency cycles | Skipped source files |
|---|---:|---:|---:|---:|---:|
| pallets/click | 492 | 6,416 | 120 | 2 | 0 |
| BurntSushi/ripgrep | 783 | 11,945 | 242 | 48 | 0 |
| jacomyal/sigma.js | 394 | 9,245 | 23 | 7 | 0 |

Unsupported external libraries, dynamic dispatch and other uncertain relationships
remain explicit diagnostics. A completed scan does not establish that every call
is correct, that a dependency cycle is a runtime defect, or that an algorithmic
cluster represents a business domain. No real-project call or business-mapping
accuracy corpus was manually annotated here. Large persistent snapshots and
whole-derived-graph rebuilds remain measured storage and latency limitations.

The same candidate also repeated the 47 authored distinct call pairs with 47
true positives, zero false positives and zero false negatives. Its 100% precision
and recall apply only to those small fixtures, not these public repositories.
Desktop GPU FPS, rendering latency and native lifecycle are separate checks.
No unmeasured native GPU result is inferred from this source scan.

## Reproduction

Obtain clean authorized checkouts in sibling folders `click`, `ripgrep`, `sigma`
at the exact revisions linked above, then run against the hash-checked candidate:

```text
python scripts/validate-public-projects.py --binary CANDIDATE_CLI --checkout-root CHECKOUTS --work-dir NEW_EXTERNAL_CACHE --output NEW_REPORT.json
```

Do not reuse an existing index when reproducing a cold scan. Original project
licenses are linked and hashed for traceability; this validation does not grant
additional redistribution rights or add their contents to the default knowledge
package.
