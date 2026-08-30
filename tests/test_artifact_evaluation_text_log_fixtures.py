from __future__ import annotations

import copy
import json
import unittest
import zipfile

from scripts import artifact_evaluation_text_log_fixtures as fixtures


class ArtifactEvaluationTextLogFixtureTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.manifest = json.loads(fixtures.MANIFEST_PATH.read_text(encoding="utf-8"))

    def test_required_categories_are_exact_and_complete(self) -> None:
        self.assertEqual([item["category"] for item in self.manifest["cases"]], list(fixtures.CATEGORIES))
        self.assertEqual(self.manifest["case_count"], 16)

    def test_paste_boundaries_are_exact_unicode_character_counts(self) -> None:
        by_category = {item["category"]: item for item in self.manifest["cases"]}
        self.assertEqual(by_category["paste_999"]["character_count"], 999)
        self.assertEqual(by_category["paste_999"]["byte_identity"]["byte_length"], 999)
        self.assertEqual(by_category["paste_1001"]["character_count"], 1001)
        self.assertEqual(by_category["paste_1001"]["byte_identity"]["byte_length"], 1001)

    def test_encoding_shapes_are_real_and_distinct(self) -> None:
        entries = fixtures.payloads()
        entries["payloads/utf8.txt"].decode("utf-8")
        self.assertTrue(entries["payloads/utf16.txt"].startswith((b"\xff\xfe", b"\xfe\xff")))
        with self.assertRaises(UnicodeDecodeError):
            entries["payloads/invalid.bin"].decode("utf-8")
        with self.assertRaises(UnicodeDecodeError):
            entries["payloads/mixed.bin"].decode("utf-8")

    def test_duplicate_stale_replaced_and_unavailable_truth_is_explicit(self) -> None:
        by_category = {item["category"]: item for item in self.manifest["cases"]}
        self.assertEqual(by_category["duplicate_original"]["byte_identity"], by_category["duplicate_reference"]["byte_identity"])
        self.assertNotEqual(by_category["stale"]["byte_identity"], by_category["stale"]["current_byte_identity"])
        self.assertNotEqual(by_category["replaced"]["byte_identity"], by_category["replaced"]["current_byte_identity"])
        for category in ("inaccessible_reference", "unknown_reference"):
            self.assertIsNone(by_category[category]["byte_identity"])
            self.assertIsNone(by_category[category]["archive_entry"])

    def test_bounded_log_regenerates_exactly_from_the_pinned_seed(self) -> None:
        content = fixtures.bounded_log()
        recipe = self.manifest["bounded_log_recipe"]
        self.assertEqual(len(content), 25 * 1024 * 1024)
        self.assertEqual(fixtures.sha256_bytes(content), recipe["generated_sha256"])
        self.assertEqual(recipe["seed"], fixtures.LOG_SEED)
        self.assertFalse(recipe["persisted_in_archive"])

    def test_archive_contains_only_exact_small_inert_payloads(self) -> None:
        with zipfile.ZipFile(fixtures.ARCHIVE_PATH) as archive:
            self.assertEqual(archive.namelist(), sorted(fixtures.payloads()))
            for name, expected in fixtures.payloads().items():
                self.assertEqual(archive.read(name), expected)

    def test_manifest_mutation_and_parser_overclaim_fail(self) -> None:
        changed = copy.deepcopy(self.manifest)
        changed["cases"].pop()
        self.assertTrue(fixtures.validate_manifest(changed))
        changed = copy.deepcopy(self.manifest)
        changed["product_parser_support_claim"] = "supported"
        self.assertTrue(fixtures.validate_manifest(changed))

    def test_checked_corpus_matches_the_reproducible_builder(self) -> None:
        self.assertEqual(fixtures.check(), [])


if __name__ == "__main__":
    unittest.main()
