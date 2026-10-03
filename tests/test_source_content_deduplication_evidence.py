from __future__ import annotations

import copy
import io
import subprocess
import unittest
from pathlib import Path
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
                upgrade = "version_one_upgrades_through_twenty_five_with_exact_history"
                self.assertIn(f"fn {upgrade}()", source)
                self.assertTrue(any(f"operational_store::tests::{upgrade}" in command for command in module.COMMANDS))
                self.assertIn(f"{upgrade} ... ok", module.MARKERS)
                report = module.expected_report()
                self.assertEqual(report["operational_store_schema_version"], 25)
                paths = {item["path"] for item in report["artifacts"]}
                self.assertTrue({
                    "kernel/engine/fixtures/operational-store/schema-18.json",
                    "kernel/engine/fixtures/operational-store/schema-19.json",
                    "kernel/engine/fixtures/operational-store/schema-20.json",
                    "kernel/engine/migrations/operational-store/0020-research-draft-readers.sql",
                    "kernel/engine/fixtures/operational-store/schema-21.json",
                    "kernel/engine/migrations/operational-store/0021-job-control-ledgers.sql",
                    "kernel/engine/src/job_ledger_store.rs",
                    "kernel/engine/fixtures/operational-store/schema-22.json",
                    "kernel/engine/migrations/operational-store/0022-run-action-histories.sql",
                    "kernel/engine/src/run_action_history_store.rs",
                    "kernel/engine/fixtures/operational-store/schema-23.json",
                    "kernel/engine/migrations/operational-store/0023-documentation-packs.sql",
                    "kernel/engine/src/doc_pack_store.rs",
                    "kernel/engine/fixtures/operational-store/schema-24.json",
                    "kernel/engine/migrations/operational-store/0024-owner-states.sql",
                    "kernel/engine/src/owner_state_store.rs",
                    "kernel/engine/fixtures/operational-store/schema-25.json",
                    "kernel/engine/migrations/operational-store/0025-run-effect-records.sql",
                    "kernel/engine/src/run_effect_record_store.rs",
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

    def test_raw_results_never_name_the_checkout_or_home_directory(self) -> None:
        # Review F2 of 6c0f51fe: Cargo's progress lines name the checkout.
        for module in (deduplication, lifecycle, attempts):
            with self.subTest(module=module.__name__):
                complete = "".join(f"{marker}\n" for marker in module.MARKERS)
                self.assertEqual(module.validate_raw(complete), [])
                for private in (str(module.ROOT), str(Path.home())):
                    leaked = f"{complete}   Compiling agentmage-kernel-engine v0.0.0 ({private}/kernel/engine)\n"
                    self.assertIn(
                        "raw results name a private checkout or home path",
                        module.validate_raw(leaked),
                    )
                completed = subprocess.CompletedProcess(
                    [], 0, stdout=f"   Compiling agentmage-kernel-engine v0.0.0 ({module.ROOT}/kernel/engine)\n"
                )
                with mock.patch.object(module.subprocess, "run", return_value=completed):
                    raw, returncode = module.capture()
                self.assertEqual(returncode, 0)
                self.assertNotIn(str(module.ROOT), raw)
                self.assertIn("(<repository-root>/kernel/engine)", raw)

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
