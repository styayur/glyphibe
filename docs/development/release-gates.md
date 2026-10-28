# v1.0.0 release gates

Status on 2026-10-28: candidate implementation is in [PR #1](https://github.com/styayur/glyphibe/pull/1).
**No Stable release is approved.** Native lifecycle, final CI and remote asset
verification remain publication blockers. A package version is not approval.

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
| Frontier/Dense/Cluster/Galaxy real graph interaction | Browser verified; native 2D framing repair pending | Nine production-preview E2E scenarios; actual installed Galaxy framing/sphere selection passed; [native findings](native-validation.md), [2D repair](2d-framing.md) |
| Glass overlays, native titlebar and fallback | Browser verified; native pending | E2E collapse/pin/focus/theme controls; Mica fallback implemented in Tauri host |
| Sliders, keyboard, accessibility and synchronized state | Browser verified; native partially verified | 24 frontend tests and nine E2E scenarios including six Cluster levels, unobscured Frontier pixels and 2D/3D selection; [snapshot isolation](snapshot-isolation.md); native independent controls and bilingual learning exercised |
| Standalone CLI | Locally verified | Release executable reports `glyphibe 1.0.0`; analyze/graph/search/trace/explain/impact/kb/export/doctor/MCP integration |
| Real MCP client interoperability | Verified on recorded development binary | [Actual Agent report](agent-interoperability.md); SDK integration additionally covers pagination, proposals and stale source |
| Hostile repository and knowledge-source boundaries | Locally verified | [Boundary review](boundary-review.md), malformed-source, URL, symlink, cache ownership and proposal-isolation regressions |
| Rust/frontend/integration/E2E and CI | Local checks passed; fresh CI required | Windows full workspace: 78 passed, one explicit network test ignored; strict full-workspace Clippy and format passed; changed explanation crate: seven passed; frontend: 24 passed; production E2E: nine passed; Python policy/distribution checks: ten passed. [Six-job hosted CI on a34bd06](https://github.com/styayur/glyphibe/actions/runs/40648297768) does not validate subsequent source changes |
| Reproducible performance measurements | Measured with stated limitations | [Analysis](../benchmarks/analysis.md), [public projects](../benchmarks/public-projects.md), [MCP](../benchmarks/mcp.md) and [rendering](../benchmarks/rendering.md); native GPU-presented FPS unverified; 5,000-node software 2D cadence below the suggested target |
| Windows install/upgrade/uninstall/reinstall lifecycle | First hosted candidate passed; final candidate pending | [Actual native record](native-validation.md): development-baseline upgrade, launch, reopen, uninstall/registry removal, reinstall and 471 original legal files checked |
| Resolved dependency and knowledge license audit | Locally verified; final build-host checks pending | [Policy](dependency-policy.md), 760-package inventory, SPDX and 432 referenced original notice texts; ten [native supplement](native-packaging-inventory.json) files; shipping target audits report zero advisories; nonshipping GTK warnings and platform terms documented |
| English/Chinese documentation and genuine screenshots | Present; final native picking capture pending | Both READMEs, installation/user guides and [actual installed source view](../images/native-source.png), showing only the original public fixture |
| Release asset re-download and checksum verification | Pending | |

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
