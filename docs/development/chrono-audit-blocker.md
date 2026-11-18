# Current target-audit blocker

RustSec RUSTSEC-2026-0543, issued 2026-11-17, reports chrono as unmaintained,
with no patched or unaffected version. This is an informational maintenance
warning, not a claim of a specific exploitable CVE. The shipping-closure audit
deliberately uses `--deny warnings`, so it blocks publication and merging even
though the audited target graphs contain zero security vulnerabilities.

The actual dependency trace is `glyphibe-cli -> glyphibe-mcp -> rmcp 3.9.0 ->
chrono 0.4.45`, also enabled through rmcp's schemars chrono support. rmcp uses
chrono in public annotation types, service timeout formatting and task dates.
It is a required platform dependency, not a removable default feature. Current
upstream rmcp 3.11.0 still requires chrono and enables `schemars/chrono04`;
blindly updating that SDK does not remove the warning.

The available paths are an upstream SDK migration to a maintained date library,
or a separately reviewed SDK fork/migration that preserves public JSON schema,
RFC3339 dates, timeout behavior and MCP compatibility. Replacing the source
graph with a hand-filtered lockfile, ignoring the warning, or changing the
configured target-audit policy is not an implemented remediation. The current
policy remains intact. See the retained raw ledgers and `audit-refresh.json`
for actual exit codes, target closures and input hashes.

Sources:
- https://rustsec.org/advisories/RUSTSEC-2026-0543.html
- https://github.com/chronotope/chrono/issues/1768
- https://github.com/modelcontextprotocol/rust-sdk/blob/rmcp-v3.11.0/crates/rmcp/Cargo.toml
