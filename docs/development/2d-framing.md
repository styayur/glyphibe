# Two-dimensional camera framing

The installed candidate revealed a usability defect with a real two-function
Frontier: Sigma's whole-window fit put both functions behind the floating
tool islands. Correct node counts did not establish that the view was usable.

The canvas still fills the window. The 2D adapter measures the visible left,
right, top and bottom islands with their actual DOM rectangles, reserves a
margin, and fits a new graph projection into the remaining region. It uses
Sigma's public `graphToViewport`, `viewportToFramedGraph`, `resize`, and camera
APIs, so graph normalization and camera rotation remain Sigma's responsibility.
Resize and visibility observers update the available region when a window or
island changes. Focus Mode uses the larger region freed by hidden islands.

Fitting happens when the visible topology/layout or available region changes.
Selection, impact highlighting, themes and explanation content do not refit an
unchanged projection. A user can pan and zoom normally. At most 24 visible nodes
have persistent labels; display labels are limited to 34 Unicode characters,
while the entity list and detail panel retain the full text. The fit reserves
measured label width where space permits; very narrow regions prioritize visible
nodes over complete canvas labels. The 3D camera and sphere picking are unchanged.

## Verification on 2026-10-28

Type checking, lint and 24 unit tests passed. The complete production browser
suite passed all 9 tests with no skipped or flaky tests. Its new framing test
imports two original function nodes and their observed call edge from the real
Rust-exported fixture graph. It measures connected components in actual rendered
canvas pixels and checks that both are between the measured islands. It also
checks that impact highlighting preserves a user pan and Focus Mode increases
the fitted visible span. Independent geometry tests cover rotated cameras,
empty/single-node projections and a narrow region with a long label.

These tests run in Chromium with SwiftShader at a 1440 × 960 viewport. They prove
rendered placement and interaction in that environment; they do not establish
native GPU frame rate. The newly packaged Windows application still needs the
maintainer's native viewport recheck before release. See the separate
[rendering measurements](../benchmarks/rendering.md) for retained performance
limits.
