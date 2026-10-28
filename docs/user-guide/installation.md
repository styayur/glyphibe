# Installing Glyphibe

Use the assets and validation notes of an approved
[GitHub release](https://github.com/styayur/glyphibe/releases). Windows x64 is
the desktop packaging target. Linux/macOS headless CI results are separate from
desktop installation support. Source version numbers do not certify a release.

## Windows desktop

1. Download the version's x64 setup executable and `SHA256SUMS`.
2. In PowerShell, run `Get-FileHash .\Glyphibe_1.0.0_x64-setup.exe -Algorithm SHA256`
   and compare its complete hash with the corresponding checksum entry.
3. Run the installer. Glyphibe is unsigned; publisher and SmartScreen warnings
   can occur. Microsoft WebView2 is a desktop prerequisite.
4. Start Glyphibe and choose **Open folder**. Select a source directory you are
   authorized to read. Git history and dependency installation are not required.
5. Follow actual scan progress, then search/select a node and open **Evidence**,
   **Source**, **Learn** or **Knowledge**. See [desktop.md](desktop.md).

To use the portable archive, extract its complete contents into a writable
folder and run `glyphibe-desktop.exe`. Keep its companion legal files. Portable
describes executable distribution; analysis caches still use the user's cache
directory. The app never stores an index in the analyzed repository.

Use Windows Installed apps or the installation's uninstaller to remove the
desktop program. Analysis caches are user data and can survive uninstallation.
Reinstallation must not require changes to the analyzed project. The release's
lifecycle report records the exact tested install/upgrade/uninstall sequence.

## CLI and Agent connection

Extract the complete CLI archive, open a terminal in that folder and run:

```powershell
.\glyphibe.exe --version
.\glyphibe.exe doctor
.\glyphibe.exe analyze C:\Path\To\Authorized\Source
.\glyphibe.exe --repo C:\Path\To\Authorized\Source search calculate
```

The executable is independent of the desktop app and needs no extra Python,
Node.js or Rust runtime. Its Windows PE imports include `VCRUNTIME140.dll`:
the [Microsoft Visual C++ v14 x64 Runtime](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist)
must be present. Install the supported x64 runtime from Microsoft if Windows
reports that DLL missing; no Microsoft runtime DLL is bundled by Glyphibe.
Keep the `licenses` directory from the archive. If
downloading only the executable, also obtain the release's licenses archive.
Source and third-party notice locations are documented in that archive.

Configure your Agent's standard stdio MCP client with this executable and
arguments `--repo`, your authorized source directory, `mcp`, `serve`. The
directory remains scoped to that server process. Proposal writes are disabled
unless explicitly enabled; read tools never execute source code. See
[cli-and-mcp.md](cli-and-mcp.md) for JSON commands and client configuration.

## Evidence and limits

The source [gate ledger](../development/release-gates.md) and release notes
separate building from actual installation, and browser interaction from native
WebView2 behavior. Hardware-presented GPU frame rates are not inferred from
browser callback cadence. See [rendering measurements](../benchmarks/rendering.md).
