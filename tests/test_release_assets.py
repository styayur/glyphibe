# SPDX-License-Identifier: MPL-2.0
import importlib.util
import json
from pathlib import Path
import stat
import tempfile
import unittest
import zipfile

spec = importlib.util.spec_from_file_location("release_verifier", Path(__file__).resolve().parents[1] / "scripts" / "verify_release_assets.py")
verifier = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verifier)


class ReleaseBoundaryTests(unittest.TestCase):
    def make_release(self, root):
        """Synthetic packaging inputs only; these bytes are never executable."""
        source = root / "source"
        assets = root / "downloads"
        source.mkdir()
        assets.mkdir()
        legal = {}
        for name in ("LICENSE", "THIRD_PARTY_NOTICES", "PLATFORM_NOTICES", "third-party/licenses/dependency.txt", "third-party/toolchain-licenses/native/NOTICE.txt"):
            path = source / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(f"original notice {name}", encoding="utf-8")
            legal[f"licenses/{name}"] = path.read_bytes()
        version = "1.0.0"
        commit = "a" * 40
        payload_bytes = {"glyphibe.exe": b"synthetic CLI", "glyphibe-desktop.exe": b"synthetic desktop"}
        cli = assets / f"glyphibe-{version}-windows-x64.exe"
        cli.write_bytes(payload_bytes["glyphibe.exe"])
        installer = assets / f"Glyphibe_{version}_x64-setup.exe"
        installer.write_bytes(b"synthetic installer")
        payloads = [{"file": name, "sha256": verifier.hashlib.sha256(content).hexdigest(), "private_build_path_scan": "passed"} for name, content in payload_bytes.items()]
        provenance = {"source_commit": commit, "visual_studio_products": [{"channelId": "VisualStudio.18.Release", "isPrerelease": False}], "payloads": payloads, "installer": {"file": installer.name, "sha256": verifier.digest(installer), "bytes": installer.stat().st_size, "signature_status": "NotSigned", "private_build_path_scan": "passed"}}
        (assets / "build-provenance.json").write_text(json.dumps(provenance), encoding="utf-8")
        (assets / "dependencies.spdx.json").write_text("{}", encoding="utf-8")
        for kind in ("portable", "cli", "licenses"):
            name = f"Glyphibe-{version}-licenses.zip" if kind == "licenses" else f"Glyphibe-{version}-windows-x64-{kind}.zip"
            with zipfile.ZipFile(assets / name, "w") as archive:
                for path, content in legal.items():
                    archive.writestr(path, content)
                if kind != "licenses":
                    archive.writestr("README.txt", "synthetic packaging test")
                    archive.writestr("glyphibe.exe", payload_bytes["glyphibe.exe"])
                if kind == "portable":
                    archive.writestr("glyphibe-desktop.exe", payload_bytes["glyphibe-desktop.exe"])
        return source, assets, version, commit, payloads

    def refresh_inventory(self, assets, version, commit, payloads):
        files = sorted(path for path in assets.iterdir() if path.is_file() and path.name not in ("SHA256SUMS", "asset-manifest.json"))
        manifest = {"version": version, "source_commit": commit, "payloads": payloads, "assets": [{"file": path.name, "bytes": path.stat().st_size, "sha256": verifier.digest(path)} for path in files]}
        (assets / "asset-manifest.json").write_text(json.dumps(manifest), encoding="utf-8")
        sums = "".join(f"{verifier.digest(path)}  {path.name}\n" for path in sorted(assets.iterdir()) if path.is_file() and path.name != "SHA256SUMS")
        (assets / "SHA256SUMS").write_text(sums, encoding="ascii")
        return verifier.digest(assets / "SHA256SUMS")

    def test_complete_inventory_binds_installer_to_original_build(self):
        with tempfile.TemporaryDirectory() as directory:
            source, assets, version, commit, payloads = self.make_release(Path(directory))
            trusted = self.refresh_inventory(assets, version, commit, payloads)
            report = verifier.verify(assets, version, commit, source, trusted)
            self.assertEqual(report["legal_files_per_archive"], 5)
            installer = assets / f"Glyphibe_{version}_x64-setup.exe"
            installer.write_bytes(b"substituted installer")
            trusted = self.refresh_inventory(assets, version, commit, payloads)
            # A consistent new asset manifest/checksum cannot replace old build evidence.
            with self.assertRaisesRegex(ValueError, "installer differs"):
                verifier.verify(assets, version, commit, source, trusted)

    def test_installer_provenance_is_required_and_exact(self):
        with tempfile.TemporaryDirectory() as directory:
            source, assets, version, commit, payloads = self.make_release(Path(directory))
            path = assets / "build-provenance.json"
            original = json.loads(path.read_text(encoding="utf-8"))
            for field, value in (("file", "unrelated.exe"), ("bytes", 0), ("private_build_path_scan", "failed")):
                provenance = json.loads(json.dumps(original))
                provenance["installer"][field] = value
                path.write_text(json.dumps(provenance), encoding="utf-8")
                trusted = self.refresh_inventory(assets, version, commit, payloads)
                with self.assertRaisesRegex(ValueError, "installer differs"):
                    verifier.verify(assets, version, commit, source, trusted)
            original.pop("installer")
            path.write_text(json.dumps(original), encoding="utf-8")
            trusted = self.refresh_inventory(assets, version, commit, payloads)
            with self.assertRaisesRegex(ValueError, "installer differs"):
                verifier.verify(assets, version, commit, source, trusted)

    def test_report_is_external_fresh_and_exclusively_created(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            downloads = root / "downloads"
            downloads.mkdir()
            asset = downloads / "SHA256SUMS"
            asset.write_text("verified input", encoding="utf-8")
            for output in (asset, downloads / "new-report.json", downloads / ".." / "downloads" / "alias.json"):
                with self.assertRaisesRegex(ValueError, "outside"):
                    verifier.report_path(downloads, output)
            external = root / "report.json"
            destination = verifier.report_path(downloads, external)
            external.write_text("created after validation", encoding="utf-8")
            with self.assertRaises(FileExistsError):
                verifier.write_report(destination, {"result": "passed"})
            with self.assertRaisesRegex(ValueError, "fresh"):
                verifier.report_path(downloads, external)
            self.assertEqual(external.read_text(encoding="utf-8"), "created after validation")
            self.assertEqual(asset.read_text(encoding="utf-8"), "verified input")
            fresh = verifier.report_path(downloads, root / "fresh.json")
            verifier.write_report(fresh, {"result": "passed"})
            self.assertEqual(json.loads(fresh.read_text(encoding="utf-8"))["result"], "passed")

    def test_report_cannot_overwrite_verified_inputs_or_an_existing_file(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            downloads = root / "downloads"
            downloads.mkdir()
            for output in (downloads / "SHA256SUMS", downloads / "nested" / "report.json"):
                with self.assertRaises(ValueError):
                    verifier.report_path(downloads, output)
            existing = root / "existing.json"
            existing.write_text("preserve", encoding="utf-8")
            with self.assertRaises(ValueError):
                verifier.report_path(downloads, existing)
            output = verifier.report_path(downloads, root / "fresh.json")
            verifier.write_report(output, {"result": "passed"})
            with self.assertRaises(FileExistsError):
                verifier.write_report(output, {"result": "replaced"})
            self.assertEqual(json.loads(output.read_text())["result"], "passed")
            self.assertEqual(existing.read_text(), "preserve")

    def test_outer_checksums_do_not_replace_installer_build_provenance(self):
        # Harmless non-executable bytes test the distribution chain, not the app.
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source, downloads = root / "source", root / "downloads"
            source.mkdir()
            downloads.mkdir()
            for directory in ("licenses", "toolchain-licenses"):
                (source / "third-party" / directory).mkdir(parents=True)
            legal = {}
            for name in ("LICENSE", "THIRD_PARTY_NOTICES", "PLATFORM_NOTICES"):
                data = f"original test notice {name}".encode()
                (source / name).write_bytes(data)
                legal[f"licenses/{name}"] = data
            version, commit = "1.0.0", "a" * 40
            cli = downloads / f"glyphibe-{version}-windows-x64.exe"
            cli.write_bytes(b"non-executable CLI identity fixture")
            desktop = b"non-executable desktop identity fixture"
            installer = downloads / f"Glyphibe_{version}_x64-setup.exe"
            installer.write_bytes(b"non-executable installer identity fixture")
            import hashlib
            payloads = {"glyphibe.exe": verifier.digest(cli), "glyphibe-desktop.exe": hashlib.sha256(desktop).hexdigest()}
            for kind in ("portable", "cli", "licenses"):
                filename = f"Glyphibe-{version}-licenses.zip" if kind == "licenses" else f"Glyphibe-{version}-windows-x64-{kind}.zip"
                with zipfile.ZipFile(downloads / filename, "w") as archive:
                    for name, data in legal.items():
                        archive.writestr(name, data)
                    if kind != "licenses":
                        archive.writestr("README.txt", "test distribution")
                        archive.writestr("glyphibe.exe", cli.read_bytes())
                    if kind == "portable":
                        archive.writestr("glyphibe-desktop.exe", desktop)
            (downloads / "dependencies.spdx.json").write_text("{}", encoding="utf-8")
            provenance = {"source_commit": commit, "payloads": [{"file": name, "sha256": checksum, "private_build_path_scan": "passed"} for name, checksum in payloads.items()], "installer": {"file": installer.name, "sha256": verifier.digest(installer), "bytes": installer.stat().st_size, "private_build_path_scan": "passed"}}
            provenance["visual_studio_products"] = [{"channelId": "VisualStudio.18.Release", "isPrerelease": False}]
            (downloads / "build-provenance.json").write_text(json.dumps(provenance), encoding="utf-8")
            def seal():
                assets = [{"file": path.name, "bytes": path.stat().st_size, "sha256": verifier.digest(path)} for path in downloads.iterdir() if path.name not in ("asset-manifest.json", "SHA256SUMS")]
                (downloads / "asset-manifest.json").write_text(json.dumps({"version": version, "source_commit": commit, "assets": assets, "payloads": [{"file": name, "sha256": checksum} for name, checksum in payloads.items()]}), encoding="utf-8")
                sums = downloads / "SHA256SUMS"
                sums.write_text("".join(f"{verifier.digest(path)}  {path.name}\n" for path in sorted(downloads.iterdir()) if path.name != "SHA256SUMS"), encoding="ascii")
                return verifier.digest(sums)
            self.assertEqual(verifier.verify(downloads, version, commit, source, seal())["installer"]["sha256"], provenance["installer"]["sha256"])
            for channel, prerelease in (("VisualStudio.18.Insiders", False), ("VisualStudio.18.Release", True)):
                provenance["visual_studio_products"] = [{"channelId": channel, "isPrerelease": prerelease}]
                (downloads / "build-provenance.json").write_text(json.dumps(provenance), encoding="utf-8")
                with self.assertRaisesRegex(ValueError, "stable Visual Studio"):
                    verifier.verify(downloads, version, commit, source, seal())
            provenance["visual_studio_products"] = [{"channelId": "VisualStudio.18.Release", "isPrerelease": False}]
            (downloads / "build-provenance.json").write_text(json.dumps(provenance), encoding="utf-8")
            installer.write_bytes(b"replaced installer after build")
            with self.assertRaisesRegex(ValueError, "installer differs"):
                verifier.verify(downloads, version, commit, source, seal())

    def test_checksum_records_are_unique_and_root_scoped(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "SHA256SUMS"
            path.write_text("a" * 64 + "  binary.exe\n", encoding="ascii")
            self.assertEqual(verifier.read_sums(path), {"binary.exe": "a" * 64})
            for content in ("a" * 64 + "  ../escape\n", ("a" * 64 + "  binary.exe\n") * 2, "", "a" * 64 + "  C:escape\n"):
                path.write_text(content, encoding="ascii")
                with self.assertRaises(ValueError):
                    verifier.read_sums(path)

    def test_archive_rejects_traversal_symlinks_and_duplicate_entries(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "test.zip"
            for name in ("../escape", "/absolute", "C:/absolute", "a\\..\\escape"):
                with zipfile.ZipFile(path, "w") as archive:
                    archive.writestr(name, "untrusted")
                with self.assertRaises(ValueError):
                    verifier.archive_hashes(path)
            link = zipfile.ZipInfo("link")
            link.create_system = 3
            link.external_attr = (stat.S_IFLNK | 0o777) << 16
            with zipfile.ZipFile(path, "w") as archive:
                archive.writestr(link, "outside")
            with self.assertRaises(ValueError):
                verifier.archive_hashes(path)
            with self.assertWarns(UserWarning):
                with zipfile.ZipFile(path, "w") as archive:
                    archive.writestr("duplicate", "first")
                    archive.writestr("duplicate", "second")
            with self.assertRaises(ValueError):
                verifier.archive_hashes(path)
            with zipfile.ZipFile(path, "w") as archive:
                archive.writestr("licenses/LICENSE", "original terms")
            self.assertEqual(set(verifier.archive_hashes(path)), {"licenses/LICENSE"})

    def test_cli_execution_requires_a_separately_trusted_hash(self):
        with tempfile.TemporaryDirectory() as directory:
            binary = Path(directory) / "never-execute.exe"
            binary.write_bytes(b"untrusted data")
            with self.assertRaises(ValueError):
                verifier.exercise_cli(binary, "0" * 64, Path(directory))


if __name__ == "__main__":
    unittest.main()
