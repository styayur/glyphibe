# Public project validation

> **Latest candidate:** see [release-candidate public-project validation](release-candidate-public-projects.md)
> for the GitHub-hosted binary from source `0f1e4b6`. The results below are
> historical measurements of an earlier local binary, not final tagged assets.

On 2026-10-28, an earlier optimized local **glyphibe 1.0.0** CLI analyzed three complete,
separately cloned public checkouts without installing dependencies or executing
project code. All tracked regular files were hashed before and after analysis;
each checkout also retained a clean Git status. No sources from these projects
were copied into Glyphibe. [Raw results](public-projects.json) include exact
revisions, license source links/hashes, diagnostics, byte counts and measurements.

| Project / revision | Root license | Parsed source files | Graph nodes / edges | End-to-end seconds | Peak CLI working set MiB | SQLite MiB | Source unchanged |
|---|---|---:|---:|---:|---:|---:|---|
| [Click](https://github.com/pallets/click/tree/2247b35ea1c47c727d7a06e51fa280e12a863ff6) / `2247b35` | BSD-3-Clause | 91 | 47,263 / 159,508 | 44.29 | 462.2 | 179.4 | Yes |
| [ripgrep](https://github.com/BurntSushi/ripgrep/tree/91f5d22ef48bde459d90a8c7dbeab416e97d0f2b) / `91f5d22` | Unlicense OR MIT | 102 | 63,753 / 222,844 | 65.72 | 644.5 | 255.7 | Yes |
| [Sigma.js](https://github.com/jacomyal/sigma.js/tree/1db338580abb10c75144b3131ba8718550cb177a) / `1db3385` | MIT | 257 | 63,223 / 213,778 | 63.37 | 644.2 | 284.1 | Yes |

The same Windows workstation described in [analysis.md](analysis.md) was used.
CLI SHA-256: `d1538a0bb77aeb9bef7244d03851b17d8a600d63f777406f4e10517695fd20c1`.
A desktop Rust release compilation was running concurrently. OS filesystem
caches were not flushed. Timings are actual observations under those conditions,
not controlled comparative performance evidence. Peak working set is sampled for
the CLI process, excluding the Python harness and Git metadata subprocesses.
The safety budget is 180 seconds / 2 GiB per CLI process.

These tests verify successful bounded analysis, graph construction and source
integrity. They do **not** certify the accuracy of every call or assign business
meaning to communities. Unsupported imports/calls/types remain diagnostics.
Large graph snapshots and full derived-graph rebuilds are known storage and
latency limitations, visible in the measured index sizes and scan times above.

That historical executable was also rechecked against the 47 annotated basic call
pairs: 47 true positives, zero false positives and zero false negatives. This
100% precision/recall result applies only to that authored, finite fixture set;
it is not a real-project accuracy estimate.

To reproduce, obtain the three named Git checkouts in sibling folders `click`,
`ripgrep` and `sigma`, check out the recorded revisions, and run:

```text
python scripts/validate-public-projects.py --binary GLYPHIBE_EXE --checkout-root CHECKOUTS --work-dir EXTERNAL_CACHE --output REPORT_JSON
```

Use an external work directory and clean authorized checkouts. The harness
checks remote identity and uses only Git metadata plus Glyphibe's read-only
analysis. It never runs a checkout's scripts, tests or package installation.
