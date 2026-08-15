from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import coding_skill_contract as contract


class CodingSkillContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.canonical = contract.build()

    def test_canonical_pack_is_complete_and_authority_free(self) -> None:
        self.assertEqual(contract.validate(self.canonical), [])
        self.assertEqual(len(self.canonical["definitions"]), 9)
        self.assertTrue(all(
            item["admission"] == "admitted"
            for item in self.canonical["assessments"]
        ))

    def test_every_authority_overclaim_fails(self) -> None:
        for field in (
            "product_registration",
            "authority_enabled",
            "network_access",
            "automatic_publication",
        ):
            changed = copy.deepcopy(self.canonical)
            changed[field] = True
            self.assertTrue(contract.validate(changed), field)

    def test_manifest_assessment_authority_and_exclusion_drift_fail(self) -> None:
        mutations = (
            lambda value: value["definitions"].pop(),
            lambda value: value["definitions"][0]["authority"].update({"writes": True}),
            lambda value: value["definitions"][0]["prohibited_operations"].pop(),
            lambda value: value["assessments"][0].update({"admission": "disabled"}),
            lambda value: value["assessments"][0].update({"authority_granted": True}),
            lambda value: value["manifests"][0].update({"trust_state": "quarantined"}),
        )
        for mutate in mutations:
            changed = copy.deepcopy(self.canonical)
            mutate(changed)
            self.assertTrue(contract.validate(changed))


if __name__ == "__main__":
    unittest.main()
