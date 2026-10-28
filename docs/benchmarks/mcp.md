# MCP stdio latency

> **Latest candidate:** see [hosted candidate MCP measurements](release-candidate-mcp.md).
> The numbers below describe an earlier local binary, not the current candidate
> or final tagged Release assets.

Measured on 2026-10-28 on the Windows 11 / Ryzen 7 H 255 environment described
in [analysis.md](analysis.md). The server was the optimized `glyphibe 1.0.0`
CLI, SHA-256 `d1538a0bb77aeb9bef7244d03851b17d8a600d63f777406f4e10517695fd20c1`.

The official rmcp client starts the actual server over stdio, completes its
handshake, then issues 20 bounded searches for `calculate` in the original
public Python fixture. Every response is checked for a successful, nonempty
structured result. No model is involved. [All samples](mcp-latency.json)
are retained, including the first request.

| Measurement | Milliseconds |
|---|---:|
| Server startup and protocol initialization | 448.87 |
| Search median | 9.25 |
| Search p95 (nearest rank, 20 samples) | 10.22 |

These are small-fixture measurements, including JSON/protocol transport but
excluding model latency. A concurrent desktop release build was active, and
background OS activity was not isolated. This is neither a large-repository
estimate nor evidence of a speedup over another build. CLI process startup and
whole-index loading on larger workloads are measured separately in analysis.md.

Reproduction, from the trusted Glyphibe checkout after indexing its fixture:

```text
cargo run --locked -p glyphibe-cli --example mcp_latency -- GLYPHIBE_EXE FIXTURE_PATH INDEX_PATH OUTPUT_JSON
```

The index must be outside the fixture. The example invokes only the selected
Glyphibe executable and does not execute analyzed project code.
