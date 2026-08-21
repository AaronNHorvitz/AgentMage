"""Tests for the frozen macOS release manifest field contract (schema version 3)."""

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = ROOT / "scripts/macos_platform_manifest_contract.py"
SPEC = importlib.util.spec_from_file_location(
    "macos_platform_manifest_contract", MODULE_PATH
)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class MacosPlatformManifestContractTests(unittest.TestCase):
    def setUp(self) -> None:
        self.freeze_text = MODULE.load_text(MODULE.FREEZE_DOC)
        self.v3_readme_text = MODULE.load_text(MODULE.V3_README)
        self.v3_entries = MODULE.enumerate_v3_entries()

    def test_committed_freeze_and_reservation_pass_contract(self) -> None:
        self.assertEqual(MODULE.run_contract(), [])

    def test_freeze_document_holds_every_required_field(self) -> None:
        self.assertEqual(MODULE.validate_freeze_document(self.freeze_text), [])
        for token in MODULE.REQUIRED_SHARED_FIELDS:
            self.assertIn(token, self.freeze_text)
        for token in MODULE.REQUIRED_SIGNING_FIELDS:
            self.assertIn(token, self.freeze_text)
        for prefix in MODULE.CANONICAL_PREIMAGE_PREFIXES:
            self.assertIn(prefix, self.freeze_text)
        for section in MODULE.REQUIRED_SECTIONS:
            self.assertIn(section, self.freeze_text)

    def test_freeze_document_binds_platform_family_and_architecture(self) -> None:
        self.assertIn(MODULE.FROZEN_PLATFORM_FAMILY, self.freeze_text)
        self.assertIn(MODULE.FROZEN_ARCHITECTURE, self.freeze_text)
        self.assertIn(MODULE.DOMAIN_SEPARATOR_V3, self.freeze_text)
        self.assertIn(MODULE.DOMAIN_SEPARATOR_V2, self.freeze_text)
        self.assertNotEqual(MODULE.DOMAIN_SEPARATOR_V2, MODULE.DOMAIN_SEPARATOR_V3)

    def test_freeze_document_pins_closure_counts(self) -> None:
        self.assertIn(MODULE.FROZEN_CAPABILITY_COUNT, self.freeze_text)
        self.assertIn(MODULE.FROZEN_RUNTIME_CLOSURE_COUNT, self.freeze_text)
        self.assertIn(MODULE.FROZEN_ADDITIONAL_INVENTORY_COUNT, self.freeze_text)
        self.assertIn(MODULE.FROZEN_ENTITLEMENT_COMPONENT_COUNT, self.freeze_text)

    def test_freeze_document_lists_every_failure_code(self) -> None:
        for code in MODULE.FROZEN_FAILURE_CODES:
            self.assertIn(code, self.freeze_text)

    def test_dropping_any_required_field_fails_contract(self) -> None:
        for field in MODULE.REQUIRED_SHARED_FIELDS + MODULE.REQUIRED_SIGNING_FIELDS:
            mutated = self.freeze_text.replace(field, "``")
            failures = MODULE.validate_freeze_document(mutated)
            self.assertTrue(
                failures, f"removing {field} must fail the freeze contract"
            )

    def test_dropping_any_canonical_preimage_fails_contract(self) -> None:
        for prefix in MODULE.CANONICAL_PREIMAGE_PREFIXES:
            mutated = self.freeze_text.replace(prefix, "agentmage.removed.prefix")
            failures = MODULE.validate_freeze_document(mutated)
            self.assertTrue(failures, f"removing {prefix} must fail")

    def test_dropping_domain_separator_fails_contract(self) -> None:
        mutated = self.freeze_text.replace(MODULE.DOMAIN_SEPARATOR_V3, "")
        self.assertTrue(MODULE.validate_freeze_document(mutated))

    def test_dropping_any_section_heading_fails_contract(self) -> None:
        for section in MODULE.REQUIRED_SECTIONS:
            mutated = self.freeze_text.replace(section, "## Removed")
            failures = MODULE.validate_freeze_document(mutated)
            self.assertTrue(failures, f"removing {section} must fail")

    def test_dropping_rosetta_rejection_fails_contract(self) -> None:
        for token in MODULE.FORBIDDEN_TOKENS:
            mutated = self.freeze_text.replace(token, "")
            failures = MODULE.validate_freeze_document(mutated)
            self.assertTrue(failures)

    def test_dropping_closure_counts_fails_contract(self) -> None:
        for token in (
            MODULE.FROZEN_CAPABILITY_COUNT,
            MODULE.FROZEN_RUNTIME_CLOSURE_COUNT,
            MODULE.FROZEN_ADDITIONAL_INVENTORY_COUNT,
            MODULE.FROZEN_ENTITLEMENT_COMPONENT_COUNT,
        ):
            mutated = self.freeze_text.replace(token, "some other count")
            self.assertTrue(MODULE.validate_freeze_document(mutated))

    def test_v3_readme_declares_reservation(self) -> None:
        for token in MODULE.V3_README_REQUIRED_TOKENS:
            self.assertIn(token, self.v3_readme_text)

    def test_v3_directory_excludes_manifest_and_key_artifacts(self) -> None:
        for entry in self.v3_entries:
            if not entry.is_file():
                continue
            self.assertNotIn(entry.suffix.lower(), MODULE.V3_FORBIDDEN_SUFFIXES)

    def test_v3_reservation_reports_missing_readme(self) -> None:
        failures = MODULE.validate_v3_reservation("", self.v3_entries)
        self.assertTrue(failures)

    def test_v3_reservation_reports_forbidden_artifact(self) -> None:
        pretend_manifest = MODULE.V3_DIR / "macos-apple-silicon.json"
        failures = MODULE.validate_v3_reservation(
            self.v3_readme_text, self.v3_entries + (pretend_manifest,)
        )
        note = "committed json artifact must fail the v3 reservation"
        self.assertTrue(
            any("forbidden .json artifact" in failure for failure in failures), note
        )

    def test_dropping_domain_separator_pointer_from_v3_readme_fails(self) -> None:
        mutated = self.v3_readme_text.replace(MODULE.DOMAIN_SEPARATOR_V3, "")
        self.assertTrue(MODULE.validate_v3_reservation(mutated, self.v3_entries))

    def test_v3_readme_points_at_freeze_document(self) -> None:
        self.assertIn("../v2/macos-fields.md", self.v3_readme_text)

    def test_v3_domain_separator_distinct_from_v2(self) -> None:
        self.assertNotEqual(MODULE.DOMAIN_SEPARATOR_V2, MODULE.DOMAIN_SEPARATOR_V3)
        self.assertIn(".v3", MODULE.DOMAIN_SEPARATOR_V3)
        self.assertIn(".v2", MODULE.DOMAIN_SEPARATOR_V2)


if __name__ == "__main__":
    unittest.main()
