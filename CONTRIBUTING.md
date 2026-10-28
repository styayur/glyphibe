# Contributing

Glyphibe is under active development; Stable is gated by explicit evidence.
Start with an issue and a minimal public example for parser defects. Do not
include private source, secrets or absolute user paths.

Install Rust 1.98.1 with MSVC Build Tools on Windows, and Node.js 24 or newer.
Clone this repository, run `cargo test --workspace --exclude glyphibe-desktop`,
then run `npm ci` in `apps/desktop`. For the desktop build, install the platform
prerequisites in the [Tauri guide](https://v2.tauri.app/start/prerequisites/).
Build artifacts may be redirected using `CARGO_TARGET_DIR`.

Before a PR, run `cargo fmt --all`, Clippy with `-D warnings`, relevant Rust
tests, and frontend typecheck/lint/test/build. Graph changes require a real
public source fixture and expected relationships, including negative examples.
UI changes require graph E2E coverage and genuine runtime screenshots where
useful. Tests must check behavior, not merely repeat implementation details.

Keep changes focused. Original contributions use MPL-2.0 and SPDX headers.
Document dependency licenses and knowledge authorship. Never copy restricted
reference implementations or unlicensed documentation. Do not claim dynamic
or inferred relationships as independently verified runtime facts.

See [architecture decisions](docs/adr/), [research](docs/research/) and
[release gates](docs/development/release-gates.md).
