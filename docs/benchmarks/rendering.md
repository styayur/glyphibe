# Rendering and interaction measurements

Latest frozen candidate: [0f1e4b6 software-rendering report](rendering-release-candidate-0f1e4b6.md),
using new candidate-generated inputs and actual opaque panels. It retains all
six scenarios and short-window timing anomalies; it does not establish native FPS.

The measurements below use the real production frontend and source-derived graph exports. They do **not** measure GPU-presented frames or prove the installed WebView2 application's native frame rate. A 30 FPS native performance target remains distinct from these browser measurements.

## Environment and inputs

Initial measurements: 2026-10-27, Asia/Singapore. Windows build 26200, x64; AMD Ryzen 7 H 255 with Radeon 780M Graphics; 16 logical CPUs; 24,445,165,568 bytes system memory. The measured renderer was headless Chromium 156.0.8078.4 using explicitly selected ANGLE SwiftShader software rendering, at 1440 × 960 and device scale factor 1. Native Radeon GPU throughput was not measured by this harness.

The original benchmark generator creates Python source containing a call chain. The Rust analyzer genuinely parses that source. Its deterministic function/call projection contains exactly 1,000 / 5,000 / 10,000 canonical function nodes and 999 / 4,999 / 9,999 canonical edges. Input SHA-256 values, actual graphics strings, all repetitions and raw latency samples are recorded in the JSON reports. These are generated-source microbenchmarks, not claims about the shape of arbitrary real repositories.

## Method

`apps/desktop/scripts/measure-rendering.mjs` imports each graph through the application's file chooser, sets density with the real control, and performs pointer pan/orbit and wheel zoom. Standard glass overlays remain visible. Each full scenario samples three windows of approximately four seconds.

The harness records animation-callback timestamps and actual WebGL draw-submission activity. The reported cadence is callback frequency; submitted draw calls can complete asynchronously, so this must not be relabeled as displayed FPS. All measured scenarios recorded draw activity and no page errors.

Selection latency measures a real DOM click through the second subsequent animation callback, including React state updates and rendering work. Level latency alternates populated function L5 and empty file L4, because these benchmark exports deliberately contain only functions. It therefore measures projection teardown/reconstruction, not navigation between two populated layers or a source rescan. Each report preserves every sample; p95 values here have small sample counts and are not statistical guarantees.

## Full run after renderer reuse and automatic 3D limits

Raw evidence: [rendering-results.json](rendering-results.json). Medians in the cadence column are across three repetitions. Latencies are milliseconds.

| Canonical nodes | Mode | Drawn nodes | Median callback Hz | Selection p95 | Level-switch p95 |
|---:|---|---:|---:|---:|---:|
| 1,000 | Dense | 1,000 | 51.00 | 34.6 | 404.9 |
| 1,000 | Galaxy | 500 | 46.50 | 162.0 | 130.0 |
| 5,000 | Dense | 5,000 | 16.25 | 158.2 | 1,552.6 |
| 5,000 | Galaxy | 500 | 48.00 | 115.4 | 164.4 |
| 10,000 | Dense | 10,000 | 9.58 | 354.6 | 2,017.3 |
| 10,000 | Galaxy | 500 | 60.00 | 149.9 | 96.5 |

The software 3D cap reduces the visible subset to 500; canonical nodes remain queryable. The table does not represent drawing all 5,000 or 10,000 nodes in 3D. Users can override automatic limits up to the documented 2,000-node cap.

Dense at 5,000 and 10,000 nodes remains slow in this software-rendering environment. This does not establish the preferred 30 FPS target. A lower visible density and local Frontier exploration are available controls; the measurements do not imply runtime failures or a percentage risk in the analyzed code.

## Baseline and adverse result

[rendering-baseline.json](rendering-baseline.json) preserves the preceding unoptimized run, rather than replacing its evidence. Initial selection p95 was 199.8 / 399.8 / 651.4 ms for Dense at the three sizes. The initial Galaxy run drew 1,000 / 2,000 / 2,000 nodes, with median callback cadence 23.24 / 12.71 / 13.17 Hz. Its drawing workload differs from the automatic 500-node policy above, so a direct frame-rate improvement claim would be misleading.

Renderer reuse improved the observed selection latencies, but the first implementation also repeatedly refreshed Sigma settings when constructing a new projection. The full run above exposed a level-switch regression: baseline Dense p95 was 166.4 / 332.7 / 565.9 ms, versus 404.9 / 1,552.6 / 2,017.3 ms. This adverse result is retained.

## Batching follow-up under concurrent load

The follow-up implementation batches presentation settings and skips the redundant refresh immediately after constructing a renderer. A browser regression verifies that selection and impact highlighting retain the same WebGL canvas. This is a structural verification; it does not establish a speed improvement.

[rendering-latency-followup.json](rendering-latency-followup.json) records the follow-up from **2026-10-28 08:13 Asia/Singapore**, using the same graph input hashes, Dense only and one four-second interaction window per size. Rust release CLI LTO compilation overlapped this run, as confirmed by the coordinating process; unit and license checks also ran. The host was noticeably slower than the previous evening. This is an **uncontrolled concurrent-load observation**, not a causal before/after comparison.

| Canonical/drawn nodes | Callback Hz (one window) | Selection p95 ms | Level-switch p95 ms | Page errors |
|---:|---:|---:|---:|---:|
| 1,000 | 17.93 | 459.8 | 4,029.4 | 0 |
| 5,000 | 16.18 | 1,048.5 | 7,732.9 | 0 |
| 10,000 | 7.11 | 2,957.7 | 20,569.7 | 0 |

These adverse latency values are retained in full. They show sensitivity to software rendering and shared CPU load. They do not prove that batching improved level-switch performance, and do not support claiming a native GPU FPS target. Source parsing was not invoked by any measured slider or selection action. No repeated retiming was used to select a favorable result.

## Camera candidate snapshot

[rendering-final-candidate.json](rendering-final-candidate.json) records the
2026-10-28 14:14 Asia/Singapore camera candidate, using the same three inputs,
three four-second windows per scenario and the same software renderer. The
native application was closed; no release LTO ran during this sample. Targeted
debug parser checks may have overlapped part of the run, and other host load
was not controlled. These are observations, not a causal optimization comparison.

| Canonical nodes | Mode | Drawn nodes | Median callback Hz | Selection p95 ms | Level-switch p95 ms |
|---:|---|---:|---:|---:|---:|
| 1,000 | Dense | 1,000 | 12.56 | 389.0 | 3,293.6 |
| 1,000 | Galaxy | 500 | 7.18 | 776.2 | 1,394.5 |
| 5,000 | Dense | 5,000 | 4.70 | 841.0 | 8,881.4 |
| 5,000 | Galaxy | 500 | 6.44 | 1,014.7 | 611.7 |
| 10,000 | Dense | 10,000 | 3.40 | 1,818.5 | 12,138.9 |
| 10,000 | Galaxy | 500 | 6.89 | 1,387.4 | 526.2 |

All six scenarios recorded real WebGL draw submissions and zero page errors.
The poor software-rendering results are retained. They establish neither a
fluid 1,000-node software-rendering experience nor the preferred 5,000-node
30 FPS target. Native GPU-presented FPS remains unverified. The camera change
has separate geometric, browser and native correctness evidence; these numbers
do not prove a performance benefit. Lower density, local exploration, opaque
panels and focus mode remain user controls, not evidence of meeting a target.

## Panel composition check

[rendering-panels-completed.json](rendering-panels-completed.json) records a
subsequent 1,000-node Dense run with the same visible islands and three windows
per preference. Glass median callback cadence was 10.89 Hz; opaque was 13.67 Hz.
Selection p95 was 359.6 / 353.0 ms respectively. The opaque preference modestly
reduced the observed cost but did not establish a fluid software-rendering
experience. The sequential comparison has uncontrolled host load and debug
checks may overlap; it is not a statistical causal estimate.

The [initial partial attempt](rendering-panels.json) is retained: its glass
sample completed, then the harness looked for the appearance control before
loading a graph and timed out. That harness-order error was corrected before
the complete run; it was not a product assertion failure. The reports identify
the actual production asset hashes. This snapshot precedes the default opaque
software-rendering policy and native asynchronous-result isolation changes.

The current software-driver fallback enables opaque panels initially; users may
restore glass explicitly. Hardware drivers retain the glass default. This
avoids imposing the optional blur cost on a known software path, without claiming
that the fallback meets an FPS target.

## Reproduce

Build the real CLI and run the source generator/analyzer harness first; consult `python scripts/benchmark.py --help` for its required binary, work directory and output arguments. Keep generated graphs outside the analyzed source tree.

```powershell
cd apps/desktop
npm ci --ignore-scripts
npm run build
npm run benchmark:render -- --graphs PATH-TO-BENCHMARK-GRAPHS --out rendering-results.json --duration 4000
```

The harness starts and closes a local production preview server and headless browser. An existing preview URL can be supplied with `--url`. `--modes Dense --repetitions 1` provides a narrower follow-up run; its sampling configuration is recorded explicitly. Background load, power settings, browser version and hardware can affect results. JavaScript heap after a sample is recorded in the raw data; it is not peak process or GPU memory.

Use `--sizes 1000 --modes Dense --panels glass,opaque` for the panel comparison.
The preference is changed through the actual appearance control after loading
the graph; import timing for opaque includes that control interaction.
