from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import frontier_recommendation_contract as contract


class FrontierRecommendationContractTests(unittest.TestCase):
    def test_canonical_corpus_is_complete_and_no_delivery(self) -> None:
        value = contract.read()
        self.assertEqual(contract.validate(value), [])
        self.assertFalse(value["external_delivery_attempted"])
        self.assertEqual(value["expanded_case_counts"]["total"], 45)

    def test_unmeasured_or_unapproved_frontier_recommendation_fails(self) -> None:
        changed = copy.deepcopy(contract.read())
        candidate = next(
            case
            for case in changed["tier_cases"]
            if case["expected_tier"] == "frontier_recommended"
        )
        candidate["local_model_attempted"] = False
        self.assertTrue(contract.validate(changed))
        changed = copy.deepcopy(contract.read())
        changed["tier_cases"][0]["expected_trigger"] = "ambient_model_confidence"
        self.assertTrue(contract.validate(changed))

    def test_delivery_disclosure_count_and_source_drift_fail(self) -> None:
        mutations = (
            lambda value: value["prohibited_delivery_attempts"].pop(),
            lambda value: value["disclosure_cases"].pop(),
            lambda value: value["expanded_case_counts"].update({"total": 44}),
            lambda value: value.update({"external_delivery_attempted": True}),
        )
        for mutate in mutations:
            changed = copy.deepcopy(contract.read())
            mutate(changed)
            self.assertTrue(contract.validate(changed))
        with patch.object(contract, "RUST_HANDOFF", contract.CORPUS_PATH):
            self.assertTrue(contract.validate(contract.read()))


if __name__ == "__main__":
    unittest.main()
