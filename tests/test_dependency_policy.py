# SPDX-License-Identifier: MPL-2.0
"""Expose dependency policy regressions to the repository's unittest discovery."""
import importlib.util
from pathlib import Path

source = Path(__file__).resolve().parents[1] / "scripts/test-audit-dependencies.py"
spec = importlib.util.spec_from_file_location("glyphibe_dependency_policy_tests", source)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
AuditTests = module.AuditTests
