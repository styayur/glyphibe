#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Read-only public project validation, using separately acquired Git checkouts.

Never installs dependencies or executes project code. Reuses benchmark.py's
bounded CLI runner and memory sampler; indices live under --work-dir.
"""
import argparse
from datetime import datetime, timedelta, timezone
import hashlib
import json
from pathlib import Path
import subprocess

from benchmark import accuracy, real_scan

CASES = (
    ("click", "pallets/click", "BSD-3-Clause", "LICENSE.txt"),
    ("ripgrep", "BurntSushi/ripgrep", "Unlicense OR MIT", "COPYING"),
    ("sigma", "jacomyal/sigma.js", "MIT", "LICENSE.txt"),
)


def git(root, *args):
    return subprocess.check_output(
        ["git", "-c", "core.fsmonitor=false", "-C", str(root), *args],
        timeout=30,
    )


def fingerprint(root):
    digest = hashlib.sha256()
    count = 0
    for entry in sorted(git(root, "ls-files", "-z").split(b"\0")):
        if not entry:
            continue
        path = root / entry.decode("utf-8")
        if path.is_symlink() or not path.is_file():
            continue
        path.resolve().relative_to(root)
        digest.update(entry + b"\0")
        with path.open("rb") as stream:
            while chunk := stream.read(65536):
                digest.update(chunk)
        count += 1
    return digest.hexdigest(), count


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--checkout-root", type=Path, required=True)
    parser.add_argument("--work-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    options = parser.parse_args()
    options.work_dir.mkdir(parents=True, exist_ok=True)
    binary = options.binary.resolve(strict=True)
    report = {
        "measured_at": datetime.now(timezone(timedelta(hours=8))).isoformat(),
        "binary_version": subprocess.check_output([str(binary), "--version"], text=True).strip(),
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "build_profile": "release; optimization 3, thin LTO, stripped",
        "hardware_reference": "analysis-results.json; same Windows workstation",
        "conditions": "Concurrent desktop Rust release compilation; OS cache not flushed. No comparative performance claim.",
        "method": "Full checkout, bounded read-only analyze; external fresh SQLite; 180 seconds and 2 GiB per CLI process. All tracked regular file hashes and clean Git status checked before/after.",
        "cases": [], "failures": [],
    }
    for number, (folder, repository, license_id, license_file) in enumerate(CASES, 1):
        try:
            root = (options.checkout_root / folder).resolve(strict=True)
            expected_remote = f"https://github.com/{repository}.git"
            if git(root, "remote", "get-url", "origin").decode().strip() != expected_remote:
                raise ValueError("checkout remote mismatch")
            if git(root, "status", "--porcelain", "--untracked-files=all").strip():
                raise ValueError("checkout must be clean")
            revision = git(root, "rev-parse", "HEAD").decode().strip()
            before, count = fingerprint(root)
            result = real_scan(binary, options.work_dir, root, number)
            after, after_count = fingerprint(root)
            unchanged = before == after and count == after_count and not git(root, "status", "--porcelain", "--untracked-files=all").strip()
            result.update(repository=repository, revision=revision, license=license_id,
                          license_source=f"https://github.com/{repository}/blob/{revision}/{license_file}",
                          license_sha256=hashlib.sha256((root / license_file).read_bytes()).hexdigest(),
                          tracked_regular_files=count, source_unchanged=unchanged)
            report["cases"].append(result)
            if not unchanged:
                report["failures"].append({"repository": repository, "error": "source integrity changed"})
            print(f"{repository}: {result['stats']['parsed_files']} parsed files; unchanged={unchanged}", flush=True)
        except Exception as error:
            report["failures"].append({"repository": repository, "error_type": type(error).__name__, "detail": "See private execution diagnostics; local paths excluded"})
    report["accuracy"] = accuracy(binary, options.work_dir, Path(__file__).resolve().parents[1])
    options.output.parent.mkdir(parents=True, exist_ok=True)
    options.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    if report["failures"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
