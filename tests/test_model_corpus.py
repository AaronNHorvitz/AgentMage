from __future__ import annotations

import copy
import hashlib
import unittest

from scripts.model_corpus import (
    EXPECTED_THRESHOLDS,
    REQUIRED_ADAPTERS,
    REQUIRED_CATEGORIES,
    generate_context_fixture,
    load_corpus,
    validate_corpus,
)


class ModelCorpusTests(unittest.TestCase):
    def setUp(self) -> None:
        self.corpus = load_corpus()

    def test_committed_corpus_is_complete_fixed_and_synthetic(self) -> None:
        self.assertEqual(validate_corpus(self.corpus), [])
        self.assertEqual(
            {item["id"] for item in self.corpus["adapters"]},
            REQUIRED_ADAPTERS,
        )
        self.assertEqual(
            {item["category"] for item in self.corpus["cases"]},
            REQUIRED_CATEGORIES,
        )
        self.assertEqual(self.corpus["global_thresholds"], EXPECTED_THRESHOLDS)
        self.assertFalse(self.corpus["contains_user_data"])

    def test_generated_context_fixture_is_byte_deterministic(self) -> None:
        fixture = self.corpus["fixture_generation"]
        first = generate_context_fixture(fixture["seed"], fixture["line_count"])
        second = generate_context_fixture(fixture["seed"], fixture["line_count"])

        self.assertEqual(first, second)
        self.assertEqual(len(first), fixture["byte_count"])
        self.assertEqual(hashlib.sha256(first).hexdigest(), fixture["sha256"])
        self.assertTrue(first.startswith(b"FACT-0000:"))
        self.assertIn(b"FACT-2047:", first)

    def test_missing_adapter_category_or_duplicate_case_is_rejected(self) -> None:
        missing_adapter = copy.deepcopy(self.corpus)
        missing_adapter["adapters"].pop()
        self.assertTrue(
            any("adapter matrix" in item for item in validate_corpus(missing_adapter))
        )

        missing_category = copy.deepcopy(self.corpus)
        missing_category["cases"] = [
            item for item in missing_category["cases"] if item["category"] != "zero_egress"
        ]
        self.assertTrue(
            any("category coverage" in item for item in validate_corpus(missing_category))
        )

        duplicate = copy.deepcopy(self.corpus)
        duplicate["cases"].append(copy.deepcopy(duplicate["cases"][0]))
        self.assertTrue(any("duplicate corpus" in item for item in validate_corpus(duplicate)))

    def test_decoder_threshold_fixture_and_data_scope_drift_are_rejected(self) -> None:
        mutations = {
            "decoder setting": lambda value: value["decoder"].update({"temperature": 0.7}),
            "thresholds changed": lambda value: value["global_thresholds"].update(
                {"citation_precision": 0.8}
            ),
            "fixture hash changed": lambda value: value["fixture_generation"].update(
                {"sha256": "0" * 64}
            ),
            "public synthetic": lambda value: value.update({"contains_user_data": True}),
        }
        for expected, mutate in mutations.items():
            with self.subTest(mutation=expected):
                changed = copy.deepcopy(self.corpus)
                mutate(changed)
                failures = validate_corpus(changed)
                self.assertTrue(any(expected in item for item in failures), failures)


if __name__ == "__main__":
    unittest.main()
