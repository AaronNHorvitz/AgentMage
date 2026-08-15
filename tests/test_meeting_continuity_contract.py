from __future__ import annotations

import copy
import unittest

from scripts import meeting_continuity_contract as contract


class MeetingContinuityContractTests(unittest.TestCase):
    def test_closed_corpus_covers_every_behavior_and_denial(self) -> None:
        value = contract.expected_document()
        self.assertEqual(contract.validate(value), [])
        self.assertEqual(value["case_count"], 80)
        self.assertFalse(value["authority_effects_enabled"])
        self.assertFalse(value["network_enabled"])
        self.assertFalse(value["source_mutation_enabled"])
        self.assertFalse(value["product_integration_claimed"])
        self.assertEqual(
            {item["requirement"] for item in value["cases"]},
            {
                "S-048-I01",
                "S-048-I02",
                "S-048-I03",
                "S-048-I04",
                "S-048-I05",
                "S-048-I06",
                "S-048-ST01",
            },
        )

    def test_case_and_effect_mutations_fail(self) -> None:
        mutations = (
            lambda value: value["cases"].pop(),
            lambda value: value["cases"].reverse(),
            lambda value: value["cases"][0].update({"expected": "unbounded"}),
            lambda value: value.update({"case_count": 0}),
            lambda value: value.update({"authority_effects_enabled": True}),
            lambda value: value.update({"network_enabled": True}),
            lambda value: value.update({"source_mutation_enabled": True}),
            lambda value: value.update({"product_integration_claimed": True}),
        )
        for mutate in mutations:
            changed = copy.deepcopy(contract.expected_document())
            mutate(changed)
            self.assertTrue(contract.validate(changed))


if __name__ == "__main__":
    unittest.main()
