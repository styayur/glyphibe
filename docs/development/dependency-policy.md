# Dependency evidence and release policy

Research date: 2026-10-27. Dependency evidence refreshed on 2026-10-28. A successful compiler or audit process
exit code is not a license opinion or proof of absence of vulnerabilities.

## Scope and reproducibility

`scripts/audit-dependencies.py` uses Python's standard library and Cargo's
`metadata --locked --format-version 1` for both workspace entry points without
a target filter. It reads the active workspace lockfile, then inventories every
entry in the frontend npm lockfile, including optional platform packages and
development/build tools. Disabled Cargo features are not claimed to ship.
Dependency edges and package checksums are exported as SPDX 2.3 JSON. Internal
absolute source/cache paths are excluded from published reports.

```text
python scripts/test-audit-dependencies.py
python scripts/audit-dependencies.py --fetch-missing-license-texts
python scripts/audit-dependencies.py --check
```

The refresh command may download locked package sources via Cargo, inspect
integrity-verified npm archives, and retrieve omitted notice files at an
immutable published VCS commit. It never installs packages, runs package
scripts, extracts archive members into the source tree, or executes downloaded
content. `--offline` disables remote recovery. `--check` verifies the committed
inventory against current manifest/lockfile hashes, SPDX policy and every preserved
notice hash without changing files. Missing notices, unknown/restricted license
expressions and stale inventories cause failure.

Manifest and lock hashes normalize CRLF to LF to remain stable across Git
checkouts on Windows and Unix. Original notice files retain exact byte hashes;
Git attributes disable text conversion for those files. Generation aborts if
dependency inputs change during collection.

Artifacts:

- [dependency-inventory.json](dependency-inventory.json): versions, scope,
  platform predicates, declared/selected licenses, checksums, source provenance
  and unresolved failures.
- [dependencies.spdx.json](dependencies.spdx.json): machine-readable package
  identities and dependency relationships.
- [THIRD_PARTY_NOTICES](../../THIRD_PARTY_NOTICES): per-package source and
  preserved notice locations.
- `third-party/licenses/`: exact collected license/NOTICE text, deduplicated by
  SHA-256 while retaining each package's original source location.

The final local refresh after the MSRV and frontend dependency updates contains
**760 packages (512 Cargo, 248 npm), 432 referenced notice texts, and zero
unknown/disallowed license or missing-notice failures**. The referenced text
occupies 1,393,250 bytes. `--check` passed against the final input hashes. These
figures describe this dependency snapshot and must be regenerated after changes.

Some upstream packages omit a file named LICENSE. Recovery is recorded rather
than hidden: original AUTHORS files and complete license comments are retained;
Cargo omissions can use the published immutable VCS commit; explicit README
copyright/license declarations and the MPL Exhibit A header are paired with
canonical SPDX terms. Canonical terms are labeled as supplements, not invented
upstream files. Reviewed generated native wrapper families require an exact
parent optional-dependency release pin and identical declared licenses; generic
dependency license inheritance is prohibited. Rolldown's bundled
THIRD-PARTY-LICENSE is retained with its native wrappers. These recovery methods
do not import implementation code or change an upstream license.

## License decisions

The parser accepts SPDX AND/OR/WITH/parentheses and explicitly records Cargo's
legacy slash-to-OR normalization. An OR branch must have a permitted option;
every member of an AND expression must be permitted. Unknown atoms and
noncommercial licenses fail closed. Only the Apache-2.0 LLVM exception is
currently accepted. A package name containing MIT does not satisfy the policy.

The allowlist covers the resolved permissive license families, MPL-2.0,
Unicode data, Python-2.0, CDLA-Permissive-2.0 and CC-BY-4.0. Their obligations
remain distinct. Preserve copyright and license notices; preserve Apache
NOTICE material and patent terms where applicable; retain Unicode/data license
texts; retain creator/source attribution for CC-BY-4.0; make covered MPL source
and modifications available to recipients. Choosing MIT from a valid OR
expression does not choose a stronger copyleft alternative. No GPL/LGPL-only,
noncommercial, proprietary, missing or invented license is automatically
approved.

This follows Mozilla's description of [MPL file-level obligations and source
availability](https://www.mozilla.org/en-US/MPL/2.0/FAQ/), the [SPDX expression
identifiers](https://spdx.org/licenses/), and the actual distributed terms of
[CDLA-Permissive-2.0](https://spdx.org/licenses/CDLA-Permissive-2.0.html),
[Python-2.0](https://spdx.org/licenses/Python-2.0.html), and
[CC-BY-4.0](https://spdx.org/licenses/CC-BY-4.0.html). The policy is an engineering
filter and evidence collector. It does not establish a blanket compatibility
claim: release review must inspect embedded notices, per-file exceptions and
actual bundled binaries. Package-manager metadata does not expose all libraries
inside a prebuilt tool binary. The inventory distinguishes build/dev tools from
the product's actual binary composition.

The default knowledge registry is separate: every original concept and pattern
records MPL-2.0 authorship and reference-only sources. Imported remote knowledge
remains untrusted local-only data pending document-specific license review.

## Vulnerability evidence by target

`cargo audit 0.22.2` supports `--target-os` and `--target-arch`, but those options
filter advisory metadata, not the workspace's target-specific dependency graph.
Checking an all-target lockfile with `--target-os windows` therefore does not
establish which crates enter a Windows executable.

`scripts/audit-target-dependencies.py` asks Cargo for a platform-filtered graph,
walks normal/build edges from the named package, and creates a temporary lockfile
preserving exact versions, sources and checksums. It then runs
`cargo audit --deny warnings` on that closure with no advisory ignore list.
Development-only edges are excluded from this shipping-graph check and remain
visible in the full dependency inventory. This is a dependency applicability
check, not a successful cross-platform build claim.

Five focused Python regression tests validate restrictive-license rejection,
SPDX alternatives/conjunctions, notice-only source collection, unpinned or
unapproved remote source rejection, and line-ending-stable source hashes. They
run through `python -m unittest discover -s tests -p test_dependency_policy.py`
as well as the standalone script above.

```text
python scripts/audit-target-dependencies.py --package glyphibe-desktop --target x86_64-pc-windows-msvc --output artifacts/audit-desktop-windows.json
python scripts/audit-target-dependencies.py --package glyphibe-cli --target x86_64-pc-windows-msvc --output artifacts/audit-cli-windows.json
python scripts/audit-target-dependencies.py --package glyphibe-cli --target x86_64-unknown-linux-gnu --output artifacts/audit-cli-linux.json
python scripts/audit-target-dependencies.py --package glyphibe-cli --target x86_64-apple-darwin --output artifacts/audit-cli-macos.json
```

Actual local results, using RustSec database commit
`99670cfc0bc574433034b75876a4b791ccaa2efb` (1,380 advisories):

| Selected graph | Packages | Vulnerabilities | Warnings | Result |
| --- | ---: | ---: | ---: | --- |
| Windows x64 desktop | 328 | 0 | 0 | Passed |
| Windows x64 CLI | 211 | 0 | 0 | Passed |
| Linux x64 CLI | 204 | 0 | 0 | Passed |
| macOS x64 CLI | 205 | 0 | 0 | Passed |

Machine-readable records are in [audits/](audits/). The full all-target audit
reported zero vulnerability entries but **two informational warnings**, which
are not silently suppressed:

- [RUSTSEC-2024-0370](https://rustsec.org/advisories/RUSTSEC-2024-0370.html):
  `proc-macro-error` is unmaintained and has no patched release. It is pulled
  through the Linux GTK desktop dependency chain, not the four selected graphs.
- [RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html):
  GLib 0.15–0.19 has unsound VariantStrIter operations; the advisory identifies
  fixes from GLib 0.20. The Tauri GTK3 path uses an affected GLib line. Linux
  desktop support is not cleared by the Windows or CLI checks and must resolve
  this dependency or independently establish an appropriate supported path
  before a Linux desktop support claim.

An initial frontend audit found two critical and one moderate advisory in the
Vitest 3 toolchain. After updating to Vitest 5/Vite 8, a fresh local npm audit
returned **zero advisories** and exit code 0 again on 2026-10-28. The current npm
report, not this historical observation, determines its gate.
No vulnerability warning is converted into a successful compatibility statement
merely because the default audit command returned zero.

The Cargo/npm inventory is supplemented by [PLATFORM_NOTICES](../../PLATFORM_NOTICES)
and the [native packaging inventory](native-packaging-inventory.json). The latter
records the actual Rust distribution, NSIS compressor, Tauri installer plugin
and Microsoft WebView2 loader used by the Windows package. Ten original upstream
notice files are retained separately under `third-party/toolchain-licenses/`;
`python scripts/audit-native-notices.py` verifies their exact bytes and reviewed
configuration without downloading or running analyzed code. See
[ADR 0003](../adr/0003-native-packaging.md) for scope and the prebuilt plugin's
explicit build-SBOM limitation. Platform prerequisites retain their own terms.
