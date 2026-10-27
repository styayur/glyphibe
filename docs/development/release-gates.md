# v1.0.0 release gates

Status on 2026-10-27: engineering has started. **No release is approved.**

Each gate requires an evidence file, command/log or CI job. Empty cells mean
unverified, never passed. This ledger will be updated as implementation lands.

| Gate | Status | Evidence |
| --- | --- | --- |
| Offline no-LLM core | Pending | |
| Python/Rust/JavaScript/TypeScript/TSX parsing | Pending | |
| Dual-plane six-level many-to-many mapping | Pending | |
| Structural clusters and evidence-backed components | Pending | |
| Calls, trace and impact queries | Pending | |
| Independent licensed knowledge registry | Pending | |
| Bilingual progressive Socratic teaching | Pending | |
| Frontier/Dense/Cluster/Galaxy real graph interaction | Pending | |
| Glass overlays, native titlebar and fallback | Pending | |
| Sliders, keyboard, accessibility and synchronized state | Pending | |
| Standalone CLI | Pending | |
| Real MCP client interoperability | Pending | |
| Hostile repository and knowledge-source boundaries | Pending | |
| Rust/frontend/integration/E2E and CI | Pending | |
| Reproducible performance measurements | Pending | |
| Windows install/upgrade/uninstall/reinstall lifecycle | Pending | |
| Resolved dependency and knowledge license audit | Pending | |
| English/Chinese documentation and genuine screenshots | Pending | |
| Release asset re-download and checksum verification | Pending | |

## Environment observations

- Windows local-first workspace. Rust stable MSVC 1.98.1, Node.js 26.7.0,
  npm 11.19.0, Git 2.55.0.windows.3 were observed.
- Target directory and remote repository did not exist before this task.
- D: had approximately 1.4 GB free at initial inspection. C: had approximately
  35 GB free. Build/cache output must use a task-specific C: directory.
- GitHub CLI authenticated as the requested owner. No credentials are recorded.
