# Chrono audit remediation

RUSTSEC-2026-0543 reports chrono as unmaintained, without a patched version.
The original mandatory chain was `glyphibe-cli -> glyphibe-mcp -> rmcp 3.9.0
-> chrono 0.4.45`, including schemars' `chrono04` feature. Updating to rmcp
3.11.0 alone is insufficient, but that does not rule out a bounded source fix.

Exact source inspection found that `Annotations::last_modified` is already
`Option<String>`. Only two convenience constructors take chrono types; Glyphibe
does not call them. The [licensed source backport](../../security/rmcp-backport.md)
keeps those constructors behind a default-on compatibility feature, replaces
internal clocks with already locked `time`, and preserves std duration/error
formatting. Glyphibe selects its original functional SDK features explicitly
while opting out of the unused typed constructors. This preserves public wire
fields, unmodified upstream JSON schemas, stdio APIs and functionality.

The shipping audit uses Cargo's actual normal/build target tree, with exact
package identities and original lock entries from metadata. Metadata alone can
retain disabled optional weak dependency edges: serde_with's inactive chrono
edge was observed even when its enabled features were only alloc/default/macros/
std and `cargo tree` and compilation showed no chrono. The collector now checks
the complete metadata/tree closure agreement and rejects missing or ambiguous
identities. A real Cargo fixture tests both disabled and explicitly enabled weak
optional dependencies. No package/advisory-specific exclusion is present.

All target audits still invoke `cargo audit --deny warnings`. The complete raw
lockfile ledger retains inactive optional chrono and unsupported-platform
proc-macro-error maintenance warnings. Neither warning is ignored or presented
as an upstream fix. Current results and bindings are in `audits/audit-refresh.json`.
Hosted product/native/schema/compatibility/GLib checks must pass on the final
source before merge. Local test success alone is not release acceptance.

References:
- https://rustsec.org/advisories/RUSTSEC-2026-0543.html
- https://github.com/chronotope/chrono/issues/1768
- https://github.com/modelcontextprotocol/rust-sdk/blob/rmcp-v3.11.0/crates/rmcp/Cargo.toml
- https://github.com/rust-lang/cargo/issues/10801
- https://doc.rust-lang.org/cargo/commands/cargo-tree.html
