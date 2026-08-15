from __future__ import annotations

import copy
import unittest

from scripts import frontier_import_contract as contract


class FrontierImportContractTests(unittest.TestCase):
    def test_canonical_corpus_is_complete_and_zero_effect(self) -> None:
        self.assertEqual(contract.validate(contract.read()), [])

    def test_identity_count_and_authority_drift_fail(self) -> None:
        mutations = (
            lambda value: value.update({"corpus_id": "changed"}),
            lambda value: value["manifest_failures"].pop(),
            lambda value: value["artifact_attacks"].pop(),
            lambda value: value["state_change_cases"].pop(),
            lambda value: value["local_flow_cases"].pop(),
            lambda value: value["round_trip_mutations"].reverse(),
            lambda value: value.update({"authority_granted": True}),
            lambda value: value.update({"effect_count": 1}),
            lambda value: value.update({"outbound_network_used": True}),
        )
        for mutate in mutations:
            changed = copy.deepcopy(contract.read())
            mutate(changed)
            self.assertTrue(contract.validate(changed))


if __name__ == "__main__":
    unittest.main()
