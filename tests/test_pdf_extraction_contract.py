from __future__ import annotations

import copy
import unittest

from scripts import pdf_extraction_contract as contract


class PdfExtractionContractTests(unittest.TestCase):
    def test_closed_records_cover_every_sprint_60_requirement_family(self) -> None:
        corpus = contract.expected_corpus()
        manifest = contract.expected_manifest()
        self.assertEqual(contract.validate(corpus, manifest), [])
        self.assertEqual(corpus["case_count"], 82)
        self.assertEqual(
            {item["requirement"] for item in corpus["cases"]},
            {"S-051-I01", "S-051-I02", "S-051-I03", "S-051-I04", "60.1.2/60.1.3", "60.1"},
        )
        self.assertEqual(corpus["executable_rust_fixture_count"], 6)
        self.assertEqual(corpus["native_ocr_fixture_count"], 0)
        self.assertFalse(manifest["cross_platform_extraction_acceptance_complete"])
        self.assertEqual(manifest["executed_native_renderer_platforms"], [])

    def test_dependency_authority_and_completion_mutations_fail(self) -> None:
        mutations = (
            lambda corpus, _: corpus["cases"].pop(),
            lambda corpus, _: corpus.update({"case_count": 0}),
            lambda corpus, _: corpus.update({"native_ocr_fixture_count": 1}),
            lambda corpus, _: corpus.update({"network_enabled": True}),
            lambda corpus, _: corpus.update({"execution_enabled": True}),
            lambda corpus, _: corpus.update({"filesystem_mutation_enabled": True}),
            lambda _, manifest: manifest["admitted_dependencies"][0].update({"version": "latest"}),
            lambda _, manifest: manifest["admitted_dependencies"][0].update({"default_features": True}),
            lambda _, manifest: manifest["unadmitted_components"].pop(),
            lambda _, manifest: manifest["executed_native_ocr_platforms"].append("fedora-x86_64"),
            lambda _, manifest: manifest.update({"cross_platform_extraction_acceptance_complete": True}),
        )
        for mutate in mutations:
            corpus = copy.deepcopy(contract.expected_corpus())
            manifest = copy.deepcopy(contract.expected_manifest())
            mutate(corpus, manifest)
            self.assertTrue(contract.validate(corpus, manifest))


if __name__ == "__main__":
    unittest.main()
