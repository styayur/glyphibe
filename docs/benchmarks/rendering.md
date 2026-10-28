# Rendering and interaction measurements

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

## Reproduce

Build the real CLI and run the source generator/analyzer harness first; consult `python scripts/benchmark.py --help` for its required binary, work directory and output arguments. Keep generated graphs outside the analyzed source tree.

```powershell
cd apps/desktop
npm ci --ignore-scripts
npm run build
npm run benchmark:render -- --graphs PATH-TO-BENCHMARK-GRAPHS --out rendering-results.json --duration 4000
```

The harness starts and closes a local production preview server and headless browser. An existing preview URL can be supplied with `--url`. `--modes Dense --repetitions 1` provides a narrower follow-up run; its sampling configuration is recorded explicitly. Background load, power settings, browser version and hardware can affect results. JavaScript heap after a sample is recorded in the raw data; it is not peak process or GPU memory.
