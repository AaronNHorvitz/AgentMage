from __future__ import annotations

import copy
import unittest

from scripts.story_1_1_security_evidence import (
    EXPECTED_REQUIREMENTS,
    build_map,
    check_map,
    validate_map,
)


class Story11SecurityEvidenceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.evidence_map = build_map()

    def test_checked_in_map_is_current(self) -> None:
        self.assertEqual(check_map(), [])

    def test_every_required_control_is_mapped_once(self) -> None:
        self.assertEqual(
            tuple(
                item["requirement_id"]
                for item in self.evidence_map["requirements"]
            ),
            EXPECTED_REQUIREMENTS,
        )

    def test_product_requirement_promotion_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.evidence_map)
        mutated["requirements"][0]["product_requirement_status"] = "complete"
        self.assertTrue(validate_map(mutated))

    def test_artifact_hash_mutation_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.evidence_map)
        mutated["artifacts"][0]["sha256"] = "0" * 64
        self.assertTrue(validate_map(mutated))

    def test_open_license_review_cannot_be_removed(self) -> None:
        mutated = copy.deepcopy(self.evidence_map)
        mutated["open_reviews"] = []
        self.assertTrue(validate_map(mutated))

    def test_macos_evidence_cannot_be_promoted(self) -> None:
        mutated = copy.deepcopy(self.evidence_map)
        mutated["macos"]["status"] = "pass"
        self.assertTrue(validate_map(mutated))

    def test_requirement_omission_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.evidence_map)
        mutated["requirements"].pop()
        self.assertTrue(validate_map(mutated))


if __name__ == "__main__":
    unittest.main()
