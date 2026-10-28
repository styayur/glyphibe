# Real Agent interoperability check

On 2026-10-28 at 08:22 Asia/Singapore, Codex CLI 0.159.2 connected to the actual
Glyphibe stdio MCP server and successfully called all eight required read tools.
The [sanitized machine-readable record](agent-interoperability.json) captures
the client, executable SHA-256, outcomes, fixture integrity and gate checks.

The executable was the optimized development CLI, SHA-256
`61b9186f88bd90f67df403cba7d15114e94b2b8c91aaf421dc1e6457d45f25a8`.
Observed successful tools were `architecture`, `search`, `node`, `trace`,
`impact`, `mappings`, `explain` and `knowledge`. All eight returned completed,
non-error results. No shell, file-editing, browser or other-server operations
were observed. The authored public Python fixture's files were unchanged.

`scripts/verify-agent.ps1` uses an ephemeral client invocation, ignores user
configuration while retaining existing authentication, specifies no model,
selects a read-only sandbox and allows only the eight selected server tools.
Its pass condition checks distinct successful tool names and results, rejects
other operation types, verifies fixture hashes and enforces a 180-second client
timeout. Timed-out runs terminate only their own started process tree.
Per-run raw JSONL/stderr remain in an external private cache; they are not
published with the repository.

The server configuration follows the
[official OpenAI MCP documentation](https://learn.chatgpt.com/docs/extend/mcp?surface=cli).
The same server also passes the real Rust SDK stdio integration test, including
invalid inputs, disabled proposal writes, stale-source refusal and historical
removed-source handling. This client check validates interoperability; it does
not establish arbitrary-client compatibility, real-project semantic accuracy or
release readiness of the native installer.

The external Agent uses an online authenticated model service for this test.
Glyphibe's parser, graph, query, knowledge and template teaching core do not call
that service, need an API key or depend on the Agent being available.
