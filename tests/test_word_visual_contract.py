from __future__ import annotations

import copy
import unittest

from scripts import word_visual_contract as contract


class WordVisualContractTests(unittest.TestCase):
    def test_closed_records_cover_all_sprint_59_requirement_families(self) -> None:
        corpus = contract.expected_corpus()
        manifest = contract.expected_manifest()
        self.assertEqual(contract.validate(corpus, manifest), [])
        self.assertEqual(corpus["case_count"], 117)
        self.assertEqual(
            {item["requirement"] for item in corpus["cases"]},
            {
                "S-050-I05",
                "S-050-I06",
                "S-050-I07",
                "S-050-I08",
                "S-050-ST01",
                "S-050-UT01/S-050-UT02",
                "S-050-IT01",
            },
        )
        self.assertEqual(corpus["native_render_fixture_count"], 0)
        self.assertFalse(manifest["cross_platform_render_acceptance_complete"])
        self.assertEqual(manifest["executed_native_renderer_platforms"], [])
        self.assertFalse(manifest["visual_comparator"]["renderer_execution_authority"])

    def test_corpus_renderer_and_authority_mutations_fail(self) -> None:
        mutations = (
            lambda corpus, _: corpus["cases"].pop(),
            lambda corpus, _: corpus["cases"].reverse(),
            lambda corpus, _: corpus.update({"case_count": 0}),
            lambda corpus, _: corpus.update({"native_render_fixture_count": 1}),
            lambda corpus, _: corpus.update({"network_enabled": True}),
            lambda corpus, _: corpus.update({"execution_enabled": True}),
            lambda corpus, _: corpus.update({"filesystem_mutation_enabled": True}),
            lambda _, manifest: manifest["admitted_ooxml_dependencies"].pop(),
            lambda _, manifest: manifest["visual_comparator"].update(
                {"renderer_execution_authority": True}
            ),
            lambda _, manifest: manifest["executed_native_renderer_platforms"].append(
                "fedora-x86_64"
            ),
            lambda _, manifest: manifest.update(
                {"cross_platform_render_acceptance_complete": True}
            ),
            lambda _, manifest: manifest["unadmitted_components"].clear(),
        )
        for mutate in mutations:
            corpus = copy.deepcopy(contract.expected_corpus())
            manifest = copy.deepcopy(contract.expected_manifest())
            mutate(corpus, manifest)
            self.assertTrue(contract.validate(corpus, manifest))


if __name__ == "__main__":
    unittest.main()
