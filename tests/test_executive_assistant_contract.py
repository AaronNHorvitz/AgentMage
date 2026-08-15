from __future__ import annotations

import copy
import unittest

from scripts import executive_assistant_contract as contract


class ExecutiveAssistantContractTests(unittest.TestCase):
    def test_exact_seventy_seven_case_corpus_is_valid(self) -> None:
        value = contract.expected_corpus()
        self.assertEqual(contract.validate(value), [])
        self.assertEqual(value["expanded_case_counts"]["total"], 77)

    def test_every_inventory_and_case_count_mutation_fails(self) -> None:
        for field in contract.EXPECTED:
            changed = copy.deepcopy(contract.expected_corpus())
            changed[field].pop()
            self.assertTrue(contract.validate(changed), field)
        changed = copy.deepcopy(contract.expected_corpus())
        changed["expanded_case_counts"]["total"] = 76
        self.assertTrue(contract.validate(changed))

    def test_every_effect_and_gate_overclaim_fails(self) -> None:
        for field in contract.ZERO_EFFECT_FIELDS:
            changed = copy.deepcopy(contract.expected_corpus())
            changed[field] = True
            self.assertTrue(contract.validate(changed), field)


if __name__ == "__main__":
    unittest.main()
