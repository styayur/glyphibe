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
            (downloads / "build-provenance.json").write_text(json.dumps(provenance), encoding="utf-8")
            def seal():
                assets = [{"file": path.name, "bytes": path.stat().st_size, "sha256": verifier.digest(path)} for path in downloads.iterdir() if path.name not in ("asset-manifest.json", "SHA256SUMS")]
                (downloads / "asset-manifest.json").write_text(json.dumps({"version": version, "source_commit": commit, "assets": assets, "payloads": [{"file": name, "sha256": checksum} for name, checksum in payloads.items()]}), encoding="utf-8")
                sums = downloads / "SHA256SUMS"
                sums.write_text("".join(f"{verifier.digest(path)}  {path.name}\n" for path in sorted(downloads.iterdir()) if path.name != "SHA256SUMS"), encoding="ascii")
                return verifier.digest(sums)
            self.assertEqual(verifier.verify(downloads, version, commit, source, seal())["installer"]["sha256"], provenance["installer"]["sha256"])
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
