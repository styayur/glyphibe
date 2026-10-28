#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Inventory locked dependencies, apply a conservative SPDX policy and retain notices.

Uses only the standard library. Never runs dependency code or package scripts.
Cargo metadata may download trusted locked package sources; --offline disables it.
Missing npm platform packages can be read from integrity-verified tarballs only
with --fetch-missing-license-texts. No archive member is extracted to disk.
"""
from __future__ import annotations

import argparse
import base64
import datetime as dt
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys
import tarfile
import tomllib
from urllib.parse import quote, urlparse
from urllib.request import Request, build_opener, HTTPRedirectHandler

ALLOWED = {
    "0BSD", "MIT", "MIT-0", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause",
    "ISC", "Zlib", "Unlicense", "CC0-1.0", "MPL-2.0", "BSL-1.0",
    "Unicode-3.0", "Unicode-DFS-2016", "BlueOak-1.0.0", "OpenSSL",
    "CDLA-Permissive-2.0", "Python-2.0", "CC-BY-4.0", "NCSA",
}
EXCEPTIONS = {"LLVM-exception"}
NOTICE_NAME = re.compile(r"^(?:(?:third[-_ ]party[-_ ])?(?:licen[cs]e|copying|notice|unlicense|copyright)|authors)(?:[-_.].*)?$", re.I)
MAX_NOTICE = 2 * 1024 * 1024
MAX_ARCHIVE = 64 * 1024 * 1024


def json_read(path: Path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def expression_choice(expression: str | None) -> tuple[str | None, str]:
    """Parse AND/OR/WITH/parentheses; never approve an unknown atom by substring."""
    if not expression:
        return None, "missing license expression"
    # Cargo documents legacy slash-separated alternatives. Preserve raw metadata
    # separately and make this normalization explicit in the generated inventory.
    normalized = expression.replace("/", " OR ")
    tokens = re.findall(r"\(|\)|[A-Za-z0-9.+-]+", normalized)
    if re.sub(r"\s+", "", "".join(tokens)) != re.sub(r"\s+", "", normalized):
        return None, "unrecognized SPDX syntax"
    position = 0

    def atom():
        nonlocal position
        if position >= len(tokens):
            raise ValueError("incomplete expression")
        value = tokens[position]
        position += 1
        if value == "(":
            choice = disjunction()
            if position >= len(tokens) or tokens[position] != ")":
                raise ValueError("unclosed parenthesis")
            position += 1
            return choice
        if value in {"AND", "OR", "WITH", ")"}:
            raise ValueError("expected SPDX identifier")
        choice = value if value in ALLOWED else None
        if position < len(tokens) and tokens[position] == "WITH":
            position += 1
            if position >= len(tokens):
                raise ValueError("missing exception")
            exception = tokens[position]
            position += 1
            choice = f"{value} WITH {exception}" if value == "Apache-2.0" and exception in EXCEPTIONS else None
        return choice

    def conjunction():
        nonlocal position
        left = atom()
        while position < len(tokens) and tokens[position] == "AND":
            position += 1
            right = atom()
            left = f"({left} AND {right})" if left and right else None
        return left

    def disjunction():
        nonlocal position
        left = conjunction()
        while position < len(tokens) and tokens[position] == "OR":
            position += 1
            right = conjunction()
            left = left or right
        return left

    try:
        choice = disjunction()
        if position != len(tokens):
            raise ValueError("trailing expression tokens")
        return choice, "legacy slash normalized to OR" if "/" in expression else "SPDX expression"
    except ValueError as error:
        return None, str(error)


def notice_files(root: Path, explicit: str | None = None) -> dict[str, bytes]:
    result = {}
    if not root.is_dir():
        return result
    canonical = root.resolve()
    for directory, folders, files in os.walk(root, followlinks=False):
        current = Path(directory)
        folders[:] = [f for f in folders if f not in {".git", "target", "node_modules"}
                      and not (current / f).is_symlink()
                      and len((current / f).relative_to(root).parts) <= 14]
        for name in files:
            path = current / name
            if (NOTICE_NAME.match(name) or current.name.lower() in {"licenses", "license"}) and not path.is_symlink() and path.stat().st_size <= MAX_NOTICE:
                data = path.read_bytes()
                if data and b"\0" not in data:
                    result[path.relative_to(root).as_posix()] = data
    if explicit:
        path = root / explicit
        if path.is_file() and not path.is_symlink() and path.resolve().is_relative_to(canonical):
            if path.stat().st_size <= MAX_NOTICE:
                result[path.relative_to(root).as_posix()] = path.read_bytes()
    if not result:
        # Some published packages put a complete license in their README or a
        # source header. Retain that notice only, never implementation code.
        candidates = list(root.glob("README*")) + list(root.glob("*.js"))
        for path in candidates:
            if path.is_file() and not path.is_symlink() and path.stat().st_size <= MAX_NOTICE:
                text = path.read_text(encoding="utf-8", errors="replace")
                if path.name.lower().startswith("readme"):
                    sections = re.split(r"(?im)^(?:#{1,6}\s+(?:licen[cs]e|copyright)[^\n]*\n|licen[cs]e[^\n]*\n[-=]+\s*\n)", text)
                    chunks = sections[1:]
                else:
                    chunks = re.findall(r"/\*.*?\*/", text[:32_768], re.S)
                for chunk in chunks:
                    mit = "Permission is hereby granted" in chunk and "THE SOFTWARE IS PROVIDED" in chunk
                    bsd = "Redistribution and use in source and binary forms" in chunk and "THIS SOFTWARE IS PROVIDED" in chunk
                    if mit or bsd:
                        result[path.name + "#embedded-license"] = chunk.encode()
    return result


class NoRedirect(HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        raise ValueError("registry redirects are forbidden")


def cached_get(url: str, cache: Path, limit: int = MAX_NOTICE) -> bytes:
    parsed = urlparse(url)
    if parsed.scheme != "https" or parsed.hostname not in {"registry.npmjs.org", "api.github.com", "raw.githubusercontent.com"} or parsed.username or parsed.port:
        raise ValueError("source notice host is not permitted")
    cache.mkdir(parents=True, exist_ok=True)
    target = cache / (hashlib.sha256(url.encode()).hexdigest() + ".response")
    if target.exists():
        if target.stat().st_size > limit:
            raise ValueError("cached notice response exceeds limit")
        return target.read_bytes()
    headers = {"User-Agent": "Glyphibe-license-audit/1"}
    if parsed.hostname == "api.github.com" and os.environ.get("GITHUB_TOKEN"):
        headers["Authorization"] = "Bearer " + os.environ["GITHUB_TOKEN"]
    with build_opener(NoRedirect()).open(Request(url, headers=headers), timeout=30) as response:
        data = response.read(limit + 1)
    if len(data) > limit:
        raise ValueError("source notice response exceeds limit")
    target.write_bytes(data)
    return data


def github_notices(repository: str, revision: str, subdirectory: str, cache: Path) -> dict[str, bytes]:
    # GitHub's canonical repository response confirms this upstream move. Keep
    # the published commit fixed; no arbitrary redirects are followed.
    canonical_moves = {"https://github.com/rust-windowing/android-ndk-rs": "https://github.com/rust-mobile/ndk"}
    repository = canonical_moves.get(repository, repository)
    parsed = urlparse(repository.removeprefix("git+"))
    if parsed.hostname != "github.com" or not re.fullmatch(r"[a-fA-F0-9]{40}", revision):
        raise ValueError("upstream notice recovery requires GitHub and a full immutable commit")
    parts = parsed.path.strip('/').split('/')
    if len(parts) < 2 or not all(re.fullmatch(r"[A-Za-z0-9_.-]+", s) for s in parts[:2]):
        raise ValueError("invalid upstream repository")
    owner, name = parts[:2]
    name = name.removesuffix(".git")
    tree_url = f"https://api.github.com/repos/{owner}/{name}/git/trees/{revision}?recursive=1"
    tree = json.loads(cached_get(tree_url, cache, 8 * 1024 * 1024))
    if tree.get("truncated"):
        raise ValueError("upstream source tree is truncated")
    scope = PurePosixPath(subdirectory)
    if scope.is_absolute() or ".." in scope.parts:
        raise ValueError("invalid published VCS subdirectory")
    ancestors = {str(p) for p in [scope, *scope.parents]}
    ancestors.add(".")
    notices = {}
    for entry in tree["tree"]:
        path = PurePosixPath(entry["path"])
        if entry.get("type") != "blob" or entry.get("mode") != "100644" or entry.get("size", MAX_NOTICE + 1) > MAX_NOTICE or path.is_absolute() or ".." in path.parts:
            continue
        parent = str(path.parent)
        license_folder = path.parent.name.lower() in {"license", "licenses"} and str(path.parent.parent) in ancestors
        if (parent in ancestors and NOTICE_NAME.match(path.name)) or license_folder:
            url = f"https://raw.githubusercontent.com/{owner}/{name}/{revision}/" + quote(str(path), safe="/")
            data = cached_get(url, cache)
            if data and b"\0" not in data:
                notices[url] = data
    return notices


def declared_terms(record: dict, source_root: Path, cache: Path) -> dict[str, bytes]:
    """Reviewed packaging omissions with explicit in-package license declarations.

    A canonical text is never attributed to the upstream package as a fictitious
    file. Its original header/copyright declaration is preserved separately.
    """
    name = record["name"]
    if name == "selectors" and record["license_declared"] == "MPL-2.0":
        source = source_root / "lib.rs"
        header = source.read_text(encoding="utf-8").split("*/", 1)[0] + "*/"
        if "This Source Code Form is subject" in header and "https://mozilla.org/MPL/2.0/" in header:
            terms = cached_get("https://raw.githubusercontent.com/spdx/license-list-data/v3.29.0/text/MPL-2.0.txt", cache)
            return {"lib.rs#original-MPL-Exhibit-A": header.encode(), "https://spdx.org/licenses/MPL-2.0.html#canonical-terms": terms}
    if name in {"keyv", "natural-compare"} and record["license_declared"] == "MIT":
        source = source_root / "README.md"
        text = source.read_text(encoding="utf-8")
        sections = re.split(r"(?im)^(?:#{1,6}\s+licen[cs]e[^\n]*\n|licen[cs]e[^\n]*\n[-=]+\s*\n)", text)
        declaration = sections[-1].strip() if len(sections) > 1 else ""
        if "MIT" in declaration and ("Copyright" in declaration or "©" in declaration):
            terms = cached_get("https://raw.githubusercontent.com/spdx/license-list-data/v3.29.0/text/MIT.txt", cache).decode()
            terms = "\n".join(line for line in terms.splitlines() if "<year>" not in line and "<copyright holders>" not in line) + "\n"
            return {"README.md#original-copyright-and-license": declaration.encode(), "https://spdx.org/licenses/MIT.html#canonical-terms-with-original-copyright-separate": terms.encode()}
    return {}


def humanfs_type_terms(record: dict, cache: Path) -> dict[str, bytes]:
    if record["name"] != "@humanfs/types" or record["license_declared"] != "Apache-2.0":
        return {}
    archive_path = cache / (hashlib.sha256(record["download"].encode()).hexdigest() + ".tgz")
    if not archive_path.is_file():
        return {}
    # archive_notices verified this exact archive's lockfile integrity first.
    with tarfile.open(archive_path) as archive:
        member = archive.getmember("package/README.md")
        if not member.isfile() or member.size > MAX_NOTICE:
            return {}
        text = archive.extractfile(member).read().decode()
    if "by [Nicholas C. Zakas]" not in text or "## License\n\nApache 2.0" not in text:
        return {}
    declaration = text.split("\n", 4)[:4] + ["", "## License", "", "Apache 2.0"]
    url = "https://raw.githubusercontent.com/spdx/license-list-data/v3.29.0/text/Apache-2.0.txt"
    return {"package/README.md#original-author-and-license": "\n".join(declaration).encode(),
            "https://spdx.org/licenses/Apache-2.0.html#canonical-terms": cached_get(url, cache)}


def archive_notices(url: str, integrity: str, cache: Path) -> dict[str, bytes]:
    parsed = urlparse(url)
    if parsed.scheme != "https" or parsed.hostname != "registry.npmjs.org" or parsed.username or parsed.port:
        raise ValueError("only the official HTTPS npm registry is allowed")
    algorithm, expected = integrity.split()[0].split("-", 1)
    if algorithm not in {"sha512", "sha256", "sha1"}:
        raise ValueError("unsupported npm integrity algorithm")
    cache.mkdir(parents=True, exist_ok=True)
    target = cache / (hashlib.sha256(url.encode()).hexdigest() + ".tgz")
    if target.exists():
        if target.stat().st_size > MAX_ARCHIVE:
            raise ValueError("cached package archive exceeds size limit")
        content = target.read_bytes()
    else:
        request = Request(url, headers={"User-Agent": "Glyphibe-license-audit/1"})
        with build_opener(NoRedirect()).open(request, timeout=30) as response:
            content = response.read(MAX_ARCHIVE + 1)
        if len(content) > MAX_ARCHIVE:
            raise ValueError("package archive exceeds 64 MiB limit")
    if base64.b64encode(hashlib.new(algorithm, content).digest()).decode() != expected:
        raise ValueError("npm archive integrity mismatch")
    if not target.exists():
        target.write_bytes(content)
    result = {}
    with tarfile.open(fileobj=io.BytesIO(content), mode="r:gz") as archive:
        uncompressed = 0
        for count, member in enumerate(archive):
            uncompressed += member.size
            if uncompressed > 512 * 1024 * 1024:
                raise ValueError("package archive uncompressed size limit exceeded")
            if count >= 100_000:
                raise ValueError("package archive has too many members")
            path = PurePosixPath(member.name)
            if not member.isfile() or path.is_absolute() or ".." in path.parts:
                continue
            if NOTICE_NAME.match(path.name) and 0 < member.size <= MAX_NOTICE:
                stream = archive.extractfile(member)
                if stream:
                    content = stream.read(MAX_NOTICE + 1)
                    if len(content) <= MAX_NOTICE and b"\0" not in content:
                        result[str(path)] = content
    return result


def cargo_metadata(manifest: Path, offline: bool):
    command = ["cargo", "metadata", "--locked", "--format-version", "1", "--manifest-path", str(manifest)]
    if offline:
        command.append("--offline")
    result = subprocess.run(command, check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=240)
    return json.loads(result.stdout)


def cargo_inventory(root: Path, offline: bool, fetch: bool = False, cache: Path | None = None):
    packages, relationships = {}, set()
    manifests = [root / "Cargo.toml", root / "apps/desktop/src-tauri/Cargo.toml"]
    for manifest in manifests:
        metadata = cargo_metadata(manifest, offline)
        lock = tomllib.loads((Path(metadata["workspace_root"]) / "Cargo.lock").read_text(encoding="utf-8"))
        checksums = {(p["name"], p["version"]): p.get("checksum") for p in lock["package"]}
        by_id = {p["id"]: p for p in metadata["packages"]}
        vcs_by_package = {}
        repository_by_commit = {}
        for p in metadata["packages"]:
            vcs = Path(p["manifest_path"]).parent / ".cargo_vcs_info.json"
            if vcs.is_file():
                data = json_read(vcs)
                vcs_by_package[p["id"]] = data
                if p.get("repository"):
                    repository_by_commit[data.get("git", {}).get("sha1")] = p["repository"]
        # An edge's dep_kind is a declaration category, not a claim that the
        # dependency is shipped in a particular final binary.
        kinds = {}
        for node in metadata["resolve"]["nodes"]:
            for dep in node["deps"]:
                kinds.setdefault(dep["pkg"], set()).update((k.get("kind") or "normal") for k in dep["dep_kinds"])
        for p in metadata["packages"]:
            if p["source"] is None:
                continue
            key = f"cargo:{p['name']}@{p['version']}"
            notice_root = Path(p["manifest_path"]).parent
            record = packages.setdefault(key, {
                "ecosystem": "cargo", "name": p["name"], "version": p["version"],
                "license_declared": p["license"], "source": p.get("repository") or "NOASSERTION",
                "download": f"https://crates.io/api/v1/crates/{quote(p['name'])}/{quote(p['version'])}/download",
                "checksum": {"algorithm": "SHA256", "value": checksums.get((p["name"], p["version"]))},
                "scopes": set(), "origins": set(), "platform_conditions": set(),
                "notices_raw": notice_files(notice_root, p.get("license_file")),
            })
            record["scopes"].update(kinds.get(p["id"], {"normal"}))
            record["origins"].add(manifest.relative_to(root).as_posix())
            if not record["notices_raw"]:
                vcs = vcs_by_package.get(p["id"], {})
                revision = vcs.get("git", {}).get("sha1")
                subdirectory = PurePosixPath(vcs.get("path_in_vcs", ""))
                for sibling in metadata["packages"]:
                    sibling_vcs = vcs_by_package.get(sibling["id"], {})
                    parent_scope = PurePosixPath(sibling_vcs.get("path_in_vcs", ""))
                    if revision and revision == sibling_vcs.get("git", {}).get("sha1") and parent_scope != subdirectory and parent_scope in subdirectory.parents and p["license"] == sibling["license"]:
                        shared = notice_files(Path(sibling["manifest_path"]).parent)
                        if shared:
                            record["notices_raw"] = {f"published-repository-root:{revision}/{name}": data for name, data in shared.items()}
                            record["notice_recovery"] = {"method": "same published immutable repository commit, ancestor package scope, identical explicit license", "revision": revision, "parent_package": sibling["name"]}
                            break
                if p["name"] in {"winapi-i686-pc-windows-gnu", "winapi-x86_64-pc-windows-gnu"}:
                    parent = next((item for item in metadata["packages"] if item["name"] == "winapi" and item.get("repository") == p.get("repository") and expression_choice(item["license"])[0] == expression_choice(p["license"])[0]), None)
                    if parent:
                        shared = notice_files(Path(parent["manifest_path"]).parent)
                        if shared:
                            record["notices_raw"] = {f"reviewed-winapi-import-library-parent:{parent['version']}/{name}": data for name, data in shared.items()}
                            record["notice_recovery"] = {"method": "reviewed winapi generated GNU import-library package; same upstream repository and explicit license; parent retains copyright texts", "parent_package": f"winapi@{parent['version']}"}
            if fetch and not record["notices_raw"]:
                supplement = declared_terms(record, notice_root, cache)
                if supplement:
                    record["notices_raw"] = supplement
                    record["notice_recovery"] = {"method": "explicit original source license header plus canonical SPDX terms; not an upstream license-file claim"}
            if fetch and not record["notices_raw"]:
                try:
                    vcs = vcs_by_package.get(p["id"], {})
                    revision = vcs.get("git", {}).get("sha1", "")
                    repository = p.get("repository") or repository_by_commit.get(revision, "")
                    record["notices_raw"] = github_notices(repository, revision, vcs.get("path_in_vcs", ""), cache)
                    record["notice_recovery"] = {"method": "published .cargo_vcs_info immutable commit", "repository": repository, "revision": revision}
                except Exception as error:
                    record["notice_error"] = f"{type(error).__name__}: {error}"
        for node in metadata["resolve"]["nodes"]:
            source = by_id[node["id"]]
            source_key = f"cargo:{source['name']}@{source['version']}"
            for dep in node["deps"]:
                target = by_id[dep["pkg"]]
                target_key = f"cargo:{target['name']}@{target['version']}"
                if source_key in packages and target_key in packages:
                    relationships.add((source_key, target_key))
                for condition in dep["dep_kinds"]:
                    if target_key in packages and condition.get("target"):
                        packages[target_key]["platform_conditions"].add(condition["target"])
    return packages, relationships


def npm_inventory(root: Path, fetch: bool, cache: Path):
    npm_root = root / "apps/desktop"
    lock = json_read(npm_root / "package-lock.json")
    entries = lock["packages"]
    packages, paths, relationships = {}, {}, set()
    for path, p in entries.items():
        if not path:
            continue
        name = p.get("name") or path.split("node_modules/")[-1]
        key = f"npm:{name}@{p['version']}"
        paths[path] = key
        record = packages.setdefault(key, {
            "ecosystem": "npm", "name": name, "version": p["version"],
            "license_declared": p.get("license"), "source": f"https://www.npmjs.com/package/{name}/v/{p['version']}",
            "download": p.get("resolved") or "NOASSERTION", "integrity": p.get("integrity"),
            "scopes": set(), "origins": {"apps/desktop/package-lock.json"},
            "platform_conditions": set(), "notices_raw": notice_files(npm_root / path),
        })
        record["scopes"].add("dev" if p.get("dev") else "normal")
        if p.get("optional"):
            record["scopes"].add("optional")
        for field in ("os", "cpu"):
            if p.get(field):
                record["platform_conditions"].add(f"{field}=" + ",".join(p[field]))
    for record in packages.values():
        if not record["notices_raw"] and fetch:
            local = npm_root / "node_modules" / record["name"]
            if local.is_dir():
                try:
                    supplement = declared_terms(record, local, cache)
                    if supplement:
                        record["notices_raw"] = supplement
                        record["notice_recovery"] = {"method": "original README copyright/license link plus canonical SPDX terms; no fictitious upstream license file"}
                except Exception as error:
                    record["notice_error"] = f"{type(error).__name__}: {error}"
        if not record["notices_raw"] and fetch and record.get("integrity") and record["download"] != "NOASSERTION":
            try:
                record["notices_raw"] = archive_notices(record["download"], record["integrity"], cache)
                if not record["notices_raw"]:
                    record["notices_raw"] = humanfs_type_terms(record, cache)
                    if record["notices_raw"]:
                        record["notice_recovery"] = {"method": "integrity-verified original README author/license declaration plus canonical Apache-2.0 terms; package omits LICENSE"}
                if not record["notices_raw"]:
                    metadata_url = f"https://registry.npmjs.org/{quote(record['name'], safe='')}/{quote(record['version'])}"
                    metadata = json.loads(cached_get(metadata_url, cache))
                    repository = metadata.get("repository", {})
                    if isinstance(repository, str):
                        repository = {"url": repository}
                    record["notices_raw"] = github_notices(repository.get("url", ""), metadata.get("gitHead", ""), repository.get("directory", ""), cache)
                    record["notice_recovery"] = {"method": "npm exact version gitHead", "repository": repository.get("url"), "revision": metadata.get("gitHead")}
            except Exception as error:
                record["notice_error"] = f"{type(error).__name__}: {error}"
    # Reviewed native wrapper families are generated artifacts of the exact
    # parent release. Require both an exact optional-dependency pin and an
    # identical declared license; generic dependency license inheritance is banned.
    for prefix, parent_name in [("@tauri-apps/cli-", "@tauri-apps/cli"), ("@rolldown/binding-", "rolldown")]:
        parent_path = f"node_modules/{parent_name}"
        parent_data = entries.get(parent_path)
        if not parent_data:
            continue
        parent = packages.get(f"npm:{parent_name}@{parent_data['version']}")
        if not parent or not parent["notices_raw"]:
            continue
        for record in packages.values():
            if record["name"].startswith(prefix) and not record["notices_raw"] and record["license_declared"] == parent["license_declared"] and parent_data.get("optionalDependencies", {}).get(record["name"]) == record["version"] == parent["version"]:
                record["notices_raw"] = {f"exact-parent-release:{parent['name']}@{parent['version']}/{name}": data for name, data in parent["notices_raw"].items()}
                record["notice_recovery"] = {"method": "reviewed generated native wrapper; exact parent optionalDependency pin and identical license", "parent": f"npm:{parent_name}@{parent['version']}", "source": parent["source"]}
                record.pop("notice_error", None)
    for path, p in entries.items():
        if not path:
            continue
        for name in {**p.get("dependencies", {}), **p.get("optionalDependencies", {})}:
            current = path
            while True:
                candidate = f"{current}/node_modules/{name}" if current else f"node_modules/{name}"
                if candidate in paths:
                    relationships.add((paths[path], paths[candidate]))
                    break
                if not current:
                    break
                current = current.rsplit("/node_modules/", 1)[0] if "/node_modules/" in current else ""
    return packages, relationships


def spdx_id(key: str) -> str:
    return "SPDXRef-" + hashlib.sha256(key.encode()).hexdigest()[:24]


def source_hash(path: Path) -> str:
    # Source manifests are text; Git checkout line-ending conversion is not a
    # dependency change. Original collected notices retain exact byte hashes.
    return hashlib.sha256(path.read_bytes().replace(b"\r\n", b"\n")).hexdigest()


def input_hashes(root: Path) -> dict[str, str]:
    paths = [root / "Cargo.toml", root / "Cargo.lock", root / "apps/desktop/src-tauri/Cargo.toml",
             root / "apps/desktop/package.json", root / "apps/desktop/package-lock.json"]
    paths += sorted((root / "crates").glob("*/Cargo.toml"))
    return {path.relative_to(root).as_posix(): source_hash(path) for path in paths}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--offline", action="store_true", help="never let Cargo download package sources")
    parser.add_argument("--check", action="store_true", help="verify committed lock-bound inventory and notice hashes without rewriting files")
    parser.add_argument("--fetch-missing-license-texts", action="store_true", help="read integrity-verified npm registry tarballs without executing code")
    parser.add_argument("--cache", type=Path, default=Path(os.environ.get("LOCALAPPDATA", str(Path.home() / ".cache"))) / "Glyphibe" / "license-audit")
    options = parser.parse_args()
    if options.offline and options.fetch_missing_license_texts:
        parser.error("--offline cannot be combined with a network fetch option")
    root = Path(__file__).resolve().parent.parent
    if options.check:
        report = json_read(root / "docs/development/dependency-inventory.json")
        errors = list(report["failures"])
        if report.get("input_sha256") != input_hashes(root):
            errors.append({"reason": "inventory stale after manifest or lockfile change"})
        for path, expected in report["lock_sha256"].items():
            if source_hash(root / path) != expected:
                errors.append({"reason": "inventory stale after lockfile change", "path": path})
        for package in report["packages"]:
            choice = expression_choice(package["license_declared"])[0]
            if choice is None or choice != package["license_selected"]:
                errors.append({"reason": "license policy changed or unapproved", "package": package["id"]})
            if not package["notices"]:
                errors.append({"reason": "missing original notice evidence", "package": package["id"]})
            for notice in package["notices"]:
                file = root / notice["file"]
                if not file.is_file() or file.is_symlink() or not file.resolve().is_relative_to(root):
                    errors.append({"reason": "missing/unsafe notice file", "package": package["id"]})
                elif hashlib.sha256(file.read_bytes()).hexdigest() != notice["sha256"]:
                    errors.append({"reason": "notice checksum mismatch", "package": package["id"]})
        print(json.dumps({"check": "passed" if not errors else "failed", "packages": len(report["packages"]), "failures": errors}, indent=2))
        return 1 if errors else 0
    generation_inputs = input_hashes(root)
    packages, dependencies = cargo_inventory(root, options.offline, options.fetch_missing_license_texts, options.cache)
    npm, npm_dependencies = npm_inventory(root, options.fetch_missing_license_texts, options.cache)
    packages.update(npm)
    dependencies.update(npm_dependencies)
    if input_hashes(root) != generation_inputs:
        raise ValueError("dependency manifests or locks changed during collection; rerun the audit")
    output = root / "docs/development"
    notices_dir = root / "third-party/licenses"
    notices_dir.mkdir(parents=True, exist_ok=True)
    inventory = []
    failures = []
    files = {}
    for key, record in sorted(packages.items()):
        choice, normalization = expression_choice(record["license_declared"])
        record["license_selected"] = choice
        record["license_normalization"] = normalization
        record["id"] = key
        record["notices"] = []
        for name, content in sorted(record.pop("notices_raw").items()):
            digest = hashlib.sha256(content).hexdigest()
            notice_path = notices_dir / f"{digest}.txt"
            notice_path.write_bytes(content)
            relative = notice_path.relative_to(root).as_posix()
            record["notices"].append({"source_file": name, "file": relative, "sha256": digest})
            files[digest] = relative
        for field in ("scopes", "origins", "platform_conditions"):
            record[field] = sorted(record[field])
        if choice is None:
            failures.append({"package": key, "reason": "unknown/disallowed SPDX expression", "declared": record["license_declared"]})
        if not record["notices"]:
            failures.append({"package": key, "reason": "source license/notice text unavailable", "detail": record.get("notice_error")})
        inventory.append(record)
    lock_hashes = {str(path.relative_to(root)).replace("\\", "/"): source_hash(path)
                   for path in [root / "Cargo.lock", root / "apps/desktop/package-lock.json"]}
    report = {"schema_version": 1, "coverage": "Cargo default enabled features with no target filter; both manifests; all npm lock entries including optional platform/build/dev packages. Disabled Cargo features are not claimed shipped.",
              "lock_sha256": lock_hashes, "input_sha256": generation_inputs, "hash_normalization": "UTF-8 manifests/locks normalize CRLF to LF; original notice text hashes are exact bytes", "packages": inventory, "failures": failures,
              "limitations": ["Manifest policy and preserved notices are engineering checks, not a blanket legal compatibility opinion.", "All discovered source license and NOTICE files are retained; embedded copyrights outside those files need release review.", "Inventory includes inactive platform and build/dev packages, not a binary composition claim."]}
    (output / "dependency-inventory.json").write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    created = dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    namespace_hash = hashlib.sha256(json.dumps(lock_hashes, sort_keys=True).encode()).hexdigest()
    spdx_packages = []
    spdx_relations = []
    for record in inventory:
        key = record["id"]
        purl = f"pkg:{record['ecosystem']}/{quote(record['name'], safe='/')}@{record['version']}"
        package = {"SPDXID": spdx_id(key), "name": record["name"], "versionInfo": record["version"],
                   "downloadLocation": record["download"], "filesAnalyzed": False,
                   "licenseDeclared": record["license_declared"].replace("/", " OR ") if record["license_declared"] else "NOASSERTION",
                   "licenseConcluded": record["license_selected"] or "NOASSERTION", "copyrightText": "NOASSERTION",
                   "externalRefs": [{"referenceCategory": "PACKAGE-MANAGER", "referenceType": "purl", "referenceLocator": purl}],
                   "comment": "Scope=" + ",".join(record["scopes"]) + "; original notice text paths are in dependency-inventory.json"}
        checksum = record.get("checksum")
        if checksum and checksum.get("value"):
            package["checksums"] = [{"algorithm": checksum["algorithm"], "checksumValue": checksum["value"]}]
        elif record.get("integrity"):
            algorithm, encoded = record["integrity"].split()[0].split("-", 1)
            package["checksums"] = [{"algorithm": algorithm.upper(), "checksumValue": base64.b64decode(encoded).hex()}]
        spdx_packages.append(package)
        spdx_relations.append({"spdxElementId": "SPDXRef-DOCUMENT", "relationshipType": "DESCRIBES", "relatedSpdxElement": spdx_id(key)})
    for source, target in sorted(dependencies):
        if source in packages and target in packages:
            spdx_relations.append({"spdxElementId": spdx_id(source), "relationshipType": "DEPENDS_ON", "relatedSpdxElement": spdx_id(target)})
    spdx = {"spdxVersion": "SPDX-2.3", "dataLicense": "CC0-1.0", "SPDXID": "SPDXRef-DOCUMENT", "name": "Glyphibe locked dependency inventory",
            "documentNamespace": f"https://github.com/styayur/glyphibe/spdx/{namespace_hash}",
            "creationInfo": {"creators": ["Tool: glyphibe-audit-dependencies-1"], "created": created},
            "packages": spdx_packages, "relationships": spdx_relations}
    (output / "dependencies.spdx.json").write_text(json.dumps(spdx, indent=2) + "\n", encoding="utf-8")
    notices = ["Glyphibe third-party notices", "===========================", "", "Generated from locked Cargo manifests and npm package-lock. Original source license/NOTICE texts are preserved unchanged in third-party/licenses/.",
               "This inventory includes build/dev and inactive platform packages. It does not assert that every listed package is in a released binary.",
               "Glyphibe source is MPL-2.0: https://github.com/styayur/glyphibe . No third-party source was modified by the notice collection process.", "", f"Policy/text collection failures: {len(failures)}. A nonzero count blocks the audit gate.", ""]
    for record in inventory:
        notices += [record["id"], f"  Declared: {record['license_declared']}; selected: {record['license_selected'] or 'UNRESOLVED'}", "  Source: " + record["source"], "  Scope: " + ", ".join(record["scopes"])]
        notices += [f"  Notice: {notice['file']} (upstream {notice['source_file']})" for notice in record["notices"]]
        notices.append("")
    (root / "THIRD_PARTY_NOTICES").write_text("\n".join(notices).rstrip() + "\n", encoding="utf-8")
    print(json.dumps({"packages": len(inventory), "unique_notice_texts": len(files), "failures": failures}, indent=2))
    return 1 if failures else 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        print(f"dependency audit failed: {type(error).__name__}: {error}", file=sys.stderr)
        raise SystemExit(2)
