"""Mutation tests for the Sprint 61 PDF generation review records."""

from __future__ import annotations

import copy
import unittest

from scripts.pdf_generation_contract import (
    expected_corpus,
    expected_manifest,
    validate,
)


class PdfGenerationContractTests(unittest.TestCase):
    def test_expected_records_are_valid_and_truthfully_blocked(self) -> None:
        corpus = expected_corpus()
        manifest = expected_manifest()
        self.assertEqual(validate(corpus, manifest), [])
        self.assertGreaterEqual(corpus["case_count"], 90)
        self.assertFalse(corpus["release_completion_claimed"])
        self.assertEqual(manifest["executed_native_renderer_platforms"], [])

    def test_corpus_semantic_drift_is_rejected(self) -> None:
        corpus = copy.deepcopy(expected_corpus())
        corpus["cases"][0]["expected"] = "broaden authority"
        self.assertIn(
            "Sprint 61 PDF generation corpus drifted",
            validate(corpus, expected_manifest()),
        )

    def test_dependency_and_platform_overclaims_are_rejected(self) -> None:
        manifest = copy.deepcopy(expected_manifest())
        manifest["development_only_dependencies"][0]["product_runtime_admitted"] = True
        manifest["executed_native_renderer_platforms"] = ["fedora-x86_64"]
        failures = validate(expected_corpus(), manifest)
        self.assertIn("Sprint 61 PDF dependency manifest drifted", failures)

    def test_sensitive_or_false_completion_fields_are_rejected(self) -> None:
        corpus = copy.deepcopy(expected_corpus())
        corpus["secret_value"] = "not-retained"
        manifest = copy.deepcopy(expected_manifest())
        manifest["release_approved"] = True
        failures = validate(corpus, manifest)
        self.assertTrue(any("prohibited review-record field" in item for item in failures))


if __name__ == "__main__":
    unittest.main()
