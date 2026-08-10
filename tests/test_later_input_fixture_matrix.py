from __future__ import annotations

import copy
import unittest

from scripts.later_input_fixture_matrix import (
    FIXTURE_SET_PATH,
    INPUT_CLASSES,
    REPORT_PATH,
    SCENARIOS,
    build_fixture_set,
    build_report,
    check_artifacts,
    read_json,
    validate_fixture_set,
    validate_report,
)


class LaterInputFixtureMatrixTests(unittest.TestCase):
    def test_checked_fixture_set_and_report_are_current(self) -> None:
        self.assertEqual(check_artifacts(), [])
        self.assertEqual(read_json(FIXTURE_SET_PATH), build_fixture_set())
        self.assertEqual(read_json(REPORT_PATH), build_report())

    def test_every_input_class_has_every_required_scenario(self) -> None:
        fixture_set = build_fixture_set()
        observed = {
            (item["input_class"], item["scenario"])
            for item in fixture_set["fixtures"]
        }
        expected = {
            (input_class, scenario)
            for input_class in INPUT_CLASSES
            for scenario in SCENARIOS
        }
        self.assertEqual(observed, expected)
        self.assertEqual(len(observed), 84)

    def test_every_fixture_is_bounded_synthetic_and_side_effect_free(self) -> None:
        for fixture in build_fixture_set()["fixtures"]:
            self.assertEqual(fixture["source_data_class"], "synthetic-public")
            self.assertLessEqual(fixture["payload_bytes"], 1024)
            self.assertTrue(
                all(value == 0 for value in fixture["prohibited_side_effects"].values())
            )

    def test_missing_scenario_or_input_class_fails_closed(self) -> None:
        missing_scenario = build_fixture_set()
        missing_scenario["fixtures"].pop()
        missing_input = build_fixture_set()
        missing_input["input_classes"].pop()
        self.assertTrue(validate_fixture_set(missing_scenario))
        self.assertTrue(validate_fixture_set(missing_input))

    def test_hash_and_side_effect_mutations_fail_closed(self) -> None:
        fixture_set = build_fixture_set()
        mutated_hash = copy.deepcopy(fixture_set)
        mutated_hash["fixtures"][0]["fixture_sha256"] = "0" * 64
        side_effect = copy.deepcopy(fixture_set)
        side_effect["fixtures"][0]["prohibited_side_effects"][
            "network_call_count"
        ] = 1
        self.assertTrue(validate_fixture_set(mutated_hash))
        self.assertTrue(validate_fixture_set(side_effect))

    def test_private_product_or_macos_claims_fail_closed(self) -> None:
        fixture_set = build_fixture_set()
        private = copy.deepcopy(fixture_set)
        private["content_contract"]["private_user_data"] = True
        claimed = copy.deepcopy(fixture_set)
        claimed["product_support_claim"] = "supported"
        unblocked = copy.deepcopy(fixture_set)
        unblocked["macos_execution_status"] = "pass"
        self.assertTrue(validate_fixture_set(private))
        self.assertTrue(validate_fixture_set(claimed))
        self.assertTrue(validate_fixture_set(unblocked))

    def test_report_rejects_weakened_coverage_or_safety(self) -> None:
        report = build_report()
        coverage = copy.deepcopy(report)
        coverage["coverage"]["fixture_count"] = 83
        unsafe = copy.deepcopy(report)
        unsafe["all_prohibited_side_effect_counts"] = 1
        self.assertTrue(validate_report(coverage))
        self.assertTrue(validate_report(unsafe))


if __name__ == "__main__":
    unittest.main()
