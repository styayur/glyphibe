# Picomatch local security backport

GHSA-fc5q-39xc-vjpf affects registry releases through 4.0.7 with no patched release as inspected 2026-11-18. The upstream package remains MIT picomatch 4.0.7, with unchanged package identity, lockfile tarball integrity, original license and source-file hashes in picomatch.json. No alert is dismissed or hidden.

The locally authored MIT helper extends the upstream repeated-extglob risky-pattern literalization policy to branches containing unescaped, unquoted wildcard stars. It conservatively covers nested wildcard bodies too. Ordinary globs, literal repetition, bracket classes, nested literal alternatives and the matcher APIs used by build tooling are tested. The existing upstream maxExtglobRecursion:false escape hatch is not used by this project.

npm ci applies the patch only to exact original parser bytes; an unknown version, changed executable file, missing/extra installation, changed lock integrity or runtime matcher fails closed. The security tests run original exploit reproduction with a 1.5-second deadline, patched exploit and compatibility cases with a 3-second deadline, parser-tampering rejection and new-advisory/runtime-advisory/audit-error rejection. Every executable upstream file, package manifest and original MIT license is hash bound.

security:audit retains the complete raw npm v2 audit in artifacts/npm-audit.json, verifies installed patched copies first, and reports VERIFIED_LOCAL_BACKPORT rather than claiming a clean registry audit. Only the exact advisory and its dependency consequences in development-only packages can be remediated by this evidence. Any other advisory, incomplete report, unavailable query or runtime finding fails. CI uploads the raw finding for review. This replaces version-only vulnerability decisions with verified source-remediation evidence; it does not waive the advisory or change severity thresholds for any unresolved finding.

Reachability is in repository/configuration glob patterns consumed by build and test tooling; no application feature directly imports picomatch. No attacker-controlled pattern endpoint has been demonstrated. Removing one tool would leave the other consumers. Remove the local patch only after an upstream fix, fresh raw audit, glob compatibility and the full native/browser suite pass.

Sources: https://github.com/advisories/GHSA-fc5q-39xc-vjpf and https://github.com/micromatch/picomatch/blob/6bb40679c218cefba5d4e9662408c5fdf699a8bb/lib/parse.js . Upstream PR 194 alone is not a fix for +(*a).

## 2026-11-19 nested-group correction

The earlier parser hash 96e3eef5f17187b9b926e793283effe864e009698c9ac21c4a3bc0c4a134a51b is incomplete: actual installed copies time out on +(@(*(a))) against 96 a characters plus !. Additional probing found +((a|aa)) and +((a)+) also time out. Earlier remediation wording is superseded by this correction; the raw advisory stays open/visible.

The corrected guard rejects non-plain nested groups inside repeated extglobs, while preserving the upstream safe single-character flattening path, including +(*(a)|*(b)). Existing compatibility cases remain. Permanent regression covers the reported examples plus 100 wrapper combinations across four nesting depths within a three-second child-process limit. The bound parser hash is 8177bbeab0ac3f0b55cdef447134e9d97225c71bff17b0a4a98b5a80091539b4. This is local code remediation evidence, not a claim of an upstream patched release or clean raw audit.
