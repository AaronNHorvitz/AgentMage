from __future__ import annotations

import copy
import unittest

from scripts import model_routing_contract as contract


class ModelRoutingContractTests(unittest.TestCase):
    def setUp(self) -> None:
        self.historical = contract.load(contract.HISTORICAL)
        self.corpus = contract.load(contract.CORPUS)
        self.table = contract.load(contract.TABLE)
        self.inventory = contract.load(contract.INVENTORY)

    def validate(self) -> None:
        contract.validate_all(
            self.historical,
            self.corpus,
            self.table,
            self.inventory,
        )

    def test_current_routing_artifacts_are_closed_and_fail_closed(self) -> None:
        self.validate()

    def test_historical_candidates_cannot_gain_profile_or_routing_authority(self) -> None:
        mutations = (
            lambda value: value.update({"enabled_profile_count": 1}),
            lambda value: value.update({"automatic_routing_enabled": True}),
            lambda value: value["candidates"][0].update({"selectable": True}),
            lambda value: value["candidates"][0].update(
                {"exact_artifact_sha256": "a" * 64}
            ),
            lambda value: value["candidates"][1].update(
                {"origin_disposition": "official_first_party_namespace"}
            ),
            lambda value: value["candidates"].pop(),
        )
        for mutate in mutations:
            changed = copy.deepcopy(self.historical)
            mutate(changed)
            with self.assertRaises(ValueError):
                contract.validate_historical(changed, self.inventory)

    def test_role_corpus_rejects_threshold_and_role_coverage_drift(self) -> None:
        mutations = (
            lambda value: value.update({"minimum_repeated_trials": 1}),
            lambda value: value.update({"model_confidence_is_routing_evidence": True}),
            lambda value: value["roles"].pop(),
            lambda value: value["roles"][0]["suite_ids"].remove("failure"),
            lambda value: value["required_exact_tuple_fields"].pop(),
        )
        for mutate in mutations:
            changed = copy.deepcopy(self.corpus)
            mutate(changed)
            with self.assertRaises(ValueError):
                contract.validate_corpus(changed)

    def test_decision_table_rejects_hidden_switches_and_remote_paths(self) -> None:
        mutations = (
            lambda value: value.update({"frontier_transfer": True}),
            lambda value: value.update({"invisible_fallback": True}),
            lambda value: value.update({"provider_marketplace": True}),
            lambda value: value.update({"automatic_install": True}),
            lambda value: value.update({"model_self_selection": True}),
            lambda value: value["rules"][0].update({"fallback": True}),
            lambda value: value["rules"][0].update({"visible": False}),
            lambda value: value["visible_budgets"][0].update({"local_only": False}),
        )
        for mutate in mutations:
            changed = copy.deepcopy(self.table)
            mutate(changed)
            with self.assertRaises(ValueError):
                contract.validate_table(changed)


if __name__ == "__main__":
    unittest.main()
