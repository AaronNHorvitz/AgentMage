from __future__ import annotations

import copy
import unittest

from scripts import markdown_artifact_skill_contract as contract


class MarkdownArtifactSkillContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.value = contract.build()

    def test_current_pack_is_closed_and_authority_free(self) -> None:
        self.assertEqual(contract.validate(copy.deepcopy(self.value)), [])
        self.assertTrue(self.value["authority"])
        self.assertFalse(any(self.value["authority"].values()))
        for field in contract.ZERO_EFFECT_FIELDS:
            self.assertFalse(self.value[field])

    def test_inventory_authority_and_effect_mutations_fail(self) -> None:
        mutations = (
            lambda value: value["skills"].pop(),
            lambda value: value["manifests"].pop(),
            lambda value: value["authority"].update({"file": True}),
            lambda value: value.update({"filesystem_access": True}),
            lambda value: value.update({"network_access": True}),
            lambda value: value.update({"execution_access": True}),
            lambda value: value.update({"citation_invention": True}),
            lambda value: value.update({"acronym_expansion": True}),
            lambda value: value["manifests"][0].update({"version": "unbounded"}),
        )
        for mutate in mutations:
            changed = copy.deepcopy(self.value)
            mutate(changed)
            self.assertTrue(contract.validate(changed))


if __name__ == "__main__":
    unittest.main()
