from __future__ import annotations

import copy
import unittest

from scripts.engineering_context_accounting_fixtures import SCENARIOS, build_suite, validate_suite


class EngineeringContextAccountingFixtureTests(unittest.TestCase):
    def setUp(self) -> None:
        self.suite = build_suite()
        self.scenarios = {item["scenario_id"]: item for item in self.suite["scenarios"]}

    def test_all_required_paths_have_exactly_one_complete_manifest(self) -> None:
        self.assertEqual([item["scenario_id"] for item in self.suite["scenarios"]], list(SCENARIOS))
        for scenario in self.suite["scenarios"]:
            manifest = scenario["context_manifest"]
            self.assertEqual(scenario["complete_manifest_count"], 1)
            self.assertEqual(manifest["source_artifact_count"], 26)
            self.assertEqual(len(manifest["items"]), 26)
            self.assertEqual(len({item["artifact_id"] for item in manifest["items"]}), 26)

    def test_token_and_window_partitions_reconcile_without_silent_overflow(self) -> None:
        for scenario in self.suite["scenarios"]:
            manifest = scenario["context_manifest"]
            accounted = sum(item["token_count"] for item in manifest["items"])
            self.assertEqual(accounted, manifest["total_input_tokens"])
            self.assertEqual(
                accounted + manifest["reserved_output_tokens"] + manifest["safety_margin_tokens"],
                scenario["context_window_tokens"],
            )
        overflow = self.scenarios["token_budget_overflow"]
        self.assertGreater(overflow["requested_input_tokens"], overflow["accounted_input_tokens"])
        self.assertEqual(overflow["expected_target_disposition"], "truncated")

    def test_each_target_exercises_its_named_disposition(self) -> None:
        for scenario in self.suite["scenarios"]:
            target = next(
                item
                for item in scenario["context_manifest"]["items"]
                if item["artifact_id"] == scenario["target_artifact_id"]
            )
            self.assertEqual(target["disposition"], scenario["expected_target_disposition"])
            if target["disposition"] in {"duplicate", "stale", "restricted", "omitted"}:
                self.assertEqual(target["ranges"], [])
                self.assertEqual(target["token_count"], 0)

    def test_model_profile_change_invalidates_instead_of_reusing_prior_context(self) -> None:
        scenario = self.scenarios["model_profile_change"]
        self.assertNotEqual(scenario["prior_model_profile_id"], scenario["context_manifest"]["model_profile_id"])
        self.assertTrue(scenario["model_profile_change_invalidates_prior_manifest"])
        target = next(item for item in scenario["context_manifest"]["items"] if item["artifact_id"] == scenario["target_artifact_id"])
        self.assertEqual(target["reason_code"], "model_profile_changed")

    def test_manifests_are_synthetic_and_do_not_claim_delivery(self) -> None:
        self.assertTrue(self.suite["synthetic_only"])
        self.assertFalse(self.suite["model_request_executed"])
        self.assertEqual(self.suite["product_context_delivery_claim"], "none")
        self.assertTrue(all(item["execution_claim"] == "synthetic_manifest_contract_only" for item in self.suite["scenarios"]))

    def test_mutations_cannot_hide_sources_tokens_paths_or_profile_invalidation(self) -> None:
        mutations = (
            lambda value: value["scenarios"][0]["context_manifest"]["items"].pop(),
            lambda value: value["scenarios"][0]["context_manifest"].update({"total_input_tokens": 0}),
            lambda value: value["scenarios"][1].update({"expected_target_disposition": "omitted"}),
            lambda value: value["scenarios"][7].update({"model_profile_change_invalidates_prior_manifest": False}),
            lambda value: value.update({"product_context_delivery_claim": "delivered"}),
        )
        self.assertEqual(validate_suite(self.suite), [])
        for mutate in mutations:
            changed = copy.deepcopy(self.suite)
            mutate(changed)
            self.assertTrue(validate_suite(changed))


if __name__ == "__main__":
    unittest.main()
