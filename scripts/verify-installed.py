#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Read-only verification of an installed Windows candidate against its assets.

Run the release-asset verifier first. This command does not install, uninstall,
launch an executable or certify that a graphical interaction was exercised.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import zipfile

from verify_release_assets import archive_hashes, digest, report_path, write_report


def verify(assets, installed, version):
    root = installed.resolve(strict=True)
    if installed.is_symlink() or (installed.stat().st_file_attributes & 0x400):
        raise ValueError("installation root is a reparse point")
    portable = assets / f"Glyphibe-{version}-windows-x64-portable.zip"
    legal = assets / f"Glyphibe-{version}-licenses.zip"
    # Validate member safety and budgets without extracting either archive.
    archive_hashes(portable)
    expected = archive_hashes(legal)
    with zipfile.ZipFile(portable) as archive:
        original = archive.read("glyphibe-desktop.exe")
    executable = root / "glyphibe-desktop.exe"
    actual = executable.read_bytes()
    marker = b"TAURI_BUNDLE_TYPE_VAR_UNK"
    if original == actual:
        match = "exact portable payload"
    elif original.count(marker) == 1 and actual == original.replace(marker, b"TAURI_BUNDLE_TYPE_VAR_NSS"):
        match = "exact payload with the documented three-byte NSIS bundle marker"
    else:
        raise ValueError("installed desktop differs beyond the documented bundle marker")
    for name, expected_hash in expected.items():
        path = root / name
        path.resolve(strict=True).relative_to(root)
        for component in (path, *path.parents):
            if component == root:
                break
            if component.is_symlink() or (component.stat().st_file_attributes & 0x400):
                raise ValueError("installed legal path contains a reparse point")
        if digest(path) != expected_hash:
            raise ValueError("installed original notice differs from its verified asset")
    return {"version": version, "result": "passed", "installed_payload_match": match,
            "installed_desktop_sha256": hashlib.sha256(actual).hexdigest(),
            "portable_desktop_sha256": hashlib.sha256(original).hexdigest(),
            "installed_legal_files_checked": len(expected),
            "scope": "Installed bytes and original notices only; actual GUI and lifecycle outcomes are recorded separately."}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--assets", type=Path, required=True)
    parser.add_argument("--installed", type=Path, required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if os.name != "nt":
        parser.error("installed Windows payload validation requires Windows")
    if not re.fullmatch(r"\d+\.\d+\.\d+", args.version):
        parser.error("expected a stable numeric version")
    output = report_path(args.assets, args.output)
    report_path(args.installed, output)
    report = verify(args.assets, args.installed, args.version)
    write_report(output, report)
    print(json.dumps({"result": report["result"], "legal_files_checked": report["installed_legal_files_checked"]}))


if __name__ == "__main__":
    main()
