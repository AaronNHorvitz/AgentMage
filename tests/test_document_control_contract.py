from __future__ import annotations

import copy
import unittest

from scripts import document_control_contract as contract


class DocumentControlContractTests(unittest.TestCase):
    def test_closed_corpus_covers_requirements_and_denies_effects(self) -> None:
        value = contract.expected_document()
        self.assertEqual(contract.validate(value), [])
        self.assertEqual(value["case_count"], 68)
        self.assertFalse(value["external_effects_enabled"])
        self.assertFalse(value["records_disposition_enabled"])
        self.assertFalse(value["network_enabled"])
        self.assertFalse(value["product_integration_claimed"])
        self.assertEqual(
            {item["requirement"] for item in value["cases"]},
            {"S-048-I07", "S-048-I08", "S-048-I09", "S-048-UT01", "S-048-UT02", "S-048-ST01", "S-048-IT01"},
        )

    def test_case_and_authority_mutations_fail(self) -> None:
        mutations = (
            lambda value: value["cases"].pop(),
            lambda value: value["cases"].reverse(),
            lambda value: value["cases"][0].update({"expected": "unbounded"}),
            lambda value: value.update({"case_count": 0}),
            lambda value: value.update({"external_effects_enabled": True}),
            lambda value: value.update({"records_disposition_enabled": True}),
            lambda value: value.update({"network_enabled": True}),
            lambda value: value.update({"product_integration_claimed": True}),
        )
        for mutate in mutations:
            changed = copy.deepcopy(contract.expected_document())
            mutate(changed)
            self.assertTrue(contract.validate(changed))


if __name__ == "__main__":
    unittest.main()
