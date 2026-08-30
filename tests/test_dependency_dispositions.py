from __future__ import annotations

import copy
import unittest

from scripts.dependency_dispositions import (
    EXPECTED_IDS,
    FAMILIES,
    load_record,
    validate_record,
)


class DependencyDispositionTests(unittest.TestCase):
    def setUp(self) -> None:
        self.record = load_record()

    def _candidate(self, record: dict, candidate_id: str) -> dict:
        return next(item for item in record["candidates"] if item["id"] == candidate_id)

    def test_canonical_record_passes(self) -> None:
        self.assertEqual(validate_record(self.record), [])

    def test_inventory_is_complete_sorted_and_covers_every_family(self) -> None:
        self.assertEqual([item["id"] for item in self.record["candidates"]], EXPECTED_IDS)
        self.assertEqual({item["family"] for item in self.record["candidates"]}, set(FAMILIES))

    def test_unknown_fields_fail_closed(self) -> None:
        root = copy.deepcopy(self.record)
        root["enables_ocr"] = True
        self.assertTrue(validate_record(root))
        candidate = copy.deepcopy(self.record)
        candidate["candidates"][0]["network_allowed"] = True
        self.assertTrue(validate_record(candidate))

    def test_candidates_cannot_be_removed_reordered_or_duplicated(self) -> None:
        removed = copy.deepcopy(self.record)
        removed["candidates"].pop()
        self.assertIn(
            "candidate inventory must be complete, unique, and sorted",
            validate_record(removed),
        )
        reordered = copy.deepcopy(self.record)
        reordered["candidates"][0], reordered["candidates"][1] = (
            reordered["candidates"][1],
            reordered["candidates"][0],
        )
        self.assertIn(
            "candidate inventory must be complete, unique, and sorted",
            validate_record(reordered),
        )
        duplicated = copy.deepcopy(self.record)
        duplicated["candidates"][-1] = copy.deepcopy(duplicated["candidates"][0])
        self.assertIn(
            "candidate inventory must be complete, unique, and sorted",
            validate_record(duplicated),
        )

    def test_deferred_or_rejected_candidate_cannot_be_promoted_by_status(self) -> None:
        for candidate_id in (
            "mime-file-format-0.29.0",
            "ocr-tesseract-5.5.3",
            "ocr-cloud-service",
            "database-tantivy-0.26.1",
        ):
            with self.subTest(candidate=candidate_id):
                mutated = copy.deepcopy(self.record)
                self._candidate(mutated, candidate_id)["disposition"] = "accepted"
                self.assertIn(
                    f"{candidate_id}: candidate cannot be promoted by status mutation",
                    validate_record(mutated),
                )

    def test_accepted_component_version_and_checksum_are_bound_to_provenance(self) -> None:
        version = copy.deepcopy(self.record)
        zip_component = self._candidate(version, "archive-zip-8.6.0")["components"][0]
        zip_component["version"] = "8.6.1"
        self.assertIn(
            "archive-zip-8.6.0: accepted version differs from provenance",
            validate_record(version),
        )
        checksum = copy.deepcopy(self.record)
        zip_component = self._candidate(checksum, "archive-zip-8.6.0")["components"][0]
        zip_component["checksum_sha256"] = "0" * 64
        self.assertIn(
            "archive-zip-8.6.0: accepted checksum differs from provenance",
            validate_record(checksum),
        )

    def test_internal_boundaries_cannot_smuggle_a_package(self) -> None:
        mutated = copy.deepcopy(self.record)
        self._candidate(mutated, "mime-in-tree-bounded-probes-v1")["components"] = [
            {"name": "infer", "version": "0.22.0", "checksum_sha256": "0" * 64}
        ]
        self.assertIn(
            "mime-in-tree-bounded-probes-v1: internal boundary must not declare a package",
            validate_record(mutated),
        )

    def test_nonaccepted_candidate_must_remain_unadmitted(self) -> None:
        mutated = copy.deepcopy(self.record)
        self._candidate(mutated, "ocr-tesseract-5.5.3")["components"] = [
            {"name": "tesseract", "version": "5.5.3", "checksum_sha256": "0" * 64}
        ]
        self.assertIn(
            "ocr-tesseract-5.5.3: deferred/rejected package must remain unadmitted",
            validate_record(mutated),
        )

    def test_archive_controls_are_closed_over_hostile_expansion(self) -> None:
        for control in ("entry-count", "expanded-bytes", "ratio", "recursion-depth", "path-safety"):
            with self.subTest(control=control):
                mutated = copy.deepcopy(self.record)
                candidate = self._candidate(mutated, "archive-zip-8.6.0")
                candidate["resource_controls"].remove(control)
                self.assertIn(
                    "archive-zip-8.6.0: archive controls are incomplete",
                    validate_record(mutated),
                )

    def test_ocr_controls_cannot_omit_provenance_or_isolation(self) -> None:
        for control in ("model-provenance", "confidence", "language", "process-isolation", "residue"):
            with self.subTest(control=control):
                mutated = copy.deepcopy(self.record)
                candidate = self._candidate(mutated, "ocr-tesseract-5.5.3")
                candidate["resource_controls"].remove(control)
                self.assertIn(
                    "ocr-tesseract-5.5.3: OCR controls are incomplete",
                    validate_record(mutated),
                )

    def test_tokenizer_mime_and_database_authority_cannot_widen(self) -> None:
        cases = (
            ("tokenization-profile-owned-exact-counter", "exact-profile-identity", "tokenization-profile-owned-exact-counter: tokenizer must bind an exact profile"),
            ("mime-in-tree-bounded-probes-v1", "extension-advisory-only", "mime-in-tree-bounded-probes-v1: extensions must remain advisory"),
            ("database-rusqlite-0.40.2", "single-sqlite-authority", "database-rusqlite-0.40.2: database authority must remain singular"),
        )
        for candidate_id, control, expected in cases:
            with self.subTest(candidate=candidate_id):
                mutated = copy.deepcopy(self.record)
                self._candidate(mutated, candidate_id)["resource_controls"].remove(control)
                self.assertIn(expected, validate_record(mutated))

    def test_platform_and_product_truth_cannot_be_promoted(self) -> None:
        platform = copy.deepcopy(self.record)
        candidate = self._candidate(platform, "parser-lopdf-0.44.0")
        candidate["platforms"]["windows-11-x86_64"] = "supported"
        self.assertIn(
            "parser-lopdf-0.44.0: platform matrix makes an unsupported claim",
            validate_record(platform),
        )
        product = copy.deepcopy(self.record)
        product["product_truth"]["enables_capability"] = True
        self.assertIn("product truth was widened", validate_record(product))

    def test_source_hash_and_retrieval_date_are_pinned(self) -> None:
        digest = copy.deepcopy(self.record)
        self._candidate(digest, "ocr-tesseract-5.5.3")["provenance"]["source_sha256"] = "bad"
        self.assertIn(
            "ocr-tesseract-5.5.3: source_sha256 must be lowercase SHA-256",
            validate_record(digest),
        )
        date = copy.deepcopy(self.record)
        self._candidate(date, "ocr-tesseract-5.5.3")["provenance"]["retrieved_on"] = "2026-08-28"
        self.assertIn(
            "ocr-tesseract-5.5.3: retrieval date drifted",
            validate_record(date),
        )


if __name__ == "__main__":
    unittest.main()
