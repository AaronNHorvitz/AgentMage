from __future__ import annotations

import copy
import json
import unittest

from scripts import artifact_evaluation_plan_fixtures as fixtures


class ArtifactEvaluationPlanFixtureTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.suite = json.loads(fixtures.OUTPUT_PATH.read_text(encoding="utf-8"))
        cls.by_category = {item["category"]: item for item in cls.suite["fixtures"]}

    def test_required_plan_fixture_matrix_is_exact(self) -> None:
        self.assertEqual([item["category"] for item in self.suite["fixtures"]], list(fixtures.CATEGORIES))
        self.assertEqual(self.suite["fixture_count"], 11)

    def test_success_and_repair_require_deterministic_verification(self) -> None:
        self.assertEqual(self.by_category["success"]["expected"]["completion_owner"], "deterministic_verifier")
        repair = self.by_category["deterministic_repair"]
        self.assertEqual([item["state"] for item in repair["proposal_revisions"]], ["schema_invalid", "schema_valid"])
        self.assertEqual(repair["expected"]["repair_count"], 1)

    def test_invalid_graph_preflight_and_call_block_before_dispatch(self) -> None:
        for category in ("missing_dependency", "stale_preflight", "malformed_call"):
            self.assertEqual(self.by_category[category]["expected"]["dispatch_count"], 0)

    def test_transient_read_uses_fresh_attempts_only(self) -> None:
        result = self.by_category["transient_read_failure"]["expected"]
        self.assertEqual(result["attempt_count"], 2)
        self.assertTrue(result["fresh_attempt_ids_required"])
        self.assertTrue(result["automatic_retry_permitted"])

    def test_effect_classes_never_gain_unsafe_automatic_retry(self) -> None:
        for category in ("conditional_conflict", "non_idempotent_effect", "uncertain_effect", "destructive_request", "external_effect"):
            self.assertFalse(self.by_category[category]["expected"]["automatic_retry_permitted"])
        self.assertTrue(self.by_category["uncertain_effect"]["expected"]["reconciliation_required"])

    def test_every_fixture_is_inert_and_self_hashed(self) -> None:
        for item in self.suite["fixtures"]:
            self.assertFalse(item["authority_minted"])
            self.assertFalse(item["effect_executed"])
            self.assertTrue(fixtures.valid_hash(item, "fixture_sha256"))

    def test_mutation_authority_and_runtime_overclaims_fail(self) -> None:
        changed = copy.deepcopy(self.suite)
        changed["fixtures"].pop()
        self.assertTrue(fixtures.validate_suite(changed))
        changed = copy.deepcopy(self.suite)
        changed["fixtures"][0]["authority_minted"] = True
        self.assertTrue(fixtures.validate_suite(changed))
        changed = copy.deepcopy(self.suite)
        changed["product_runtime_executed"] = True
        self.assertTrue(fixtures.validate_suite(changed))

    def test_checked_suite_matches_the_reproducible_builder(self) -> None:
        self.assertEqual(fixtures.check(), [])


if __name__ == "__main__":
    unittest.main()
