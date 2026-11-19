# rmcp 3.9.0 bounded source maintenance backport

This keeps Glyphibe on the same SDK/API generation. Updating to rmcp 3.11.0
alone does not remove its mandatory chrono dependency. The published 3.9.0
Apache-2.0 archive, original license, commit and every source byte are identified
in `rmcp-upstream-inventory.json`; modified payloads are separately bound in
`rmcp-provenance.json`, with a reviewable `rmcp-backport.patch`.

## Scope and compatibility

The only chrono-typed public APIs are `Annotations::for_resource` and
`Annotations::with_timestamp`. The stored `last_modified` field is already
`Option<String>`; neither its Serde representation nor its schema changes.
Glyphibe does not call these typed convenience methods. They remain available
under a new **default-on** `chrono` compatibility feature, preserving the
upstream SDK's default API. Glyphibe disables SDK defaults and explicitly
retains base64, macros, client, server and its two stdio/process transports.

Internal `with_timestamp_now`, task timestamps and request-state milliseconds
use the already locked maintained `time 0.3.55`; timeout errors use `std::time`.
UTC formatting preserves chrono's grouped AutoSi fractional digits and explicit
`+00:00` offset; milliseconds floor correctly before the Unix epoch. Timeout
formatting preserves zero, fractional seconds and the former overflow-to-zero
behavior. Removing schemars' unused `chrono04` feature does not change a public
schema because none of these model fields stores a chrono type.

The normalized Cargo manifest disables the published repository-hook installer
build script, which has no product/runtime role, and isolates upstream workspace
tooling with an empty workspace. No other SDK feature or implementation is
intentionally migrated. The original published manifest remains unchanged in
`Cargo.toml.orig` for provenance. This backport is qualified for Glyphibe's native
targets, not a general claim about untested downstream/WASM SDK consumers.

## Required verification

- `python scripts/verify-source-backports.py` verifies every vendored byte,
  declared changes and unchanged original license. Notice collection includes
  both path-patched third-party crates and identifies their modified source.
- `cargo test --locked --manifest-path security/rmcp-regression/Cargo.toml`
  checks annotations, live clocks, timeout display, request-state compilation,
  and both JSON-RPC schemas against **unchanged published upstream fixtures**.
- Repeat with `--features chrono` to compare the actual replacement clock code
  against the original formatter and exercise the original typed constructors.
  This optional test oracle is not part of Glyphibe's shipping feature graph.
- Existing workspace/clippy/real SDK stdio tests, native Windows desktop build,
  packaging and all unchanged target `cargo audit --deny warnings` gates apply.

The complete lockfile still records the disabled optional chrono compatibility
dependency. Its raw advisory remains visible in the all-target ledger. This is
an unused-feature removal from the actual shipping closure, not an advisory
ignore, version spoof, warning-policy change or upstream maintenance claim.
