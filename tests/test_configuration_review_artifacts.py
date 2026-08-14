from __future__ import annotations

import copy
import tempfile
import unittest
from pathlib import Path

from scripts.configuration_review_artifacts import (
    DELTA_PATH,
    DIFF_FIXTURE_PATH,
    MIGRATION_FIXTURES,
    REPORT_PATH,
    ROLLBACK_FIXTURE_PATH,
    ROOT,
    build_outputs,
    build_report,
    check_artifacts,
    read_json,
    validate_report,
)


class ConfigurationReviewArtifactTests(unittest.TestCase):
    def test_checked_artifacts_and_report_are_current(self) -> None:
        self.assertEqual(check_artifacts(), [])
        self.assertEqual(read_json(REPORT_PATH), build_report())

    def test_migration_fixture_closure_is_exact_and_versioned(self) -> None:
        outputs = build_outputs()
        self.assertEqual(len(MIGRATION_FIXTURES), 5)
        legacy = outputs[MIGRATION_FIXTURES[0]]
        expected = outputs[MIGRATION_FIXTURES[1]]
        self.assertEqual(legacy["schema_version"], 0)
        self.assertEqual(expected["schema_version"], 1)
        self.assertIn("profile", legacy["core"])
        self.assertNotIn("profile_id", legacy["core"])
        for section in (
            "core",
            "platform",
            "model",
            "workspace",
            "tool",
            "permission",
            "budget",
            "logging",
            "retention",
            "skill",
            "shell",
        ):
            self.assertNotIn("schema_version", legacy[section])
        self.assertNotIn("operation", legacy["tool"]["tools"][0])
        self.assertEqual(legacy["tool"]["tools"][0]["side_effect_class"], "read")

    def test_invalid_migration_fixtures_cover_version_reserved_and_missing_cases(self) -> None:
        outputs = build_outputs()
        self.assertEqual(outputs[MIGRATION_FIXTURES[2]]["schema_version"], 1)
        self.assertIn("schema_version", outputs[MIGRATION_FIXTURES[3]]["core"])
        self.assertNotIn("shell", outputs[MIGRATION_FIXTURES[4]])

    def test_profile_deltas_keep_planned_authority_inactive(self) -> None:
        delta = read_json(DELTA_PATH)
        self.assertEqual(len(delta["profiles"]), 7)
        self.assertEqual(delta["effective_authority_broadening_profile_count"], 0)
        self.assertTrue(
            all(
                not item["effective_authority_broadening"]
                and not item["network_effective"]
                and not item["product_registration"]
                for item in delta["profiles"]
            )
        )
        coding = next(item for item in delta["profiles"] if item["profile_id"] == "coding")
        self.assertEqual(
            coding["planned_not_effective"],
            ["process.run", "workspace.write"],
        )

    def test_diff_format_is_sorted_hash_only_and_closed_by_schema_tests(self) -> None:
        diff = read_json(DIFF_FIXTURE_PATH)
        self.assertEqual(
            [item["path"] for item in diff["changes"]],
            sorted(item["path"] for item in diff["changes"]),
        )
        self.assertNotIn("before_value", str(diff))
        self.assertNotIn("after_value", str(diff))

    def test_rollback_format_records_preimage_atomicity_and_privacy(self) -> None:
        rollback = read_json(ROLLBACK_FIXTURE_PATH)
        self.assertTrue(rollback["preimage_verified"])
        self.assertTrue(rollback["backup_retained"])
        self.assertTrue(rollback["atomic_replace"])
        self.assertFalse(rollback["raw_configuration_persisted"])
        self.assertFalse(rollback["private_path_persisted"])

    def test_stale_artifact_and_product_or_macos_overclaim_fail_closed(self) -> None:
        report = build_report()
        stale = copy.deepcopy(report)
        stale["review_artifacts"][0]["sha256"] = "0" * 64
        product = copy.deepcopy(report)
        product["product_profile_activation_claim"] = "pass"
        macos = copy.deepcopy(report)
        macos["macos_execution_status"] = "pass"
        for changed in (stale, product, macos):
            self.assertTrue(validate_report(changed))

        with tempfile.TemporaryDirectory() as directory:
            self.assertTrue(check_artifacts(Path(directory)))


if __name__ == "__main__":
    unittest.main()
