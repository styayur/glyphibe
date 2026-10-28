# Frozen release-candidate rendering sample

This is the final software-rendering observation of frontend source
`0f1e4b64acf46752349029bbba4bf265efb13c6f`, collected on 2026-10-28 at
21:00–21:09 Asia/Singapore. It does not establish native GPU-presented FPS or
the preferred interaction targets. [All raw samples and provenance](rendering-release-candidate-0f1e4b6.json)
are retained; earlier reports remain unchanged.

## Inputs and conditions

The candidate CLI from [the six-job Windows build](https://github.com/styayur/glyphibe/actions/runs/40683111999)
generated new exports by actually parsing generated Python call-chain source.
Its SHA-256 is `c85d4b38a785cdf885f5610ea18d40c3573ed80215974b5e285d1ff18fd55386`.
The three exports contain exactly 1,000 / 5,000 / 10,000 canonical functions and
999 / 4,999 / 9,999 call edges. Their hashes are in the raw report.

The existing production frontend ran in headless Chromium 156.0.8078.4 with
ANGLE SwiftShader, on Windows 11 build 26200, Ryzen 7 H 255, 16 logical CPUs and
24,445,165,568 bytes RAM. Viewport: 1440 × 960, scale factor 1. Actual opaque
panels were verified through the appearance control and DOM state, matching the
software-renderer fallback policy. Normal overlays stayed enabled and Focus Mode
was not used; the on-demand detail panel opens when selection begins. Galaxy
automatically drew 500 nodes; its other canonical nodes remained in the graph.

No frontend build ran during sampling. All six production asset hashes matched
before and after; the desktop runtime had no uncommitted changes. The coordinator
confirmed that prior analysis/MCP measurements had finished and the native app
was closed. Before/after process snapshots found no Rust build/link or native
Glyphibe processes. The OS and background services were not isolated or
continuously profiled. These observations are not a causal optimization comparison.

## Complete results

One complete run covered six scenarios, with three requested four-second
interaction windows each. The following lists every window rather than choosing
the best result. Selection and level latency are milliseconds to the second
animation callback, with 15 and 10 samples respectively; they are not screen
presentation latency. Level changes alternate populated L5 with empty L4 and
back because these exports contain only functions.

| Canonical nodes | Mode | Drawn nodes | Callback Hz, all three windows | Selection p95 ms | Level-switch p95 ms |
|---:|---|---:|---|---:|---:|
| 1,000 | Dense | 1,000 | 14.02 / 14.56 / 12.70 | 445.0 | 3,213.9 |
| 1,000 | Galaxy | 500 | 11.80 / 6.13 / 6.29 | 950.8 | 932.8 |
| 5,000 | Dense | 5,000 | 6.49 / 10.86¹ / 5.79 | 1,095.5 | 7,870.3 |
| 5,000 | Galaxy | 500 | 12.46 / 5.77 / 6.38 | 1,577.3 | 475.4 |
| 10,000 | Dense | 10,000 | 3.71 / 2.13 / 80.03² | 1,863.9 | 18,323.6 |
| 10,000 | Galaxy | 500 | 12.43 / 7.53 / 7.28 | 1,305.0 | 659.4 |

All six scenarios completed without page errors, and every window recorded
WebGL draw submissions. Callback frequency is not displayed FPS; the software
path remains slow even with opaque panels and the 3D cap. This run establishes
neither fluid 1,000-node software interaction nor the preferred 5,000-node
30 FPS target. Native GPU-presented FPS remains unverified.

¹ The observed callback timestamp span was only 1,933.17 ms.

² The observed span was only 49.98 ms, containing five callbacks and one callback
with WebGL drawing. Its 80.03 Hz calculation is not a steady-state throughput
estimate. The harness starts its wall-clock budget before its first callback,
so stalls can leave a short observed timestamp span despite a requested
4,000 ms window. Actual spans and counts are preserved for all windows. No
retiming or sample deletion was used to hide this measurement limitation.

## Reproduce

Use the frozen production build and the candidate-generated graph directory:

```text
node scripts/measure-rendering.mjs --graphs CANDIDATE_GRAPH_DIRECTORY --out OUTPUT.json --duration 4000 --repetitions 3 --modes Dense,Galaxy --sizes 1000,5000,10000 --panels opaque
```

Run from `apps/desktop`. The report records actual graphics strings, input hashes,
production asset hashes, every repetition, latency samples and the collection
conditions. Its provenance metadata was appended after collection; measured
sample values were not modified.
