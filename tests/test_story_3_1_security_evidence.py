from __future__ import annotations

import copy
import tempfile
import unittest
from pathlib import Path

from scripts.story_3_1_security_evidence import (
    EVIDENCE_PATHS,
    EXPECTED_REQUIREMENTS,
    MAPPINGS,
    REPORT_PATH,
    build_map,
    check_map,
    read_json,
    validate_map,
)


class Story31SecurityEvidenceTests(unittest.TestCase):
    def test_checked_map_is_current_and_exact(self) -> None:
        self.assertEqual(check_map(), [])
        self.assertEqual(read_json(REPORT_PATH), build_map())

    def test_requirement_mappings_are_exact_and_product_incomplete(self) -> None:
        evidence_map = build_map()
        self.assertEqual(
            [item["requirement_id"] for item in evidence_map["requirements"]],
            list(EXPECTED_REQUIREMENTS),
        )
        self.assertEqual(evidence_map["summary"]["product_requirements_complete"], 0)
        for item in evidence_map["requirements"]:
            self.assertEqual(
                item,
                {"requirement_id": item["requirement_id"], **MAPPINGS[item["requirement_id"]]},
            )
            self.assertTrue(item["remaining"])

    def test_requirement_omission_reordering_and_weakening_fail_closed(self) -> None:
        evidence_map = build_map()
        omitted = copy.deepcopy(evidence_map)
        omitted["requirements"].pop()
        reordered = copy.deepcopy(evidence_map)
        reordered["requirements"].reverse()
        completed = copy.deepcopy(evidence_map)
        completed["requirements"][0]["story_contribution"] = "complete"
        no_remaining = copy.deepcopy(evidence_map)
        no_remaining["requirements"][0]["remaining"] = ""
        for changed in (omitted, reordered, completed, no_remaining):
            self.assertTrue(validate_map(changed))

    def test_artifact_hash_omission_and_unknown_path_fail_closed(self) -> None:
        evidence_map = build_map()
        changed_hash = copy.deepcopy(evidence_map)
        changed_hash["artifacts"][0]["sha256"] = "0" * 64
        omitted = copy.deepcopy(evidence_map)
        omitted["artifacts"].pop()
        unknown = copy.deepcopy(evidence_map)
        unknown["requirements"][0]["evidence"][0] = "unreviewed/evidence.json"
        for changed in (changed_hash, omitted, unknown):
            self.assertTrue(validate_map(changed))

    def test_product_release_and_macos_overclaims_fail_closed(self) -> None:
        evidence_map = build_map()
        product = copy.deepcopy(evidence_map)
        product["product_startup_activation_claim"] = "pass"
        release = copy.deepcopy(evidence_map)
        release["release_claim"] = "pass"
        macos = copy.deepcopy(evidence_map)
        macos["macos_execution_status"] = "pass"
        substitution = copy.deepcopy(evidence_map)
        substitution["macos_evidence_substituted"] = True
        for changed in (product, release, macos, substitution):
            self.assertTrue(validate_map(changed))

    def test_missing_evidence_prevents_map_validation(self) -> None:
        evidence_map = build_map()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for relative in EVIDENCE_PATHS[:-1]:
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b"fixture")
            self.assertTrue(validate_map(evidence_map, root))


if __name__ == "__main__":
    unittest.main()
