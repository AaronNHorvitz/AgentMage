from __future__ import annotations

import copy
import unittest

from scripts import word_artifact_contract as contract


class WordArtifactContractTests(unittest.TestCase):
    def test_closed_records_cover_requirements_and_preserve_blockers(self) -> None:
        corpus = contract.expected_corpus()
        dependencies = contract.expected_dependencies()
        self.assertEqual(contract.validate(corpus, dependencies), [])
        self.assertEqual(corpus["case_count"], 73)
        self.assertEqual(
            {item["requirement"] for item in corpus["cases"]},
            {"S-050-I01", "S-050-I02", "S-050-I03", "S-050-I04", "S-050-I02/S-050-I04", "S-050-ST01"},
        )
        self.assertFalse(corpus["network_enabled"])
        self.assertFalse(corpus["execution_enabled"])
        self.assertFalse(corpus["filesystem_mutation_enabled"])
        self.assertFalse(corpus["renderer_admitted"])
        self.assertFalse(dependencies["cross_platform_acceptance_complete"])
        self.assertEqual(dependencies["unadmitted_components"][0]["capability"], "word-renderer")

    def test_corpus_and_dependency_authority_mutations_fail(self) -> None:
        mutations = (
            lambda corpus, _: corpus["cases"].pop(),
            lambda corpus, _: corpus["cases"].reverse(),
            lambda corpus, _: corpus.update({"case_count": 0}),
            lambda corpus, _: corpus.update({"network_enabled": True}),
            lambda corpus, _: corpus.update({"execution_enabled": True}),
            lambda corpus, _: corpus.update({"filesystem_mutation_enabled": True}),
            lambda corpus, _: corpus.update({"renderer_admitted": True}),
            lambda _, dependencies: dependencies["admitted_components"].pop(),
            lambda _, dependencies: dependencies["admitted_components"][0].update({"version": "unbounded"}),
            lambda _, dependencies: dependencies["isolation"].update({"ambient_process_execution": True}),
            lambda _, dependencies: dependencies.update({"cross_platform_acceptance_complete": True}),
            lambda _, dependencies: dependencies["unadmitted_components"].clear(),
        )
        for mutate in mutations:
            corpus = copy.deepcopy(contract.expected_corpus())
            dependencies = copy.deepcopy(contract.expected_dependencies())
            mutate(corpus, dependencies)
            self.assertTrue(contract.validate(corpus, dependencies))


if __name__ == "__main__":
    unittest.main()
