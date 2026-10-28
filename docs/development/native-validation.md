# Installed Windows validation

This records actual observations, not approval inferred from compilation.
Environment: Windows 11 Home x64 build 26200, Ryzen 7 H 255 / Radeon 780M,
24 GB RAM. The captured application viewport is 1435 × 900 logical pixels.
Only the project's original public Python fixture is shown in saved screenshots.

## First hosted candidate, 2026-10-28

Source: `a34bd0669a0ad72fc5f1621c9842423b1d79dcd4`.
[All six CI jobs passed](https://github.com/styayur/glyphibe/actions/runs/40648297768).
The authenticated Actions artifact ZIP matched GitHub's recorded digest:
`664835f397f38fdb9c8459b00bdfeae8b86f703eaebe825b285da2f07fa1ea23`.
Its nine top-level files passed inventory/hash/legal checks. The CLI actually
ran version/help/doctor and an offline public-fixture analyze/search.

| Item | Actual result |
|---|---|
| Unsigned NSIS installer | 6,219,608 bytes; SHA-256 `511a2ec47e4379423adf5eb2baa813661785576c1d316fd4ad8bca306f9291b0` |
| Portable desktop | SHA-256 `cb01bc1b60c9b13bb5307a8716a719f8ea935d8ada59a58f805cd148ef213c35` |
| Installed desktop | Version 1.0.0; SHA-256 `76bf81bbcb38b8d6777933c88aee63473f62a4391987240ac8b40f7326f3efaf` |
| Payload difference | Exactly the documented three-byte NSIS bundle-type marker |
| Original installed legal files | All 471 matched the authenticated license ZIP |
| Development baseline → candidate | `0.1.0-dev` → `1.0.0`, installer exit 0 |
| Bare launch and native folder picker | Opened the actual public Python directory; 71 entities, 17 L5 entities and 20 L5 relations |
| Galaxy | Initial 17-node framing correct; a real sphere click selected `floor` and updated source/impact state |
| Source | The Rust host returned the actual `helpers.py:14–15` excerpt |
| Teaching | Native question, hint, answer and review exercised; English and Chinese explanations displayed |
| Shared controls | L2 grain and professional explanation depth selected independently; Focus/Escape, light theme and opaque fallback exercised |
| Modes | Dense/Galaxy/Cluster showed 17 nodes; settled Frontier correctly showed the two-node calling neighborhood |
| Close and reopen | Reopened with `--repo`; real repository and 71 entities loaded after launcher exit |
| Uninstall | Exit 0; application executable and its registry entry removed |
| Reinstall | Exit 0; version 1.0.0 and all 471 legal files reverified |

This baseline is a development build created for testing, not a previous public
Stable release. These observations do not certify upgrades from other versions.
Native GPU-presented FPS and native mouse orbit timing were not measured; actual
browser E2E separately exercises orbit and pan.

## Findings requiring the next candidate

- The two Frontier nodes were behind the floating islands. Focus Mode exposed
  them, but the default view was unsuitable. The [2D framing repair](2d-framing.md)
  must be rechecked in the next installed package.
- Long facts preceded the first teaching question. Facts/limits are now
  collapsible so the progressive controls appear first. Repository/file
  diagnostics are explicitly scoped, rather than implying they concern the
  selected function. Incoming/outgoing relationship arrows remove ambiguity.
- The actual hosted image used Visual Studio Enterprise 2026. Platform references
  were updated; final build provenance must additionally establish Release
  channel and `isPrerelease: false` before publication.

These repairs and the final source/installer hashes require fresh CI and native
validation. No Stable approval or remote Release verification is asserted here.

## Corrected hosted candidate, 2026-10-28

Source: `0f1e4b64acf46752349029bbba4bf265efb13c6f`.
[All six fresh CI jobs passed](https://github.com/styayur/glyphibe/actions/runs/40683111999).
The downloaded Actions artifact matched its independently recorded SHA-256
`d09a18a267ae0c07b31b5fdd0498c966ac8223b4a24c05936f66df7e45186cc8`.
All eight checksummed assets, archive members, original notices, provenance and
the actual offline CLI version/help/doctor/analyze/search checks passed again.

| Item | Actual result |
|---|---|
| Unsigned NSIS installer | 6,221,417 bytes; SHA-256 `709e458e0e29d2b538ac2b1315798246d87b121e99b848140a080ab9a88c08b3` |
| Portable desktop | SHA-256 `711beffd3a3b15c541f06a20901499c358a9a79d3bcd4cfb2e691154dbf44873` |
| Installed desktop | SHA-256 `0af1da556f0d4bbbbf7487b20168acc61e6c6dafd2ac6c0ae9f9d151a91ba152`; exactly the three-byte NSIS marker difference |
| Installation | Installer exit 0; all 471 original legal files matched again |
| Build host | GitHub-hosted Windows image `20261021.278.1`, Visual Studio Enterprise 2026 `18.11.12315.225`, channel `VisualStudio.18.Release`, `isPrerelease: false`, MSVC toolset `14.51.36231` |
| Dense | The 17-node public Python graph fitted between the floating islands, with readable persistent labels |
| Frontier | Selecting `floor` in search produced its two-node neighborhood; both `floor` and `pipeline` were visibly clear of the left and right islands |
| Teaching | The first question and progressive buttons were visible with facts/limits collapsed; Next advanced from step 1/4 to step 2/4 |
| Actual Agent | The candidate CLI completed all eight read tools in a real client; [record](agent-interoperability-release-candidate.json) |
| Close | Alt+F4 closed the native window and the desktop process exited before the larger candidate benchmarks |

[Frontier screenshot](../images/native-frontier.png) is an unmodified capture
of this installed candidate, with the selected public fixture function. The
earlier Galaxy/source screenshots and full lifecycle record retain their
original candidate scope. No native GPU frame-rate claim follows from these
visual checks. The final main-commit package must still pass its own installation
and remote-asset acceptance, recorded in the published Release notes.

After all timed benchmarks had finished, the same installed candidate opened
the actual generated 1,000-function Python source workload (ten files), rather
than a fabricated graph. Dense displayed 1,000/1,000 L5 nodes and 1,998 call plus
reference relations. Galaxy drew the graph, then its slow-device policy reduced
the displayed subset to 500/1,000 nodes; the internal callback-cadence heuristic
reported 12 Hz. This is a fallback observation, not GPU-presented FPS or proof
of a fluid 1,000-node experience. The automation helper rejected a native orbit
drag before sending input because it selected Tauri's 13-pixel auxiliary surface;
browser E2E remains the direct orbit-interaction evidence. No native orbit
success is inferred from that rejected action.
