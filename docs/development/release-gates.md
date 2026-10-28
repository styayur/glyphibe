# v1.0.0 release gates

Prepublication evidence snapshot on 2026-10-28: implementation is in
[PR #1](https://github.com/styayur/glyphibe/pull/1). This checked-in ledger records
the source-review checkpoint; it is not a live publication-status service.
Completed per-asset acceptance and transport verification are recorded in
[GitHub Release notes](https://github.com/styayur/glyphibe/releases).
A package version or a green CI run alone never approves Stable publication.

Each gate requires an evidence file, command/log or CI job. Empty cells mean
unverified, never passed. This ledger will be updated as implementation lands.

| Gate | Status | Evidence |
| --- | --- | --- |
| Offline no-LLM core | Locally verified | CLI integration runs without keys and with unreachable network proxies: `crates/glyphibe-cli/tests/cli.rs` |
| Python/Rust/JavaScript/TypeScript/TSX parsing | Locally verified | `crates/glyphibe-parser/src/tests.rs`; [47 annotated basic call pairs](../benchmarks/analysis.md) |
| Dual-plane six-level many-to-many mapping | Locally verified | Core graph and explanation tests; [model](../architecture/graph-model.md) |
| Structural clusters and evidence-backed components | Locally verified | Deterministic Louvain/resolution and SCC tests in `glyphibe-analysis`; directory evidence remains separate from communities |
| Calls, trace and impact queries | Locally verified | CLI/SDK integration; nine [source-scope impact regressions](../architecture/impact-scope.md) |
| Independent licensed knowledge registry | Locally verified | 34 original bilingual concepts, ten patterns, prerequisite validation and per-entry attribution |
| Bilingual progressive Socratic teaching | Locally verified | Seven explanation tests; questions, hints, answers and review with independent depth/level |
| Frontier/Dense/Cluster/Galaxy real graph interaction | Browser and recorded native candidates verified | Nine production-preview E2E scenarios; installed Galaxy framing/sphere selection and corrected Dense/Frontier placement passed; [native record](native-validation.md), [2D repair](2d-framing.md) |
| Glass overlays, native titlebar and fallback | Browser and native interaction verified | E2E collapse/pin/focus/theme controls; actual installed glass islands, Focus/Escape, light theme and opaque fallback exercised; Mica attempted by the Tauri host, compositor effect not separately instrumented |
| Sliders, keyboard, accessibility and synchronized state | Browser verified; recorded native controls exercised | 24 frontend tests and nine E2E scenarios including six Cluster levels, unobscured Frontier pixels and 2D/3D selection; [snapshot isolation](snapshot-isolation.md); native independent controls and bilingual learning exercised |
| Standalone CLI | Locally verified | Release executable reports `glyphibe 1.0.0`; analyze/graph/search/trace/explain/impact/kb/export/doctor/MCP integration |
| Real MCP client interoperability | Verified on hosted candidate and development binaries | [Actual Agent reports](agent-interoperability.md): all eight read tools succeeded on the hosted `0f1e4b6` candidate; SDK integration additionally covers pagination, proposals and stale source |
| Hostile repository and knowledge-source boundaries | Locally verified | [Boundary review](boundary-review.md), malformed-source, URL, symlink, cache ownership and proposal-isolation regressions |
| Rust/frontend/integration/E2E and CI | Local checks and corrected-candidate CI passed | Windows full workspace: 78 passed, one explicit network test ignored; strict full-workspace Clippy and format passed; changed explanation crate: seven passed; frontend: 24 passed; production E2E: nine passed; Python policy/distribution checks: 13 passed. [Six-job hosted CI on 0f1e4b6](https://github.com/styayur/glyphibe/actions/runs/40683111999); final documentation/test additions require fresh CI before merge |
| Reproducible performance measurements | Measured with stated limitations | Hosted-candidate [analysis](../benchmarks/release-candidate-analysis.md), [public projects](../benchmarks/release-candidate-public-projects.md), [MCP](../benchmarks/release-candidate-mcp.md) and [rendering](../benchmarks/rendering.md); full-graph persistence/loading remains slow; native GPU-presented FPS unverified; software-rendering recommendations are not established |
| Windows install/upgrade/uninstall/reinstall lifecycle | Hosted candidate passed; final main-package acceptance follows source freeze | [Actual native record](native-validation.md): development-baseline upgrade, launch, reopen, uninstall/registry removal, reinstall and 471 original legal files checked; final package hash/results belong in the Release notes |
| Resolved dependency and knowledge license audit | Source audit and hosted build checks passed | [Policy](dependency-policy.md), 760-package inventory, SPDX and 432 referenced original notice texts; ten [native supplement](native-packaging-inventory.json) files; shipping target audits report zero advisories; stable VS Release channel, native notices and private-path scans passed; nonshipping GTK warnings and platform terms documented |
| English/Chinese documentation and genuine screenshots | Present and genuinely captured | Both READMEs, installation/user guides and actual installed [source](../images/native-source.png), [Galaxy selection](../images/native-galaxy.png) and corrected [Frontier](../images/native-frontier.png), showing only the original public fixture |
| Release asset re-download and checksum verification | Post-build acceptance required | [Release procedure](releasing.md), `scripts/verify_release_assets.py`; exact final source, asset hashes and fresh download execution results must be recorded in the Release notes before Stable publication |

## Environment observations

Final distribution candidates must come from the recorded hosted Windows build.
Earlier local optimized executables contained private user-directory diagnostics
and are excluded from release inputs. The build script remaps Rust prefixes,
isolates native C source caches and scans both payloads and the installer.
Complete candidate assets bind source commit, binary and installer hashes to
sanitized build provenance. None of these checks replaces actual installation.

- Windows local-first workspace. Rust stable MSVC 1.98.1, Node.js 26.7.0,
  npm 11.19.0, Git 2.55.0.windows.3 were observed.
- Target directory and remote repository did not exist before this task.
- D: had approximately 1.4 GB free at initial inspection. C: had approximately
  35 GB free. Build/cache output must use a task-specific C: directory.
- GitHub CLI authenticated as the requested owner. No credentials are recorded.
