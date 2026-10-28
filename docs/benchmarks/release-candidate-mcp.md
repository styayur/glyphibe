# Hosted release candidate: MCP stdio latency

Measured on 2026-10-28 against `glyphibe 1.0.0`, the GitHub-hosted Windows x64
candidate from source `0f1e4b64acf46752349029bbba4bf265efb13c6f`. Server CLI SHA-256:
`c85d4b38a785cdf885f5610ea18d40c3573ed80215974b5e285d1ff18fd55386`. This is the CI candidate, not an assertion about final tagged or
published Release assets. [Raw samples](release-candidate-mcp.json) retain all
20 requests and the client executable's hash. Older [local-build results](mcp.md)
remain separate.

An existing compiled `mcp_latency` debug example used the official rmcp client
to spawn the actual candidate over stdio and complete protocol initialization.
It issued 20 bounded searches for `calculate` against a fresh external index of
the original public Python fixture. Every response was checked for a successful,
nonempty structured result. No model, project code execution, dependency
installation or client/server rebuild was involved.

| Measurement | Milliseconds |
|---|---:|
| Startup and protocol initialization | 321.86 |
| Search median | 9.65 |
| Search p95 (nearest rank, 20 samples) | 15.24 |

The Windows 11 / Ryzen 7 H 255 environment is recorded in the paired
[analysis measurements](release-candidate-analysis.md). All analysis and public
project scans had finished first; no build, renderer benchmark or native window
ran intentionally in parallel. Background OS activity was not isolated.

These are actual small-fixture transport and tool-call timings, including the
first request, JSON and protocol transport but excluding model latency. They do
not estimate large-repository query latency. CLI startup plus whole-graph loading
is a separate measurement, and the larger synthetic CLI searches remain slow.
The independent real-Agent eight-tool acceptance check is documented separately
in [Agent interoperability](../development/agent-interoperability.md).

To reproduce without recompilation, use the existing trusted rmcp example:

```text
MCP_LATENCY_EXE CANDIDATE_CLI PUBLIC_PYTHON_FIXTURE EXTERNAL_INDEX NEW_REPORT.json
```

First index that fixture with the selected CLI. The example source is
`crates/glyphibe-cli/examples/mcp_latency.rs`; raw process logs from this run
remain outside the repository and are not distribution assets.
