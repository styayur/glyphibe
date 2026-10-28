#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Verify downloaded Windows release assets without extracting archives.

Executing the downloaded CLI additionally requires --run-cli and its separately
trusted local-build SHA-256. Checksums alone do not establish authenticity.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import subprocess
import tempfile
import zipfile


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def report_path(directory, output):
    """Keep the report separate from the inputs and refuse existing destinations."""
    downloaded = directory.resolve(strict=True)
    resolved = output.resolve()
    if resolved == downloaded or resolved.is_relative_to(downloaded):
        raise ValueError("verification report must be outside the downloaded directory")
    if output.is_symlink() or resolved.exists():
        raise ValueError("verification report must use a fresh destination")
    return resolved


def write_report(output, report):
    # Exclusive creation also rejects a destination created after report_path.
    with output.open("x", encoding="utf-8") as stream:
        stream.write(json.dumps(report, indent=2) + "\n")


def safe_name(name):
    path = PurePosixPath(name)
    if not name or "\\" in name or ":" in name or path.is_absolute() or ".." in path.parts:
        raise ValueError("unsafe archive/checksum path")
    return name


def read_sums(path):
    if path.is_symlink():
        raise ValueError("checksum manifest must be a regular file")
    records = {}
    for line in path.read_text(encoding="ascii").splitlines():
        match = re.fullmatch(r"([a-f0-9]{64})  ([^/]+)", line)
        if not match:
            raise ValueError("invalid checksum record")
        checksum, name = match.groups()
        safe_name(name)
        if name in records:
            raise ValueError("duplicate checksum record")
        records[name] = checksum
    if not records:
        raise ValueError("empty checksum manifest")
    return records


def archive_hashes(path):
    result = {}
    total = 0
    with zipfile.ZipFile(path) as archive:
        for entry in archive.infolist():
            safe_name(entry.filename)
            if stat.S_ISLNK(entry.external_attr >> 16):
                raise ValueError("symlink archive member")
            if entry.is_dir():
                continue
            if entry.filename in result:
                raise ValueError("duplicate archive member")
            total += entry.file_size
            if total > 256 * 1024 * 1024 or entry.file_size > 128 * 1024 * 1024:
                raise ValueError("archive verification budget exceeded")
            with archive.open(entry) as stream:
                result[entry.filename] = hashlib.file_digest(stream, "sha256").hexdigest()
    return result


def verify(directory, version, source_commit, source_root, trusted_sums_sha256):
    if not re.fullmatch(r"\d+\.\d+\.\d+", version):
        raise ValueError("invalid stable version")
    if not re.fullmatch(r"[a-f0-9]{64}", trusted_sums_sha256) or digest(directory / "SHA256SUMS") != trusted_sums_sha256:
        raise ValueError("downloaded checksum manifest differs from trusted local packaging")
    sums = read_sums(directory / "SHA256SUMS")
    top = {p.name for p in directory.iterdir() if p.is_file() and p.name != "SHA256SUMS"}
    if top != sums.keys():
        raise ValueError("checksum coverage differs from downloaded top-level files")
    for name, expected in sums.items():
        path = directory / name
        if path.is_symlink() or digest(path) != expected:
            raise ValueError("downloaded asset checksum mismatch")
    manifest = json.loads((directory / "asset-manifest.json").read_text(encoding="utf-8-sig"))
    if manifest["version"] != version or manifest["source_commit"] != source_commit:
        raise ValueError("release identity differs from expected reviewed source")
    identities = {entry["file"] for entry in manifest["assets"]}
    if identities != top - {"asset-manifest.json"}:
        raise ValueError("asset inventory coverage differs from checksum manifest")
    for entry in manifest["assets"]:
        safe_name(entry["file"])
        if entry["sha256"] != sums[entry["file"]] or entry["bytes"] != (directory / entry["file"]).stat().st_size:
            raise ValueError("asset inventory content differs")
    cli_name = f"glyphibe-{version}-windows-x64.exe"
    payloads = {p["file"]: p["sha256"] for p in manifest["payloads"]}
    if sums[cli_name] != payloads["glyphibe.exe"]:
        raise ValueError("CLI payload identity differs")
    provenance = json.loads((directory / "build-provenance.json").read_text(encoding="utf-8-sig"))
    if provenance["source_commit"] != source_commit or {p["file"]: p["sha256"] for p in provenance["payloads"]} != payloads or any(p["private_build_path_scan"] != "passed" for p in provenance["payloads"]):
        raise ValueError("Build provenance differs from the reviewed source or payloads")
    installer_name = f"Glyphibe_{version}_x64-setup.exe"
    installer = provenance.get("installer")
    if not isinstance(installer, dict) or installer.get("file") != installer_name or installer.get("sha256") != sums.get(installer_name) or installer.get("bytes") != (directory / installer_name).stat().st_size or installer.get("private_build_path_scan") != "passed":
        raise ValueError("installer differs from the validated build provenance")
    legal = {f"licenses/{name}": digest(source_root / name) for name in ("LICENSE", "THIRD_PARTY_NOTICES", "PLATFORM_NOTICES")}
    for path in (source_root / "third-party" / "licenses").glob("*.txt"):
        legal[f"licenses/third-party/licenses/{path.name}"] = digest(path)
    for path in (source_root / "third-party" / "toolchain-licenses").rglob("*"):
        if path.is_file():
            legal[f"licenses/third-party/toolchain-licenses/{path.relative_to(source_root / 'third-party' / 'toolchain-licenses').as_posix()}"] = digest(path)
    for kind in ("portable", "cli", "licenses"):
        name = f"Glyphibe-{version}-licenses.zip" if kind == "licenses" else f"Glyphibe-{version}-windows-x64-{kind}.zip"
        received = archive_hashes(directory / name)
        required = dict(legal)
        if kind != "licenses":
            required["glyphibe.exe"] = payloads["glyphibe.exe"]
        if kind == "portable":
            required["glyphibe-desktop.exe"] = payloads["glyphibe-desktop.exe"]
        allowed = set(required) | ({"README.txt"} if kind != "licenses" else set())
        if set(received) != allowed:
            raise ValueError("archive has an unexpected or missing member")
        if any(received.get(name) != checksum for name, checksum in required.items()):
            raise ValueError("archive is missing or changes a required binary/legal payload")
    return {"version": version, "source_commit": source_commit, "assets_checked": len(sums), "asset_sha256": sums, "legal_files_per_archive": len(legal), "payloads": payloads, "installer": installer}


def exercise_cli(binary, expected_sha, source_root):
    if os.name != "nt" or not re.fullmatch(r"[a-f0-9]{64}", expected_sha or "") or digest(binary) != expected_sha:
        raise ValueError("execution requires Windows and the trusted local build's exact SHA-256")
    environment = dict(os.environ, HTTP_PROXY="http://127.0.0.1:9", HTTPS_PROXY="http://127.0.0.1:9")
    for key in ("OPENAI_API_KEY", "ANTHROPIC_API_KEY", "GEMINI_API_KEY"):
        environment.pop(key, None)
    def run(*args):
        result = subprocess.run([str(binary), *map(str, args)], capture_output=True, text=True, encoding="utf-8", env=environment, timeout=60, check=True)
        return result.stdout
    version = run("--version").strip()
    help_text = run("--help")
    if any(command not in help_text for command in ("analyze", "graph", "search", "trace", "explain", "impact", "kb", "export", "doctor", "mcp")):
        raise ValueError("CLI help omits a required command")
    json.loads(run("doctor"))
    with tempfile.TemporaryDirectory(prefix="glyphibe-remote-verify-") as cache:
        source = source_root / "tests" / "fixtures" / "python"
        index = Path(cache) / "index.sqlite"
        analysis = json.loads(run("analyze", source, "--index", index))
        hits = json.loads(run("--repo", source, "--index", index, "search", "calculate"))
        if not analysis["nodes"] or not any(n["label"] == "calculate" and n["kind"] == "function" for n in hits):
            raise ValueError("downloaded CLI failed the actual public fixture query")
    return {"version": version, "help": "passed", "doctor_json": "passed", "offline_fixture_analyze_and_search": "passed", "execution_sha256": expected_sha}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--source-root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--run-cli", action="store_true")
    parser.add_argument("--trusted-cli-sha256")
    parser.add_argument("--trusted-sums-sha256", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    output = report_path(args.directory, args.output)
    report = verify(args.directory, args.version, args.source_commit, args.source_root, args.trusted_sums_sha256)
    if args.run_cli:
        report["cli_execution"] = exercise_cli(args.directory / f"glyphibe-{args.version}-windows-x64.exe", args.trusted_cli_sha256, args.source_root)
        if report["cli_execution"]["version"] != f"glyphibe {args.version}":
            raise ValueError("downloaded executable version mismatch")
    report.update(verified_at=datetime.now(timezone.utc).isoformat(), result="passed", method="SHA-256 of downloaded assets, exact inventory/source identity, non-extracting ZIP payload and legal-file verification; optional hash-pinned CLI execution")
    write_report(output, report)
    print(json.dumps({"result": "passed", "assets_checked": report["assets_checked"], "cli_executed": args.run_cli}))


if __name__ == "__main__":
    main()
