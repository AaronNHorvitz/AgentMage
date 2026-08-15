from __future__ import annotations

import copy
import unittest

from scripts import markdown_artifact_contract as contract


class MarkdownArtifactContractTests(unittest.TestCase):
    def test_closed_corpus_covers_requirements_and_denies_effects(self) -> None:
        value = contract.expected_document()
        self.assertEqual(contract.validate(value), [])
        self.assertEqual(value["case_count"], 68)
        self.assertFalse(value["network_enabled"])
        self.assertFalse(value["execution_enabled"])
        self.assertFalse(value["filesystem_mutation_enabled"])
        self.assertFalse(value["citation_invention_enabled"])
        self.assertFalse(value["product_integration_claimed"])
        self.assertEqual(
            {item["requirement"] for item in value["cases"]},
            {
                "S-049-I02/S-049-I03",
                "S-049-I04",
                "S-049-I06",
                "S-049-I07/S-049-IT01",
                "S-049-UT01",
                "S-049-UT02",
                "S-049-ST01",
                "S-049-IT01",
            },
        )

    def test_case_and_authority_mutations_fail(self) -> None:
        mutations = (
            lambda value: value["cases"].pop(),
            lambda value: value["cases"].reverse(),
            lambda value: value["cases"][0].update({"expected": "unbounded"}),
            lambda value: value.update({"case_count": 0}),
            lambda value: value.update({"network_enabled": True}),
            lambda value: value.update({"execution_enabled": True}),
            lambda value: value.update({"filesystem_mutation_enabled": True}),
            lambda value: value.update({"citation_invention_enabled": True}),
            lambda value: value.update({"product_integration_claimed": True}),
        )
        for mutate in mutations:
            changed = copy.deepcopy(contract.expected_document())
            mutate(changed)
            self.assertTrue(contract.validate(changed))


if __name__ == "__main__":
    unittest.main()
