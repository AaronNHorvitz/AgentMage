"""Mutation tests for the Story 50.2 component-removal campaign contract."""

from __future__ import annotations

import copy
import json
import unittest
from pathlib import Path

from scripts import runtime_component_removal as campaign

ROOT = Path(__file__).resolve().parents[1]


class RuntimeComponentRemovalTests(unittest.TestCase):
    def setUp(self) -> None:
        self.profile = json.loads((ROOT / campaign.PROFILE).read_text(encoding="utf-8"))

    def test_current_profile_is_closed(self) -> None:
        campaign.validate_profile(self.profile)

    def test_test_summary_aggregates_and_rejects_failure(self) -> None:
        output = (
            "test result: ok. 110 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out\n"
            "test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out\n"
        )
        self.assertEqual(
            campaign.parse_test_results(output),
            {
                "passed": 112,
                "failed": 0,
                "ignored": 3,
                "measured": 0,
                "filtered_out": 4,
            },
        )
        with self.assertRaises(campaign.CampaignError):
            campaign.parse_test_results("test result: FAILED")

    def test_every_source_boundary_detects_presence_and_absence(self) -> None:
        for scenario in self.profile["scenarios"]:
            with self.subTest(scenario=scenario["id"]):
                valid = [scenario["crate_root"], *scenario["required_sources"]]
                self.assertEqual(campaign.source_manifest_failures(valid, scenario), [])

                removed = [*valid, scenario["removed_sources"][0]]
                self.assertIn(
                    "removed source compiled",
                    campaign.source_manifest_failures(removed, scenario)[0],
                )

                missing = [scenario["crate_root"], *scenario["required_sources"][1:]]
                self.assertTrue(campaign.source_manifest_failures(missing, scenario))

    def test_command_feature_and_source_mutations_fail_closed(self) -> None:
        mutations = []

        command = copy.deepcopy(self.profile)
        command["scenarios"][0]["test_argv"][0] = "sh"
        mutations.append(command)

        feature = copy.deepcopy(self.profile)
        feature["scenarios"][1]["features"].append("native-chat")
        mutations.append(feature)

        removed = copy.deepcopy(self.profile)
        removed["scenarios"][2]["removed_sources"] = []
        mutations.append(removed)

        registry = copy.deepcopy(self.profile)
        registry["source_paths"].remove("shells/host/src/workflow_caller.rs")
        mutations.append(registry)

        limitations = copy.deepcopy(self.profile)
        limitations["declared_limitations"].pop()
        mutations.append(limitations)

        for index, value in enumerate(mutations):
            with self.subTest(index=index):
                with self.assertRaises(campaign.CampaignError):
                    campaign.validate_profile(value)


if __name__ == "__main__":
    unittest.main()
