from __future__ import annotations

import copy
import unittest

from scripts.source_lifecycle_transactions_evidence import (
    MARKERS,
    MIGRATION_PATH,
    MODULE_PATH,
    REQUIRED_API_FRAGMENTS,
    REQUIRED_MIGRATION_FRAGMENTS,
    expected_report,
    validate_raw,
    validate_report,
    validate_sources,
)


class SourceLifecycleTransactionsEvidenceTests(unittest.TestCase):
    def test_current_sources_and_report_are_exact(self) -> None:
        self.assertEqual(
            validate_sources(MIGRATION_PATH.read_text(), MODULE_PATH.read_text()),
            [],
        )
        self.assertEqual(validate_report(expected_report()), [])

    def test_missing_migration_or_api_contract_is_rejected(self) -> None:
        migration = MIGRATION_PATH.read_text()
        module = MODULE_PATH.read_text()
        for fragment in REQUIRED_MIGRATION_FRAGMENTS:
            self.assertTrue(validate_sources(migration.replace(fragment, "removed", 1), module), fragment)
        for fragment in REQUIRED_API_FRAGMENTS:
            self.assertTrue(validate_sources(migration, module.replace(fragment, "removed")), fragment)

    def test_second_payload_authority_is_rejected(self) -> None:
        migration = MIGRATION_PATH.read_text()
        self.assertTrue(
            validate_sources(f"{migration}\nCREATE TABLE source_payloads (payload BLOB);", MODULE_PATH.read_text())
        )

    def test_follow_on_and_product_completion_overclaim_is_rejected(self) -> None:
        for field in (
            "typed_source_publication_complete",
            "exhaustive_source_crash_campaign_complete",
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
