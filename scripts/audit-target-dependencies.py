#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Audit a selected package's real normal/build target graph without global ignores.

Preserves version/source/checksum from the active workspace Cargo.lock. The
temporary lock is generated from Cargo's filtered resolve graph, never by
deleting a known advisory package. Development-only edges are excluded from this
shipping-graph check; the full all-target audit remains a separate ledger.
"""
import argparse
from collections import deque
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import tomllib


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package", required=True)
    parser.add_argument("--target", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--no-fetch", action="store_true")
    options = parser.parse_args()
    if options.target not in {"x86_64-pc-windows-msvc", "x86_64-unknown-linux-gnu", "x86_64-apple-darwin"}:
        parser.error("unsupported audited target")
    root = Path(__file__).resolve().parent.parent
    process = subprocess.run(["cargo", "metadata", "--locked", "--format-version", "1", "--filter-platform", options.target], cwd=root, check=True, capture_output=True, timeout=240)
    metadata = json.loads(process.stdout)
    packages = {p["id"]: p for p in metadata["packages"]}
    matches = [p["id"] for p in packages.values() if p["name"] == options.package and p["source"] is None]
    if len(matches) != 1:
        raise ValueError("package must identify exactly one local workspace package")
    nodes = {n["id"]: n for n in metadata["resolve"]["nodes"]}
    reached = set()
    queue = deque(matches)
    dependency_edges = {}
    while queue:
        package = queue.popleft()
        if package in reached:
            continue
        reached.add(package)
        dependencies = [d["pkg"] for d in nodes[package]["deps"] if any(k["kind"] != "dev" for k in d["dep_kinds"])]
        dependency_edges[package] = dependencies
        queue.extend(dependencies)
    locked = tomllib.loads((Path(metadata["workspace_root"]) / "Cargo.lock").read_text())
    lock_by_identity = {(p["name"], p["version"], p.get("source")): p for p in locked["package"]}
    lines = ["# Generated from cargo metadata --filter-platform; normal/build closure.", f"version = {locked['version']}", ""]
    selected = []
    for package_id in sorted(reached):
        p = packages[package_id]
        entry = lock_by_identity[(p["name"], p["version"], p["source"])]
        lines.append("[[package]]")
        for key in ("name", "version", "source", "checksum"):
            if key in entry:
                lines.append(f"{key} = {json.dumps(entry[key])}")
        deps = []
        for dep_id in dependency_edges.get(package_id, []):
            dep = packages[dep_id]
            name = f"{dep['name']} {dep['version']}"
            if dep["source"]:
                name += f" ({dep['source']})"
            deps.append(name)
        if deps:
            lines += ["dependencies = ["] + [" " + json.dumps(d) + "," for d in sorted(set(deps))] + ["]"]
        lines.append("")
        selected.append({"name": p["name"], "version": p["version"], "source": p["source"]})
    with tempfile.TemporaryDirectory(prefix="glyphibe-target-audit-") as directory:
        lock = Path(directory) / "Cargo.lock"
        lock.write_text("\n".join(lines))
        command = ["cargo", "audit", "--json", "--deny", "warnings", "--file", str(lock)]
        if options.no_fetch:
            command.append("--no-fetch")
        audit = subprocess.run(command, cwd=root, capture_output=True, timeout=240)
        report = json.loads(audit.stdout)
    options.output.parent.mkdir(parents=True, exist_ok=True)
    combined = {"schema_version": 1, "package": options.package, "target": options.target,
                "method": "Cargo filtered normal/build dependency closure; dev-only edges excluded; no advisory ignore list",
                "selected_packages": selected, "audit_exit_code": audit.returncode, "audit": report}
    options.output.write_text(json.dumps(combined, indent=2) + "\n", encoding="utf-8")
    warnings = {key: len(value) for key, value in report.get("warnings", {}).items()}
    print(json.dumps({"package": options.package, "target": options.target, "packages": len(selected), "exit_code": audit.returncode,
                      "vulnerabilities": report.get("vulnerabilities", {}).get("count"), "warnings": warnings}))
    return audit.returncode


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        print(f"target dependency audit failed: {type(error).__name__}: {error}", file=sys.stderr)
        raise SystemExit(2)
