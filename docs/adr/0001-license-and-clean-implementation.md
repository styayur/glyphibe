# ADR 0001: MPL-2.0 and original implementation

Date: 2026-10-27. Status: accepted.

The target repository was absent and contained no existing license. The owner
explicitly authorized a new public repository and proposed MPL-2.0.

Use SPDX identifier `MPL-2.0` for original Glyphibe sources and original
knowledge entries. File-level reciprocity supports open maintenance while
allowing separately licensed integrations. Keep the unmodified license text
and source availability notice with distributions. This decision follows the
[MPL text](https://spdx.org/licenses/MPL-2.0.html) and Mozilla's
[FAQ](https://www.mozilla.org/en-US/MPL/2.0/FAQ/).

Reference repositories are studied for public architecture and algorithms.
They are not vendored or assembled into duplicate analyzers. GitNexus source
is excluded: its noncommercial restrictions do not fit this project's intended
redistribution. Other reference licenses are recorded per pinned revision in
the research audit. Unknown licenses are a release blocker.

Dependencies must retain their own notices. License compatibility is checked
against the resolved Cargo/npm dependency graph, including build and native
components; accepting this ADR does not certify a dependency graph that has
not yet been resolved. Knowledge entries are original prose with primary
reference links and individual authorship/license metadata. Remote imports
remain local and are never silently redistributed as a default knowledge pack.
