#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Collect/check native packaging notices separately from Cargo/npm metadata.

Collection reads trusted installed Rust/NSIS tools and a previously acquired,
hash-pinned Microsoft SDK package. It never installs or runs analyzed code.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tomllib
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[1]
DESTINATION = ROOT / "third-party" / "toolchain-licenses"
REPORT = ROOT / "docs" / "development" / "native-packaging-inventory.json"
PLUGIN_COMMIT = "13d9edd27b69310e108d6fbd49f90992f8a05390"
PLUGIN_SHA256 = "5ba143b5db4a87d32d6e7802e033330aae56cbceabe0d1e3ba41948385ad4709"
WEBVIEW_VERSION = "1.0.3800.47"
WEBVIEW_ARCHIVE_SHA256 = "56c9f26bdd07916a2d1949fb58a5c7e434dfa1173577dca879206050c4e718db"
WEBVIEW_LOADER_SHA256 = "89c6b872783b8f6c3cedbff618adb42082d455c615453ae10cfc753f1e8f25d8"


def sha(data):
    return hashlib.sha256(data).hexdigest()


def check():
    report = json.loads(REPORT.read_text(encoding="utf-8"))
    config = json.loads((ROOT / "apps/desktop/src-tauri/tauri.conf.json").read_text())
    if config["bundle"]["windows"]["nsis"]["compression"] != "zlib":
        raise ValueError("The reviewed installer uses zlib, not the alternate LZMA module")
    lock = json.loads((ROOT / "apps/desktop/package-lock.json").read_text())
    if lock["packages"]["node_modules/@tauri-apps/cli"]["version"] != report["tauri_cli_version"]:
        raise ValueError("Tauri CLI version changed; refresh native packaging evidence")
    cargo = tomllib.loads((ROOT / "Cargo.lock").read_text())
    if not any(p["name"] == "webview2-com-sys" and p["version"] == "0.39.1" for p in cargo["package"]):
        raise ValueError("WebView2 binding version changed; recheck its actual loader")
    files = report["notices"]
    expected = {entry["file"] for entry in files}
    actual = {p.relative_to(ROOT).as_posix() for p in DESTINATION.rglob("*") if p.is_file()}
    if expected != actual:
        raise ValueError("Native notice coverage differs from the recorded files")
    for entry in files:
        path = ROOT / entry["file"]
        path.resolve().relative_to(DESTINATION.resolve())
        if path.is_symlink() or sha(path.read_bytes()) != entry["sha256"]:
            raise ValueError("Native original notice bytes changed")
    print(json.dumps({"result": "passed", "native_notice_files": len(files), "compressor": "zlib"}))


def collect(options):
    notices = []
    def preserve(name, data, source, component):
        destination = DESTINATION / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(data)
        notices.append({"component": component, "file": destination.relative_to(ROOT).as_posix(), "sha256": sha(data), "bytes": len(data), "source": source})

    rust = Path(subprocess.check_output(["rustc", "--print", "sysroot"], text=True).strip())
    rust_info = subprocess.check_output(["rustc", "-vV"], text=True)
    rust_commit = next(line.split(": ", 1)[1] for line in rust_info.splitlines() if line.startswith("commit-hash:"))
    rust_version = next(line.split(": ", 1)[1] for line in rust_info.splitlines() if line.startswith("release:"))
    rust_docs = rust / "share/doc/rust"
    for relative in ("COPYRIGHT-library.html", "licenses/MIT.txt", "licenses/Apache-2.0.txt"):
        preserve("rust/" + relative, (rust_docs / relative).read_bytes(), f"Official installed Rust {rust_version} distribution, share/doc/rust/{relative}; source https://github.com/rust-lang/rust/tree/{rust_commit}", "Rust standard library")

    nsis = options.nsis_root
    nsis_version = subprocess.check_output([str(nsis / "makensis.exe"), "/VERSION"], text=True).strip()
    if nsis_version != "v3.13":
        raise ValueError("Unexpected NSIS tool version; review its own terms")
    for relative, name in (("COPYING", "COPYING.txt"), ("Docs/NSISdl/License.txt", "NSISdl-LICENSE.txt"), ("Docs/Modern UI 2/License.txt", "Modern-UI-2-LICENSE.txt")):
        preserve("nsis/" + name, (nsis / relative).read_bytes(), f"Unmodified NSIS 3.13 distribution: {relative}; https://nsis.sourceforge.io/License", "NSIS installer")
    plugins = list(nsis.rglob("nsis_tauri_utils.dll"))
    if len(plugins) != 1 or sha(plugins[0].read_bytes()) != PLUGIN_SHA256:
        raise ValueError("Native Tauri plugin differs from its official v0.5.3 asset")
    for name, blob_sha in (("LICENSE_MIT", "7e54d8e4c28e943a545c1bef79f8dfdb2a2affe2"), ("LICENSE_APACHE-2.0", "f433b1a53f5b830a205fd2df78e2b34974656c7b")):
        url = f"https://raw.githubusercontent.com/tauri-apps/nsis-tauri-utils/{PLUGIN_COMMIT}/{name}"
        with urllib.request.urlopen(url, timeout=30) as response:
            data = response.read(100000)
        identity = hashlib.sha1(b"blob " + str(len(data)).encode() + b"\0" + data).hexdigest()
        if identity != blob_sha:
            raise ValueError("Pinned native plugin notice blob differs")
        preserve("tauri-nsis-utils/" + name + ".txt", data, url, "Tauri NSIS utilities")

    archive_data = options.webview_package.read_bytes()
    if sha(archive_data) != WEBVIEW_ARCHIVE_SHA256:
        raise ValueError("Microsoft NuGet package differs from independently checked acquisition")
    with zipfile.ZipFile(options.webview_package) as package:
        if sha(package.read("build/native/x64/WebView2LoaderStatic.lib")) != WEBVIEW_LOADER_SHA256:
            raise ValueError("Microsoft loader differs from actual compiled loader")
        for name in ("LICENSE.txt", "NOTICE.txt"):
            preserve("webview2-sdk/" + name, package.read(name), f"https://api.nuget.org/v3-flatcontainer/microsoft.web.webview2/{WEBVIEW_VERSION}/microsoft.web.webview2.{WEBVIEW_VERSION}.nupkg :: {name}", "Microsoft WebView2 Loader")

    cli_version = json.loads((ROOT / "apps/desktop/package-lock.json").read_text())["packages"]["node_modules/@tauri-apps/cli"]["version"]
    report = {
        "schema_version": 1, "collected_at": datetime.now(timezone.utc).isoformat(),
        "scope": "Windows installer/runtime supplement; Cargo/npm dependency evidence remains separate. Not a claim that every Windows OS component is redistributed.",
        "tauri_cli_version": cli_version,
        "rust": {"version": rust_version, "commit": rust_commit, "license": "MIT OR Apache-2.0 with third-party library notices retained", "notice_scope": "Complete upstream library notices, including platform-specific entries; not all listed source components enter the Windows target"},
        "nsis": {"version": "3.13", "selected_compressor": "zlib", "selected_runtime_license": "Zlib", "source": "https://nsis.sourceforge.io/License", "note": "Original COPYING also describes alternative compression modules; the CPL LZMA module is not selected"},
        "tauri_nsis_utils": {"version": "0.5.3", "source_commit": PLUGIN_COMMIT, "binary_sha256": PLUGIN_SHA256, "official_asset": "https://github.com/tauri-apps/nsis-tauri-utils/releases/download/nsis_tauri_utils-v0.5.3/nsis_tauri_utils.dll", "license": "MIT OR Apache-2.0", "source_dependency_families": ["nsis-plugin-api (same repository)", "windows-sys 0.61.2 range", "semver 1.0, no_std", "build-only nsis-fn/proc-macro2/quote/syn"], "limitation": "Official no_std plugin artifact identity and pinned source checked. An exact transitive-build-version SBOM is not supplied upstream and is not invented here."},
        "webview2_loader": {"version": WEBVIEW_VERSION, "binding_crate": "webview2-com-sys 0.39.1", "license": "BSD-3-Clause", "nuget_archive_sha256": WEBVIEW_ARCHIVE_SHA256, "nuget_acquisition": "Official HTTPS flat-container package; SHA-512 matched its x-ms-meta-sha512 response header", "compiled_x64_static_library_sha256": WEBVIEW_LOADER_SHA256, "provenance": "Official package member SHA-256 exactly matches the crate's x64 WebView2LoaderStatic.lib"},
        "microsoft_platform": {"vc_runtime": "CLI imports VCRUNTIME140.dll; install the Microsoft v14 x64 Runtime separately. Desktop links runtime code through Tauri's static_vcruntime build support.", "terms": ["https://visualstudio.microsoft.com/license-terms/vs2026-ga-pro-enterprise/", "https://visualstudio.microsoft.com/license-terms/vs2026-ga-visualcpp-v14-redist-runtime/", "https://learn.microsoft.com/en-us/visualstudio/releases/2026/redistribution", "https://visualstudio.microsoft.com/license-terms/vs2022-ga-diagnosticbuildtools/", "https://visualstudio.microsoft.com/license-terms/vs2022-cruntime/", "https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist"], "scope": "Platform/toolchain components keep Microsoft terms and are not relicensed as Glyphibe MPL sources. No standalone Microsoft runtime installer or DLL is added as a release asset."},
        "notices": notices,
    }
    REPORT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--collect", action="store_true")
    parser.add_argument("--verify-build-host", action="store_true")
    parser.add_argument("--target-directory", type=Path)
    parser.add_argument("--webview-package", type=Path)
    parser.add_argument("--nsis-root", type=Path, default=Path(os.environ.get("LOCALAPPDATA", "")) / "tauri/NSIS")
    options = parser.parse_args()
    if options.collect:
        if not options.webview_package:
            parser.error("--collect requires the checked --webview-package")
        collect(options)
    check()
    if options.verify_build_host:
        if os.name != "nt" or not options.target_directory:
            parser.error("Build-host validation requires Windows and --target-directory")
        info = subprocess.check_output(["rustc", "-vV"], text=True)
        report = json.loads(REPORT.read_text(encoding="utf-8"))
        if f"commit-hash: {report['rust']['commit']}" not in info or f"release: {report['rust']['version']}" not in info:
            raise ValueError("Actual Rust compiler differs from the recorded toolchain")
        if subprocess.check_output([str(options.nsis_root / "makensis.exe"), "/VERSION"], text=True).strip() != "v3.13":
            raise ValueError("Actual NSIS compiler version differs")
        plugins = list(options.nsis_root.rglob("nsis_tauri_utils.dll"))
        if len(plugins) != 1 or sha(plugins[0].read_bytes()) != PLUGIN_SHA256:
            raise ValueError("Actual installer plugin differs from the reviewed official asset")
        cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
        crates = list((cargo_home / "registry/src").glob("*/webview2-com-sys-0.39.1"))
        loaders = [p for crate in crates for p in crate.rglob("WebView2LoaderStatic.lib") if "x64" in p.parts]
        if not loaders or any(sha(p.read_bytes()) != WEBVIEW_LOADER_SHA256 for p in loaders):
            raise ValueError("Actual x64 WebView2 loader differs from the original Microsoft package")
        script = (options.target_directory / "release/nsis/x64/installer.nsi").read_text(encoding="utf-8-sig")
        import re
        if not re.search(r'(?im)^\s*SetCompressor\s+(?:/SOLID\s+)?"?zlib"?\s*$', script):
            raise ValueError("Generated installer does not select the reviewed zlib compressor")
        print(json.dumps({"result": "passed", "actual_build_host_components": "Rust/NSIS/plugin/WebView2 loader/zlib"}))


if __name__ == "__main__":
    main()
