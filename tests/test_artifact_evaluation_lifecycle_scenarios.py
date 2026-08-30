from __future__ import annotations

import copy
import json
import unittest

from scripts import artifact_evaluation_lifecycle_scenarios as fixtures


class ArtifactEvaluationLifecycleScenarioTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.suite = json.loads(fixtures.OUTPUT_PATH.read_text(encoding="utf-8"))
        cls.by_category = {item["category"]: item for item in cls.suite["scenarios"]}

    def test_required_scenario_matrix_is_exact(self) -> None:
        self.assertEqual([item["category"] for item in self.suite["scenarios"]], list(fixtures.CATEGORIES))
        self.assertEqual(self.suite["scenario_count"], 9)

    def test_every_source_identity_resolves_to_a_bound_fixture_manifest(self) -> None:
        index = fixtures.source_index()
        for scenario in self.suite["scenarios"]:
            for source in scenario["source_fixtures"]:
                self.assertEqual(source, index[source["fixture_id"]])

    def test_event_sequences_are_hash_chained_and_monotonic(self) -> None:
        for scenario in self.suite["scenarios"]:
            previous = fixtures.ZERO_SHA256
            for sequence, event in enumerate(scenario["events"], start=1):
                self.assertEqual(event["sequence"], sequence)
                self.assertEqual(event["previous_event_sha256"], previous)
                self.assertTrue(fixtures.valid_hash(event, "event_sha256"))
                previous = event["event_sha256"]

    def test_overflow_and_profile_change_bind_existing_complete_manifests(self) -> None:
        self.assertEqual(self.by_category["combined_artifact_token_overflow"]["context_binding"], fixtures.context_binding("token_budget_overflow"))
        self.assertEqual(self.by_category["model_profile_change"]["context_binding"], fixtures.context_binding("model_profile_change"))
        self.assertFalse(self.by_category["combined_artifact_token_overflow"]["expected"]["completion_allowed"])
        self.assertFalse(self.by_category["model_profile_change"]["expected"]["stale_context_reused"])

    def test_cancellation_crash_and_restart_never_replay_or_false_complete(self) -> None:
        self.assertEqual(self.by_category["cancellation"]["expected"]["residue_count"], 0)
        self.assertFalse(self.by_category["crash"]["expected"]["effect_replayed"])
        self.assertFalse(self.by_category["restart"]["expected"]["effect_replayed"])
        self.assertTrue(self.by_category["restart"]["expected"]["source_revalidated"])

    def test_cache_refresh_retention_and_deletion_preserve_lifecycle_truth(self) -> None:
        self.assertFalse(self.by_category["cache_stale"]["expected"]["cached_derivative_used"])
        self.assertFalse(self.by_category["refresh"]["expected"]["prior_derivative_used"])
        self.assertEqual(self.by_category["retention"]["expected"]["persisted_payload_count"], 0)
        self.assertEqual(self.by_category["deletion"]["expected"]["remaining_reference_count"], 0)
        self.assertFalse(self.by_category["deletion"]["expected"]["resurrection_allowed"])

    def test_mutation_effect_and_runtime_overclaims_fail(self) -> None:
        changed = copy.deepcopy(self.suite)
        changed["scenarios"].pop()
        self.assertTrue(fixtures.validate_suite(changed))
        changed = copy.deepcopy(self.suite)
        changed["scenarios"][0]["prohibited_effects"]["network_calls"] = 1
        self.assertTrue(fixtures.validate_suite(changed))
        changed = copy.deepcopy(self.suite)
        changed["product_runtime_executed"] = True
        self.assertTrue(fixtures.validate_suite(changed))

    def test_checked_suite_matches_the_reproducible_builder(self) -> None:
        self.assertEqual(fixtures.check(), [])


if __name__ == "__main__":
    unittest.main()
