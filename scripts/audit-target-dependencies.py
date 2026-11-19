#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Audit a selected package's real normal/build target graph without global ignores.

Preserves version/source/checksum from the active workspace Cargo.lock. The
temporary lock is generated from Cargo's selected build graph, never by
deleting a known advisory package. Development-only edges are excluded from this
shipping-graph check; the full all-target audit remains a separate ledger.
"""
import argparse
from collections import deque
import json
import re
from pathlib import Path
import subprocess
import sys
import tempfile
import tomllib


def select_tree_packages(text, packages):
    """Map Cargo's actual build graph to exact metadata identities; fail closed.

    Metadata can retain disabled weak optional edges (Cargo #10801). Tree uses
    the build feature resolver. No package/advisory names are special-cased.
    """
    identities = {}
    for package_id, package in packages.items():
        identities.setdefault((package["name"], package["version"]), []).append(package_id)
    selected = set()
    for line in text.splitlines():
        match = re.fullmatch(r"([A-Za-z0-9_-]+) v([^\s]+)(?: .*)?", line)
        if not match:
            raise ValueError("unrecognized Cargo tree line")
        candidates = identities.get(match.groups(), [])
        if len(candidates) != 1:
            raise ValueError("unknown/ambiguous Cargo tree package identity")
        selected.add(candidates[0])
    if not selected:
        raise ValueError("empty Cargo shipping graph")
    return selected


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
    # Keep host build-script/proc-macro identities too; tree selects the actual
    # host/target graph. A target-only metadata filter can lose host-side edges.
    process = subprocess.run(["cargo", "metadata", "--locked", "--format-version", "1"], cwd=root, check=True, capture_output=True, timeout=240)
    metadata = json.loads(process.stdout)
    packages = {p["id"]: p for p in metadata["packages"]}
    matches = [p["id"] for p in packages.values() if p["name"] == options.package and p["source"] is None]
    if len(matches) != 1:
        raise ValueError("package must identify exactly one local workspace package")
    tree = subprocess.run(["cargo", "tree", "--locked", "--package", options.package,
        "--target", options.target, "--edges", "normal,build", "--prefix", "none",
        "--format", "{p}", "--color", "never"], cwd=root, check=True,
        capture_output=True, text=True, timeout=240)
    actual = select_tree_packages(tree.stdout, packages)
    nodes = {n["id"]: n for n in metadata["resolve"]["nodes"]}
    reached = set()
    queue = deque(matches)
    dependency_edges = {}
    while queue:
        package = queue.popleft()
        if package in reached:
            continue
        reached.add(package)
        dependencies = [d["pkg"] for d in nodes[package]["deps"]
                        if d["pkg"] in actual and any(k["kind"] != "dev" for k in d["dep_kinds"])]
        dependency_edges[package] = dependencies
        queue.extend(dependencies)
    if reached != actual:
        raise ValueError("Cargo metadata/tree closure disagreement; refusing incomplete audit")
    locked = tomllib.loads((Path(metadata["workspace_root"]) / "Cargo.lock").read_text())
    lock_by_identity = {(p["name"], p["version"], p.get("source")): p for p in locked["package"]}
    lines = ["# Generated from cargo tree normal/build closure; exact metadata/lock identities.", f"version = {locked['version']}", ""]
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
                "method": "Cargo tree normal/build target closure, exact identities and lock entries from metadata; dev-only/disabled edges excluded; no advisory ignore list",
                "cargo_tree": tree.stdout.replace(str(root), "/glyphibe/source").splitlines(),
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
