# ADR 0003: Preserve native packaging and platform notices

Date: 2026-10-28. Status: accepted packaging policy; final release acceptance
requires separate artifact and installation evidence.

## Context

Cargo and npm inventories cover their resolved package graphs. Windows desktop
packaging also uses Rust standard-library code, installer components, a native
Tauri plugin, the WebView2 loader and Microsoft platform/runtime components.
Those need explicit provenance and original notices rather than an assumption
that the application's MPL-2.0 label covers them.

The inspected supplement is
[native-packaging-inventory.json](../development/native-packaging-inventory.json),
collected at `2026-10-28T06:02:41.180409+00:00`. The accompanying
[collector/checker](../../scripts/audit-native-notices.py) preserves ten original
notice files. This ADR and [PLATFORM_NOTICES](../../PLATFORM_NOTICES) are authored
indexes; they do not replace the original terms or certify legal compliance.

## Decision

Use the Windows x64 NSIS package with `bundle.windows.nsis.compression = "zlib"`
in [tauri.conf.json](../../apps/desktop/src-tauri/tauri.conf.json). NSIS 3.13's
zlib selection keeps the selected compressor under zlib/libpng terms. Upstream
[NSIS licensing](https://nsis.sourceforge.io/License) separately describes its
bzip2 and CPL-1.0 LZMA modules. We retain its complete COPYING file while choosing
zlib. This decision does not declare those alternative licenses incompatible;
it makes the shipped compressor choice explicit. A larger installer is an
acceptable tradeoff.

Keep native/toolchain notices separate from the Cargo/npm package inventory:

| Component | Recorded identity | Preserved evidence and scope |
| --- | --- | --- |
| Rust library | 1.98.1; commit `48a229ceaefd4985c50990b14116b6d856af0985` | Original `COPYRIGHT-library.html`, MIT and Apache-2.0 texts from the installed official distribution; platform-wide notice coverage |
| NSIS | 3.13; zlib selected | Original COPYING, Modern UI 2 and NSISdl notices |
| Tauri NSIS utilities | 0.5.3; commit `13d9edd27b69310e108d6fbd49f90992f8a05390` | Both MIT and Apache-2.0 files from pinned source; official prebuilt DLL identity |
| WebView2 loader | SDK 1.0.3800.47; `webview2-com-sys` 0.39.1 | Original SDK BSD-3-Clause LICENSE and complete NOTICE; archive and x64 static-library hashes |
| Microsoft runtime/platform | CLI dynamic CRT dependency; desktop static runtime support | External prerequisites and applicable Microsoft terms; no standalone Microsoft runtime DLL/installer release asset |

The Rust source's [license
description](https://github.com/rust-lang/rust/blob/48a229ceaefd4985c50990b14116b6d856af0985/README.md#license)
includes additional third-party terms. Preserve the complete library notice
collection instead of replacing it with a generic MIT/Apache statement. Its
platform-specific entries are not a binary composition or cross-platform
support claim.

## Plugin provenance and prebuilt limitation

The [official
release](https://github.com/tauri-apps/nsis-tauri-utils/releases/tag/nsis_tauri_utils-v0.5.3)
identifies source commit `13d9edd27b69310e108d6fbd49f90992f8a05390`. Its
`nsis_tauri_utils.dll` asset has size 34,304 bytes and publisher-reported SHA-256:

```text
5ba143b5db4a87d32d6e7802e033330aae56cbceabe0d1e3ba41948385ad4709
```

This matches the local identity recorded by the collector. The pinned
[source manifest](https://github.com/tauri-apps/nsis-tauri-utils/blob/13d9edd27b69310e108d6fbd49f90992f8a05390/Cargo.toml)
and inspected crate declarations identify source dependency families and version
ranges. They do not establish exact transitive versions, build flags or toolchain
inputs for the prebuilt artifact. No exact upstream binary SBOM or reproducible
binary comparison is asserted. The recorded digest matches the selected asset
identity in the publisher's record; it is not a claim of independent
build provenance or code-signing verification.

## WebView2 SDK and Runtime boundary

The [official SDK
package](https://www.nuget.org/packages/Microsoft.Web.WebView2/1.0.3800.47) archive
is recorded with SHA-256
`56c9f26bdd07916a2d1949fb58a5c7e434dfa1173577dca879206050c4e718db`.
Its x64 `WebView2LoaderStatic.lib` matches the crate's recorded library hash
`89c6b872783b8f6c3cedbff618adb42082d455c615453ae10cfc753f1e8f25d8`.
The acquisition record also notes comparison with the server's SHA-512 header.
That transport check is distinct from NuGet package-signature verification.

Preserve the SDK's BSD-3-Clause LICENSE and its entire NOTICE without translation
or selective removal. The NOTICE can describe package components beyond this
loader; preserving it does not assert that they are all linked into the app.

The separately provided WebView2 Runtime remains subject to Microsoft's own
terms and servicing. Microsoft documents its prerequisites and deployment in
[WebView2 distribution](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution).
The current configuration does not explicitly set `webviewInstallMode`; final
installer behavior and prerequisite acquisition require observation. The core's
offline operation is not evidence of offline installation on a clean machine.

## Microsoft runtime terms

The native inventory records a `VCRUNTIME140.dll` import for the CLI, with the
v14 x64 Runtime supplied separately, and Tauri static_vcruntime support for the
desktop. Static linkage does not change Microsoft's terms for those components.
The project does not add a separate Microsoft runtime DLL or installer to its
release assets.

For the hosted 2026 candidate, retain the applicable [Enterprise/Professional
2026 terms](https://visualstudio.microsoft.com/license-terms/vs2026-ga-pro-enterprise/),
[V14 Redistributable and Runtime 2026
terms](https://visualstudio.microsoft.com/license-terms/vs2026-ga-visualcpp-v14-redist-runtime/),
[Visual Studio 2026 redistribution list](https://learn.microsoft.com/en-us/visualstudio/releases/2026/redistribution),
and [supported runtime
downloads](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist).
Microsoft's permitted redistribution depends on its license terms and the
distributor's entitlement. Those are not established merely by having an
installed compiler or an MPL application repository. Debug/nonredistributable
tool components must not be treated as application runtime assets.

Microsoft's [deployment documentation](https://learn.microsoft.com/en-us/cpp/windows/deployment-in-visual-cpp#static-linking)
describes static linkage as an available deployment method. Its recommendation
for dynamic libraries concerns servicing: static-runtime fixes require rebuilding
and redistributing the application. We found no general static-linking prohibition
in that guidance; the applicable license conditions and entitlement still govern.

### 2026 terms read and entitlement boundary

The official English [Enterprise/Professional 2026
document](https://visualstudio.microsoft.com/wp-content/uploads/2025/10/Visual_Studio_2026-License-Enterprise_Professional_ENU.docx)
and [V14 Redistributable and Runtime
document](https://visualstudio.microsoft.com/wp-content/uploads/2025/10/Visual-C-V14-License-Redistributable_and_Runtime_ENU.docx)
were read on 2026-10-28. Both identify their last update as October 1, 2025.
The Enterprise Distributable Code section permits listed object-code
redistribution subject to significant application functionality, protective
terms for distributors/external users, and its indemnification condition. It
restricts Unreleased Components, distribution of software developed using
Insiders Channel Software, misleading Microsoft branding, and making Microsoft
Distributable Code source subject to an Excluded License. Keeping Glyphibe's own
MPL-covered files separate does not place Microsoft component source under MPL.

The 2026 [permitted list](https://aka.ms/vs/18/redistribution) resolves to
Microsoft's 2026 redistribution page. It conditions redistribution on a validly
licensed product and excludes preview and debug/nonredistributable components.
The Runtime document grants installation/use; its scope restrictions do not
provide a separate redistribution grant. Microsoft's current v14 download
guidance explicitly covers VS 2017-2026 and requires a suitable architecture and
a runtime version at least as recent as the build tools. No runtime installer
or DLL is added to Glyphibe's release assets by this policy.

The document review records these conditions rather than certifying the
publisher's entitlement or asserting that all contractual obligations have been
independently verified. The final build record must identify the actual product
and channel; an Enterprise display name alone does not establish that the build
used the Stable channel or only released components.
The [Microsoft license directory](https://visualstudio.microsoft.com/license-terms/)
states that subscription purchases are governed by [Microsoft Product
Terms](https://www.microsoft.com/licensing/terms/productoffering/VisualStudioSubscriptions/all).
Reviewing the public Enterprise EULA does not identify or certify the hosting
provider's particular licensing agreement.

### Historical local 2022 audit

The earlier local audit on 2026-10-28 read the official [Build Tools, March 2024
update](https://visualstudio.microsoft.com/wp-content/uploads/2024/03/Visual-Studio-2022-Diagnostic-Build-Tools-Agent-License_Update-March-2024_EN.docx),
[Runtime 2015-2022](https://visualstudio.microsoft.com/wp-content/uploads/2021/09/Visual-C-Runtime-2015-2022-License-1.docx)
and [Community
2022](https://visualstudio.microsoft.com/wp-content/uploads/2021/11/Visual-Studio-2022-Community-License-EN.docx).
Build Tools' exception for required third-party OSI C++ dependencies was not
treated as a grant to develop all newly authored application code. The Runtime
installation/use terms were not treated as a redistribution grant.

Read-only local `vswhere` enumeration listed only
`Microsoft.VisualStudio.Product.BuildTools` (Visual Studio Build Tools 2022).
It did not list Community, Professional or Enterprise. Local installed Redist
documentation was also inspected and points to Microsoft's official permitted
list. These technical observations cannot establish whether the publisher has
some other valid entitlement, or certify fulfillment of the applicable grant's
conditions. The audit distinguishes reviewed source licenses from locally
unverified compiler-use and redistribution entitlement. This 2022 evidence is a
historical local observation, not an assessment or authorization of the actual
2026 hosted candidate. Neither reviewed environment identifies a general ban on
this MPL application using a static runtime; no unconditional permission is
claimed.

### Hosted release-build path

The existing `desktop` job in [ci.yml](../../.github/workflows/ci.yml)
uses a GitHub-hosted Windows runner and builds the NSIS package; the Rust jobs
also build the CLI. [GitHub's runner
reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
documents this supported build environment. The reviewed CI candidate at
`a34bd0669a0ad72fc5f1621c9842423b1d79dcd4` records:

| Candidate build field | Observed value |
| --- | --- |
| Runner image | `win25-vs2026` |
| Image version | `20261021.278.1` |
| Visual Studio product | Visual Studio Enterprise 2026 |
| Product ID | `Microsoft.VisualStudio.Product.Enterprise` |
| Installation version | `18.11.12315.225` |

The official [Windows 2025 with VS 2026 image
manifest](https://github.com/actions/runner-images/blob/main/images/windows/Windows2025-VS2026-Readme.md)
examined on 2026-10-28 corroborates that image and installed product version.
The older Windows 2025 manifest describing VS 2022 is not evidence of this
candidate's build inputs. Subsequent candidates require their own provenance.

Producing final tag assets through that existing workflow avoids treating an
unverified local compiler entitlement as release-build evidence. Record the
actual run's image identity, Visual Studio product and compiler versions, then
refresh notices and verify the resulting assets. Image manifests and `latest`
labels can change; this inspection is not evidence of the next run's inputs.
Use remains subject to [GitHub's applicable service
terms](https://docs.github.com/en/site-policy/github-terms/github-terms-for-additional-products-and-features)
and the software terms. An installed Enterprise product on a hosted image does
not certify an individual's license rights or waive redistribution conditions.

## Packaging layout and checks

The Tauri resource mapping requests the following installed layout:

| Repository resource | Requested destination |
| --- | --- |
| `LICENSE` | `licenses/LICENSE` |
| `THIRD_PARTY_NOTICES` | `licenses/THIRD_PARTY_NOTICES` |
| `PLATFORM_NOTICES` | `licenses/PLATFORM_NOTICES` |
| `third-party/licenses/` | `licenses/third-party/licenses/` |
| `third-party/toolchain-licenses/` | `licenses/third-party/toolchain-licenses/` |

This is configuration intent. Confirming that the final installer and installed
application contain these bytes is a separate release check.

`python scripts/audit-native-notices.py` is a read-only check by default. It
compares the notice set and recorded SHA-256 values, checks zlib configuration,
and constrains the Tauri CLI and WebView2 binding versions. Collection additionally
checks the installed NSIS version, pinned plugin digest, supplied SDK archive
and loader-library bytes, and captures Rust distribution notices.

The default check does not attest the current Rust compiler or NSIS installation,
recheck a final executable's import table, validate package signatures, prove
plugin build reproducibility, or inspect a completed installer. Record the actual
release tool versions, build logs, final hashes, import/prerequisite observations,
installed notice files and remote-asset verification separately. Changes to the
toolchain, plugin, loader, compressor or packaging mode require refreshed
evidence and review rather than reusing a previous successful notice check.

## Consequences

Recipients receive original native notices alongside the source dependency
notices. Maintainers can identify the chosen assets and the limits of their
provenance. This preserves 1,557,958 bytes of original native notice material and makes
the prebuilt-plugin SBOM gap visible. The policy provides auditable evidence;
it does not turn incomplete build provenance into a complete compatibility or
compliance guarantee.
