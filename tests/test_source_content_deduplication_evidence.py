from __future__ import annotations

import copy
import io
import unittest
from unittest import mock

from scripts import source_content_deduplication_evidence as deduplication
from scripts import source_lifecycle_transactions_evidence as lifecycle
from scripts import workflow_attempt_invariants_evidence as attempts

from scripts.source_content_deduplication_evidence import (
    MARKERS,
    MIGRATION_PATH,
    REQUIRED_MIGRATION_FRAGMENTS,
    expected_report,
    validate_migration,
    validate_raw,
    validate_report,
)


class SourceContentDeduplicationEvidenceTests(unittest.TestCase):
    def test_related_builders_bind_current_upgrade_and_complete_schema_inputs(self) -> None:
        for module in (deduplication, lifecycle, attempts):
            with self.subTest(module=module.__name__):
                source = (module.ROOT / "kernel/engine/src/operational_store.rs").read_text()
                upgrade = "version_one_upgrades_through_nineteen_with_exact_history"
                self.assertIn(f"fn {upgrade}()", source)
                self.assertTrue(any(f"operational_store::tests::{upgrade}" in command for command in module.COMMANDS))
                self.assertIn(f"{upgrade} ... ok", module.MARKERS)
                report = module.expected_report()
                self.assertEqual(report["operational_store_schema_version"], 19)
                paths = {item["path"] for item in report["artifacts"]}
                self.assertTrue({
                    "kernel/engine/fixtures/operational-store/schema-18.json",
                    "kernel/engine/fixtures/operational-store/schema-19.json",
                    "kernel/engine/migrations/operational-store/0019-research-budgets.sql",
                    "kernel/engine/src/research_journal.rs",
                } <= paths)

    def test_marker_rejection_retains_raw_output_without_publishing_a_report(self) -> None:
        for module in (deduplication, lifecycle, attempts):
            with self.subTest(module=module.__name__):
                raw = "retained synthetic output with missing required tests\n"
                stderr = io.StringIO()
                with (
                    mock.patch.object(module, "capture", return_value=(raw, 0)),
                    mock.patch.object(module.sys, "argv", ["evidence", "--write"]),
                    mock.patch.object(module.sys, "stderr", stderr),
                    mock.patch.object(module, "RAW_PATH") as raw_path,
                    mock.patch.object(module, "REPORT_PATH") as report_path,
                ):
                    self.assertEqual(module.main(), 1)
                self.assertIn(raw, stderr.getvalue())
                self.assertIn("raw results missing marker", stderr.getvalue())
                raw_path.write_text.assert_not_called()
                report_path.write_text.assert_not_called()

    def test_current_migration_and_report_are_exact(self) -> None:
        self.assertEqual(validate_migration(MIGRATION_PATH.read_text()), [])
        self.assertEqual(validate_report(expected_report()), [])

    def test_missing_identity_constraint_is_rejected(self) -> None:
        migration = MIGRATION_PATH.read_text()
        for fragment in REQUIRED_MIGRATION_FRAGMENTS:
            self.assertTrue(validate_migration(migration.replace(fragment, "removed", 1)), fragment)

    def test_second_payload_authority_is_rejected(self) -> None:
        migration = MIGRATION_PATH.read_text()
        self.assertTrue(validate_migration(f"{migration}\nCREATE TABLE source_payloads (payload BLOB);"))

    def test_product_completion_overclaim_is_rejected(self) -> None:
        for field in (
            "refresh_invalidation_lifecycle_complete",
            "story_completion_claim",
            "sprint_completion_claim",
        ):
            changed = copy.deepcopy(expected_report())
            changed["product_truth"][field] = True
            self.assertTrue(validate_report(changed), field)
        changed = copy.deepcopy(expected_report())
        changed["product_truth"]["release_claim"] = "ga"
        self.assertTrue(validate_report(changed))

    def test_raw_results_require_every_marker_and_no_failure(self) -> None:
        valid = "\n".join(MARKERS)
        self.assertEqual(validate_raw(valid), [])
        self.assertTrue(validate_raw(valid.replace(MARKERS[0], "")))
        self.assertTrue(validate_raw(f"{valid}\ntest result: FAILED"))


if __name__ == "__main__":
    unittest.main()
