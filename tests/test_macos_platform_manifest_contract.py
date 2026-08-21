"""Tests for the frozen macOS release manifest field contract (schema version 3)."""

from __future__ import annotations

import importlib.util
import tempfile
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

    def test_committed_freeze_declares_exactly_the_closed_field_set(self) -> None:
        self.assertEqual(
            MODULE.extract_declared_fields(self.freeze_text),
            set(MODULE.FROZEN_MANIFEST_FIELDS),
        )

    def test_adding_unknown_manifest_field_fails_contract(self) -> None:
        addition = "\n- `unauthorized_new_field`: SHA-256 of something not in the freeze.\n"
        mutated = self.freeze_text.replace(
            "- `team_id`:",
            addition + "- `team_id`:",
            1,
        )
        failures = MODULE.validate_freeze_document(mutated)
        self.assertTrue(
            any("unauthorized manifest field" in failure for failure in failures),
            "adding an unknown declared field must be reported as an addition",
        )

    def test_removing_field_declaration_leaving_reference_fails_contract(self) -> None:
        bullet = "- `schema_version`: integer `3`."
        self.assertIn(bullet, self.freeze_text)
        mutated = self.freeze_text.replace(
            bullet,
            "- integer three (`schema_version` referenced but no longer declared).",
        )
        self.assertIn("schema_version", mutated)
        failures = MODULE.validate_freeze_document(mutated)
        self.assertTrue(
            any(
                "missing declaration of frozen field: schema_version" in failure
                for failure in failures
            ),
            "removing a declaration bullet must fail even when the identifier "
            "still appears in prose",
        )

    def test_reordering_frozen_component_array_fails_contract(self) -> None:
        original = "`host`, `bridge`, `xpc_helper`, `inference` in that exact order"
        self.assertIn(original, self.freeze_text)
        mutated = self.freeze_text.replace(
            original,
            "`bridge`, `host`, `xpc_helper`, `inference` in that exact order",
        )
        failures = MODULE.validate_freeze_document(mutated)
        self.assertTrue(
            any("frozen component ordering" in failure for failure in failures),
            "reordering the runtime closure order must fail the freeze contract",
        )

    def test_renaming_canonical_preimage_key_fails_contract(self) -> None:
        mutated = self.freeze_text.replace(
            "xcode-command-line-tools-build=<XcodeCLTBuild>",
            "unexpected-key=<XcodeCLTBuild>",
        )
        failures = MODULE.validate_freeze_document(mutated)
        self.assertTrue(
            any(
                "exact canonical preimage string" in failure for failure in failures
            ),
            "renaming a canonical-preimage key must fail the freeze contract",
        )

    def test_weakening_authoritative_source_rule_fails_contract(self) -> None:
        mutated = self.freeze_text.replace(
            "Contents/Resources/app/product.json",
            "some-other-source.txt",
        )
        failures = MODULE.validate_freeze_document(mutated)
        self.assertTrue(
            any(
                "weakened an authoritative source rule" in failure
                for failure in failures
            ),
            "weakening the Visual Studio Code product.json source must fail",
        )

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

    def test_v3_reservation_rejects_nested_forbidden_artifact(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "README.md").write_text("stub", encoding="utf-8")
            fixtures = root / "fixtures"
            fixtures.mkdir()
            nested = fixtures / "macos-apple-silicon.json"
            nested.write_text("{}", encoding="utf-8")
            entries = tuple(sorted(root.rglob("*")))
            failures = MODULE.validate_v3_reservation(self.v3_readme_text, entries)
            self.assertTrue(
                any(
                    "forbidden .json artifact" in failure
                    and "fixtures/macos-apple-silicon.json" in failure
                    for failure in failures
                ),
                "nested forbidden artifacts must be reported with their "
                "relative path, not silently accepted",
            )

    def test_v3_reservation_descends_into_directory_entries(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            nested_dir = root / "fixtures"
            nested_dir.mkdir()
            (nested_dir / "macos.json").write_text("{}", encoding="utf-8")
            failures = MODULE.validate_v3_reservation(
                self.v3_readme_text, (nested_dir,)
            )
            self.assertTrue(
                any("forbidden .json artifact" in failure for failure in failures),
                "passing a directory entry must trigger recursive descent",
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

    def test_unknown_field_under_build_boundary_section_fails_contract(self) -> None:
        """CM-7.1.1.2-001: unauthorized manifest-field declarations outside the
        previously allowlisted sections must still be rejected."""
        addition = "\n- `unauthorized_new_field`: required manifest field.\n"
        mutated = self.freeze_text.replace(
            "## macOS Build Boundary\n",
            "## macOS Build Boundary\n" + addition,
        )
        self.assertNotEqual(mutated, self.freeze_text)
        failures = MODULE.validate_freeze_document(mutated)
        self.assertTrue(
            any(
                "unauthorized manifest field" in failure
                and "unauthorized_new_field" in failure
                for failure in failures
            ),
            "unauthorized field added under macOS Build Boundary must fail",
        )

    def test_unknown_field_in_newly_added_section_fails_contract(self) -> None:
        """CM-7.1.1.2-001: adding a whole new section with a manifest-field
        declaration bullet must still be rejected."""
        addition = (
            "\n## Sneaky Additional Manifest Fields\n\n"
            "- `unauthorized_new_field`: required manifest field.\n"
        )
        mutated = self.freeze_text + addition
        failures = MODULE.validate_freeze_document(mutated)
        self.assertTrue(
            any(
                "unauthorized manifest field" in failure
                and "unauthorized_new_field" in failure
                for failure in failures
            ),
            "unauthorized field in a newly added section must fail",
        )

    def test_duplicate_section_heading_cannot_hide_unauthorized_field(
        self,
    ) -> None:
        """CM-7.1.1.2-003: appending a second occurrence of an existing
        heading must not let an unauthorized manifest-field declaration
        bypass the closed-field-set validation."""
        addition = (
            "\n## Shared Runtime Identity\n\n"
            "- `unauthorized_new_field`: required manifest field.\n"
        )
        mutated = self.freeze_text + addition
        failures = MODULE.validate_freeze_document(mutated)
        self.assertTrue(
            any(
                "unauthorized manifest field" in failure
                and "unauthorized_new_field" in failure
                for failure in failures
            ),
            "duplicate heading must not hide an unauthorized manifest field",
        )

    def test_adapter_observation_identifier_is_not_treated_as_manifest_field(
        self,
    ) -> None:
        """Separately typed adapter-observation identifiers must not be
        reported as manifest-field additions if a future edit ever declares
        one with a bullet head."""
        addition = (
            "\n- `observed_os_build_sha256`: adapter observation, separately "
            "typed from the manifest.\n"
        )
        mutated = self.freeze_text.replace(
            "## macOS Build Boundary\n",
            "## macOS Build Boundary\n" + addition,
        )
        failures = MODULE.validate_freeze_document(mutated)
        self.assertFalse(
            any(
                "unauthorized manifest field" in failure
                and "observed_os_build_sha256" in failure
                for failure in failures
            ),
            "allowlisted adapter-observation identifier must not be flagged",
        )

    def test_installed_closure_member_grammar_bound_to_owning_field(self) -> None:
        """CM-7.1.1.2-002: mutating only the installed-closure member-line key
        while leaving the additional-signed-inventory member line intact must
        fail specifically for the installed-closure preimage."""
        installed_bullet_start = self.freeze_text.index(
            "- `installed_closure_sha256`:"
        )
        additional_bullet_start = self.freeze_text.index(
            "- `additional_signed_inventory_sha256`:"
        )
        self.assertLess(installed_bullet_start, additional_bullet_start)
        member_line = (
            "`component=<name> path=<installed-relative-path> "
            "sha256=<lowercase-hex>\\n`"
        )
        replacement = (
            "`runtime=<name> path=<installed-relative-path> "
            "sha256=<lowercase-hex>\\n`"
        )
        prefix = self.freeze_text[:installed_bullet_start]
        middle = self.freeze_text[installed_bullet_start:additional_bullet_start]
        suffix = self.freeze_text[additional_bullet_start:]
        self.assertIn(member_line, middle)
        mutated_middle = middle.replace(member_line, replacement, 1)
        mutated = prefix + mutated_middle + suffix
        self.assertIn(
            member_line,
            mutated,
            "additional-signed-inventory member line must remain unchanged",
        )
        failures = MODULE.validate_freeze_document(mutated)
        installed_failures = [
            failure
            for failure in failures
            if "installed_closure_sha256" in failure
            and "canonical preimage" in failure
        ]
        self.assertTrue(
            installed_failures,
            "installed-closure preimage must be reported when only its "
            "member-line key drifts",
        )
        additional_failures = [
            failure
            for failure in failures
            if "additional_signed_inventory_sha256" in failure
            and "canonical preimage" in failure
        ]
        self.assertFalse(
            additional_failures,
            "unchanged additional-signed-inventory preimage must not be "
            "reported when only the installed-closure member line drifts",
        )


if __name__ == "__main__":
    unittest.main()
