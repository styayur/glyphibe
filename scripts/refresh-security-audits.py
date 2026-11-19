#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Refresh all security ledgers, retaining failures and returning a failing status."""
from __future__ import annotations

import datetime as dt
import json
from pathlib import Path
import shutil
import subprocess
import sys


def public_diagnostic(text: str, root: Path) -> str:
    # Preserve warnings/errors while keeping workstation and hosted-runner path
    # prefixes out of public evidence. Original process logs stay with the run.
    for path, replacement in [(Path.home(), "/glyphibe-build/home"), (root, "/glyphibe/source")]:
        text = text.replace(str(path), replacement).replace(path.as_posix(), replacement)
    return text


def main() -> int:
    root = Path(__file__).resolve().parent.parent
    output = root / "docs/development/audits"
    output.mkdir(parents=True, exist_ok=True)
    commands = [
        ("cargo-audit.json", ["cargo", "audit", "--json"]),
        ("npm-audit.json", [shutil.which("npm.cmd") or shutil.which("npm") or "npm", "run", "security:audit", "--prefix", "apps/desktop"]),
    ]
    for name, package, target in [
        ("desktop-windows", "glyphibe-desktop", "x86_64-pc-windows-msvc"),
        ("cli-windows", "glyphibe-cli", "x86_64-pc-windows-msvc"),
        ("cli-linux", "glyphibe-cli", "x86_64-unknown-linux-gnu"),
        ("cli-macos", "glyphibe-cli", "x86_64-apple-darwin"),
    ]:
        filename = f"audit-{name}.json"
        commands.append((filename, [sys.executable, "scripts/audit-target-dependencies.py", "--package", package, "--target", target, "--output", str(output / filename), "--no-fetch"]))
    results = []
    for filename, command in commands:
        try:
            result = subprocess.run(command, cwd=root, capture_output=True, text=True, timeout=900)
            if filename in {"cargo-audit.json", "npm-audit.json"}:
                # Require actual JSON; never replace an unavailable audit with an
                # empty object that could be mistaken for a clean ledger.
                report = json.loads((root / "apps/desktop/artifacts/npm-audit.json").read_text(encoding="utf-8")) if filename == "npm-audit.json" else json.loads(result.stdout)
                (output / filename).write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
            else:
                json.loads((output / filename).read_text(encoding="utf-8"))
            entry = {"file": filename, "exit_code": result.returncode, "stderr": public_diagnostic(result.stderr, root), "status": "PASS" if result.returncode == 0 else "FAIL"}
        except (OSError, ValueError, subprocess.SubprocessError) as error:
            entry = {"file": filename, "exit_code": None, "status": "NOT_VERIFIED", "error": public_diagnostic(f"{type(error).__name__}: {error}", root)}
        results.append(entry)
        print(json.dumps(entry))
    # Bind ledgers to exact manifests/locks using the inventory's shared hashing.
    import importlib.util
    spec = importlib.util.spec_from_file_location("inventory", root / "scripts/audit-dependencies.py")
    inventory = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(inventory)
    index = {"schema_version": 1, "generated_at": dt.datetime.now(dt.timezone.utc).isoformat(), "input_sha256": inventory.input_hashes(root), "results": results}
    (output / "audit-refresh.json").write_text(json.dumps(index, indent=2) + "\n", encoding="utf-8")
    return int(any(entry["status"] != "PASS" for entry in results))


if __name__ == "__main__":
    raise SystemExit(main())
