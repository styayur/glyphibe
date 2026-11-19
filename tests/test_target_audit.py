# SPDX-License-Identifier: MPL-2.0
"""Shipping graph selection must neither retain phantom edges nor hide packages."""
import importlib.util
from pathlib import Path
import unittest
import json
import subprocess
import tempfile

spec = importlib.util.spec_from_file_location("target_audit", Path(__file__).resolve().parents[1] / "scripts/audit-target-dependencies.py")
audit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(audit)


class TargetAuditTests(unittest.TestCase):
    def test_real_cargo_weak_optional_feature_selection(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in ("root", "middle", "leaf"):
                package = root / name
                (package / "src").mkdir(parents=True)
                (package / "src/lib.rs").write_text("")
                (package / "Cargo.toml").write_text(f'[package]\nname="{name}"\nversion="1.0.0"\nedition="2024"\n[workspace]\n')
            with (root / "root/Cargo.toml").open("a") as file:
                file.write('[dependencies]\nmiddle={path="../middle"}\n[features]\nactive=["middle/leaf"]\n')
            with (root / "middle/Cargo.toml").open("a") as file:
                file.write('[dependencies]\nleaf={path="../leaf",optional=true}\n[features]\ndefault=["std"]\nstd=["leaf?/std"]\n')
            with (root / "leaf/Cargo.toml").open("a") as file:
                file.write('[features]\nstd=[]\n')
            for features, expected in [([], {"root", "middle"}), (["--features", "active"], {"root", "middle", "leaf"})]:
                common = ["--offline", "--manifest-path", str(root / "root/Cargo.toml"), *features]
                metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--format-version", "1", *common], stderr=subprocess.PIPE))
                tree = subprocess.check_output(["cargo", "tree", "--edges", "normal,build", "--prefix", "none", "--format", "{p}", *common], text=True, stderr=subprocess.PIPE)
                packages = {p["id"]: p for p in metadata["packages"]}
                actual = audit.select_tree_packages(tree, packages)
                self.assertEqual({packages[p]["name"] for p in actual}, expected)

    def test_disabled_dependency_is_excluded_but_enabled_one_is_kept(self):
        packages = {"root": {"name": "root", "version": "1.0.0"},
                    "optional": {"name": "optional", "version": "2.0.0"}}
        self.assertEqual(audit.select_tree_packages("root v1.0.0 (/source)\n", packages), {"root"})
        self.assertEqual(audit.select_tree_packages("root v1.0.0 (/source)\noptional v2.0.0\noptional v2.0.0 (*)\n", packages), {"root", "optional"})

    def test_unknown_ambiguous_empty_and_malformed_graphs_fail_closed(self):
        packages = {"one": {"name": "same", "version": "1.0.0"},
                    "two": {"name": "same", "version": "1.0.0"}}
        for text in ["", "same v1.0.0\n", "missing v1.0.0\n", "unexpected output\n"]:
            with self.assertRaises(ValueError):
                audit.select_tree_packages(text, packages)
