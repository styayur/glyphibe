# Glyphibe desktop development

This package contains the React graph canvas and the Tauri 2 desktop adapter. The native adapter calls the same Rust parser, clustering, explanation and impact-query code as the CLI/MCP interfaces. It does not start a web server in an installed application.

## Build and run

Use the repository's supported Rust/MSVC toolchain, Node.js and the locked npm dependencies. On Windows the native application uses the installed WebView2 runtime.

```powershell
cd apps/desktop
npm ci --ignore-scripts
npm run typecheck
npm run lint
npm test
npm run build
npm run tauri -- dev
```

`npm run dev` also opens a browser-compatible frontend at `http://127.0.0.1:1420`. The browser imports a real Glyphibe graph JSON export; it does not have filesystem or native analysis access. There is no sample-data fallback. The installed desktop application opens a repository with the native folder chooser, or an explicit launch argument:

```text
glyphibe-desktop --repo PATH
glyphibe-desktop --help
glyphibe-desktop --version
```

The folder is read locally. Its scripts, tests, package-manager hooks and executables are not run. Index files stay in Glyphibe's application cache outside the repository.

## Canvas and state

Frontier, Dense and Cluster project a shared canonical graph into Sigma 3.0.2/Graphology. Galaxy projects that same graph into a real Three.js 3D renderer. Canonical IDs preserve selection across modes and planes. Display aggregation is explicitly inferred and does not write new analysis facts.

Selection and impact highlighting reuse the 2D WebGL context. Presentation changes are batched with Sigma reducers; full renderer construction is reserved for a changed visible topology/label set or a 2D/3D transition. Source parsing is not triggered by view controls.

The native impact panel uses `glyphibe_core::Graph::impact` through a restricted command. The browser uses traversal of the imported graph because no Rust session is present. Native source reads accept a registered node ID, enforce repository containment, reject historical source ranges, and check the current file's content fingerprint. Explanations and guided lessons come from the shared Rust engine. Concept text is imported from the same original bilingual knowledge registry as Rust.

## Rendering limits

The density slider changes a visible projection, leaving facts intact. Dense is capped at 10,000 drawn nodes. Galaxy normally permits up to 2,000; automatic mode uses 500 for detected software renderers, 500 after observed cadence below 15 Hz, or 1,000 below 24 Hz. These are rendering policies, not code-analysis confidence scores. Users can explicitly choose full Galaxy detail or reduce density further. The status bar reports the effective cap.

The canvas also retains keyboard navigation through the entity list when graphics are unavailable. Reduced motion avoids animated focus transitions. Opaque panels provide a blur-free fallback. `Ctrl+K` focuses search, `Ctrl+O` opens a repository/import chooser, `F` enters focus mode and `Esc` restores controls. Every tool island can be pinned and collapsed.

Detected software renderers start with opaque panels to reduce compositing cost;
users can manually restore glass. Hardware renderers retain the glass default.
This policy does not establish the preferred interaction frame-rate target.

See [the rendering report](../../docs/benchmarks/rendering.md) for measured software-rendering limits. Headless Chromium callback cadence is not native GPU-presented FPS.

## Verification

The browser tests build and serve the production frontend on isolated port 1430, and import an actual graph produced by the Rust CLI from the original public fixtures. Set its location before running tests; the default is `tests/data/fixture-graph.json`.

```powershell
$env:GLYPHIBE_TEST_GRAPH = 'PATH-TO-EXPORTED-FIXTURE-GRAPH.json'
npm run test:e2e
```

The suite exercises actual WebGL pixel selection, pan/zoom, 3D orbit, shared selection, plane mappings, hierarchy, sliders, themes, focus mode, floating islands, software-renderer limits and malformed imports. It does not replace the installed-native lifecycle checks.

```powershell
Set-Location ../..
cargo test -p glyphibe-desktop
```

The native source-read regression uses real Tree-sitter analysis and incremental changes, rather than a mocked analyzer. See the root release evidence for actual installation, startup and packaging validation.

## Permissions and distribution

The Tauri capability grants event listen/unlisten, title-bar dragging and seven explicit application commands. The frontend has no shell, unrestricted filesystem or network plugin capability. Graph labels remain text; 3D HTML tooltip content is escaped. Graph imports validate enum values, bounded text, source spans and edge endpoints before rendering.

The Windows bundle includes the MPL-2.0 license, third-party notices and unchanged upstream license texts under `licenses/`. Bundle source paths are mapped to a stable installed layout. The original vector icon and its Windows regeneration script are in `src-tauri/icons/` and `scripts/generate-icon.ps1`.
