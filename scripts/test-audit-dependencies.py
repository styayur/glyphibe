#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Focused security and policy regression tests for notice collection."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("audit", Path(__file__).with_name("audit-dependencies.py"))
audit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(audit)


class AuditTests(unittest.TestCase):
    def test_unknown_restrictive_atoms_are_not_substring_approved(self):
        for expression in [None, "", "MIT-Noncommercial", "MIT AND LicenseRef-Secret", "PolyForm-Noncommercial-1.0.0", "AGPL-3.0-only", "MIT; RunSomething", "MIT OR", "(MIT", "MIT WITH Invented-exception"]:
            self.assertIsNone(audit.expression_choice(expression)[0], expression)

    def test_alternatives_conjunctions_and_legacy_cargo_are_explicit(self):
        self.assertEqual(audit.expression_choice("MIT OR GPL-3.0-only")[0], "MIT")
        self.assertEqual(audit.expression_choice("(Apache-2.0 OR MIT) AND Unicode-3.0")[0], "(Apache-2.0 AND Unicode-3.0)")
        self.assertEqual(audit.expression_choice("Apache-2.0 WITH LLVM-exception")[0], "Apache-2.0 WITH LLVM-exception")
        self.assertEqual(audit.expression_choice("MIT/Apache-2.0"), ("MIT", "legacy slash normalized to OR"))

    def test_source_collection_copies_notices_without_implementation(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "LICENSE-MIT").write_text("Original license and copyright text")
            (root / "run.js").write_text("executeDangerousCode();")
            result = audit.notice_files(root)
            self.assertEqual(list(result), ["LICENSE-MIT"])
            self.assertNotIn(b"executeDangerousCode", b"".join(result.values()))

    def test_notice_recovery_rejects_unpinned_or_unapproved_hosts(self):
        with tempfile.TemporaryDirectory() as directory:
            for repo, revision in [("https://example.invalid/repo", "a" * 40), ("https://github.com/a/b", "main")]:
                with self.assertRaises(ValueError):
                    audit.github_notices(repo, revision, "", Path(directory))
            with self.assertRaises(ValueError):
                audit.archive_notices("http://registry.npmjs.org/a.tgz", "sha512-xxx", Path(directory))

    def test_source_hashes_ignore_git_line_endings_without_altering_notice_hashes(self):
        with tempfile.TemporaryDirectory() as directory:
            first, second = Path(directory) / "lf", Path(directory) / "crlf"
            first.write_bytes(b"name = 'example'\nversion = '1'\n")
            second.write_bytes(b"name = 'example'\r\nversion = '1'\r\n")
            self.assertEqual(audit.source_hash(first), audit.source_hash(second))


if __name__ == "__main__":
    unittest.main()
