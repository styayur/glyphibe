# Desktop workflow

Open a source folder and inspect its actual file progress and diagnostics.
No Git repository is required. `glyphibe-desktop --repo PATH` also opens an
explicit local directory. Reopen a folder to reuse its external content cache;
changed files are reparsed and relationships are rebuilt.

Search or switch architecture/explanation planes in the left island. Selecting
a result updates its level. The right Evidence panel shows provenance, source
ranges, relationships and mappings. Following a mapping changes plane and level
together. Structural parents and children are graph links, not an enforced tree.

Frontier shows a local neighborhood. Dense shows the chosen level with a density
cap. Cluster projects communities and directory components, aggregating canonical
dependency evidence for display. Galaxy uses a Three.js 3D scene with rotation,
zoom, focus and node selection, bounded to 2,000 visible nodes. Unsupported or
slow graphics have fallbacks; display projections never add facts.

The bottom controls independently select six code levels, four explanation
depths and density. Evidence controls dependency direction and hops, displays
counts and paths, and highlights possible effects. Reachability is not risk.

Source text is available for a live desktop repository. Changed fingerprints
require a rescan before old source ranges can be trusted. Imported graphs retain
locations without granting arbitrary filesystem access. Learn offers questions,
hints, answers and review. Knowledge shows original concepts, prerequisites
and attribution; business interpretations without evidence remain uncertain.

Pin or collapse each island; unpinned islands hide on pointer exit. F hides
nonessential chrome, Esc restores it, Ctrl+K searches and Ctrl+O opens. Controls
are keyboard accessible. Choose dark/light or opaque panels; system reduced
motion is respected. Native Mica is attempted with ordinary background fallback.

The browser imports `glyphibe export` JSON; it does not scan folders. Full
source-backed teaching uses the desktop Rust engine. Verification reports
record genuine screenshots and platform checks separately from builds.
