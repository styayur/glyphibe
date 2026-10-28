# Galaxy pointer selection regression

On 2026-10-28 the installed Windows WebView2 application rendered and rotated
its real 3D graph, but repeated sphere clicks did not update canonical selection.
An added production-preview browser test reproduced this without native input
automation: move one pixel between pointer down and pointer up. The renderer's
upstream interaction layer classifies any mouse movement as dragging and
suppresses its node-click callback. A stationary browser click passed; the
one-pixel test failed because the details panel never opened.

Glyphibe now tracks the maximum movement during the entire pointer gesture.
Up to four CSS pixels is a click; larger movements remain orbit/pan gestures.
For a click, Three.js casts a ray from the public camera through the actual
canvas bounds and selects the nearest intersected rendered sphere. Coordinates
come from the same fixed layout supplied to the renderer. Explicit node value
1 and radius 5 keep visual geometry and selection geometry consistent. No
private renderer metadata or hover-cache timing is required. Three.js remains
loaded only for Galaxy. The original callback remains a fallback, with the
same pointer event deduplicated.

The complete production-preview suite subsequently passed **eight tests**,
including the failing one-pixel regression and a check that orbiting preserves
the original selection. Fifteen frontend unit tests also passed. Native
revalidation of the rebuilt installer is a separate release gate; a browser
test alone does not establish an installed application's behavior.

The rebuilt installed application subsequently selected `pipeline` from a real
sphere click and updated the shared details/highlight state. That native check
confirmed the pointer repair. It also exposed a separate initial camera issue:
entering Galaxy with no selected entity could leave an oversized sphere filling
the viewport. The previous camera setup relied on callback/ref timing during
lazy renderer creation. A small lazy `GalaxyCanvas` adapter now reports when its
imperative API is committed; the parent then explicitly fits the initial graph
or focuses the current entity. Later engine-stop fitting remains enabled.

The browser suite includes a source-derived 17-function projection and checks
that it is fitted rather than dominated by a viewport-sized sphere. This browser
check also passed before the explicit readiness repair and therefore does not
reproduce or prove the native timing fix. All eight E2E and 15 unit tests passed
after that repair; the final installer must still be checked on Windows.

The native debug recheck showed that renderer readiness alone was insufficient:
the first viewport still showed an oversized sphere. The current implementation
therefore frames the bounding sphere of Glyphibe's own fixed layout, including
node radius, using the camera's vertical field of view and viewport aspect.
This uses ordinary perspective geometry rather than the renderer's transient
object bounding box. It runs after readiness, engine updates and viewport size
changes. An independent frustum test checks sphere extents in narrow/square/wide
viewports and invalid inputs; the frontend unit suite now contains 16 tests.
Native validation of this additional change remains necessary.

The subsequent native debug run did show all 17 initial entities within the
viewport, selected `doubled` by clicking its sphere, and returned to a correctly
fitted graph after choosing the public Python directory in the actual Windows
folder dialog and rescanning it. This confirms the owned-layout camera change
in the native host. Optimized installer lifecycle verification remains separate.
