#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Verify the complete, licensed source payloads of the two bounded backports."""
import hashlib
import json
from pathlib import Path
import sys


def verify(root: Path, name: str) -> dict:
    if name not in {"rmcp", "glib"}:
        raise ValueError("unreviewed source backport")
    security = root / "security"
    binding = json.loads((security / f"{name}-provenance.json").read_text())
    upstream = json.loads((security / f"{name}-upstream-inventory.json").read_text())
    payload = root / "vendor" / name
    files = list(payload.rglob("*"))
    if any(p.is_symlink() for p in files):
        raise ValueError(f"{name}: symlink in source payload")
    actual = {p.relative_to(payload).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
              for p in files if p.is_file()}
    if actual != binding["payloadSha256"]:
        raise ValueError(f"{name}: payload inventory/hash mismatch")
    original = upstream["upstreamSha256"]
    changed = sorted(p for p, digest in original.items() if actual.get(p) != digest)
    added = sorted(set(actual) - set(original))
    if changed != sorted(binding["modifiedUpstreamFiles"]) or added != binding.get("addedFiles", []):
        raise ValueError(f"{name}: undeclared source change")
    for key in ("crate", "version", "license", "archiveSha256", "archiveUrl"):
        if upstream[key] != binding[key]:
            raise ValueError(f"{name}: inconsistent upstream identity")
    if actual["LICENSE"] != original["LICENSE"]:
        raise ValueError(f"{name}: original license changed")
    return {"provenance": f"security/{name}-provenance.json",
            "upstream_archive_sha256": binding["archiveSha256"],
            "payload_inventory_sha256": hashlib.sha256(json.dumps(actual, sort_keys=True).encode()).hexdigest(),
            "modifications": changed + added}


if __name__ == "__main__":
    try:
        root = Path(__file__).resolve().parent.parent
        for name in ("rmcp", "glib"):
            result = verify(root, name)
            sys.stdout.write(json.dumps({"crate": name, "verification": "passed", **result}) + "\n")
    except (OSError, ValueError, KeyError) as error:
        sys.stderr.write(f"source backport verification failed: {error}\n")
        raise SystemExit(1)
