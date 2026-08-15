"""Mutation tests for the Sprint 63 reconciliation records."""

from __future__ import annotations

import copy
import unittest

from scripts.reconciliation_contract import expected_corpus, expected_manifest, validate


class ReconciliationContractTests(unittest.TestCase):
    def test_expected_records_are_valid_and_truthfully_blocked(self) -> None:
        corpus, manifest = expected_corpus(), expected_manifest()
        self.assertEqual(validate(corpus, manifest), [])
        self.assertGreaterEqual(corpus["case_count"], 100)
        self.assertFalse(corpus["release_completion_claimed"])
        self.assertEqual(manifest["executed_native_office_platforms"], [])

    def test_corpus_semantic_drift_is_rejected(self) -> None:
        corpus = copy.deepcopy(expected_corpus()); corpus["cases"][0]["expected"] = "broaden authority"
        self.assertIn("Sprint 63 reconciliation corpus drifted", validate(corpus, expected_manifest()))

    def test_dependency_and_platform_overclaims_are_rejected(self) -> None:
        manifest = copy.deepcopy(expected_manifest()); manifest["executed_native_office_platforms"] = ["fedora-x86_64"]
        self.assertIn("Sprint 63 dependency manifest drifted", validate(expected_corpus(), manifest))

    def test_sensitive_or_false_completion_fields_are_rejected(self) -> None:
        corpus, manifest = copy.deepcopy(expected_corpus()), copy.deepcopy(expected_manifest())
        corpus["secret_value"] = "not-retained"; manifest["release_approved"] = True
        self.assertTrue(any("prohibited review-record field" in item for item in validate(corpus, manifest)))


if __name__ == "__main__": unittest.main()
