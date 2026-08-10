from __future__ import annotations

import copy
import tempfile
import unittest
from pathlib import Path

from scripts.story_3_2_security_evidence import (
    EVIDENCE_PATHS,
    EXPECTED_REQUIREMENTS,
    MAPPINGS,
    REPORT_PATH,
    build_map,
    check_map,
    read_json,
    validate_map,
)


class Story32SecurityEvidenceTests(unittest.TestCase):
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
            self.assertEqual(item["product_requirement_status"], "not-complete")
            self.assertEqual(
                item,
                {
                    "requirement_id": item["requirement_id"],
                    "product_requirement_status": "not-complete",
                    **MAPPINGS[item["requirement_id"]],
                },
            )
            self.assertTrue(item["remaining"])

    def test_required_support_patch_test_and_tabletop_evidence_is_retained(self) -> None:
        paths = set(EVIDENCE_PATHS)
        self.assertIn("support/vulnerability-support-policy.json", paths)
        self.assertIn("schemas/support/signed-manual-patch-metadata.schema.json", paths)
        self.assertIn("tests/test_manual_patch_verifier.py", paths)
        self.assertIn("tests/test_emergency_disable_policy.py", paths)
        self.assertIn(
            "docs/security/story-3.2-vulnerability-response-tabletop.md", paths
        )

    def test_requirement_omission_reordering_and_weakening_fail_closed(self) -> None:
        evidence_map = build_map()
        omitted = copy.deepcopy(evidence_map)
        omitted["requirements"].pop()
        reordered = copy.deepcopy(evidence_map)
        reordered["requirements"].reverse()
        completed = copy.deepcopy(evidence_map)
        completed["requirements"][0]["product_requirement_status"] = "complete"
        no_remaining = copy.deepcopy(evidence_map)
        no_remaining["requirements"][0]["remaining"] = ""
        for changed in (omitted, reordered, completed, no_remaining):
            with self.subTest(changed=changed):
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
            with self.subTest(changed=changed):
                self.assertTrue(validate_map(changed))

    def test_rv_product_release_network_and_macos_overclaims_fail_closed(self) -> None:
        evidence_map = build_map()
        rv_21 = copy.deepcopy(evidence_map)
        rv_21["summary"]["rv_21_complete"] = True
        rv_22 = copy.deepcopy(evidence_map)
        rv_22["summary"]["rv_22_complete"] = True
        product = copy.deepcopy(evidence_map)
        product["product_patch_claim"] = "pass"
        release = copy.deepcopy(evidence_map)
        release["release_claim"] = "pass"
        network = copy.deepcopy(evidence_map)
        network["network_used"] = True
        macos = copy.deepcopy(evidence_map)
        macos["macos_execution_status"] = "pass"
        substitution = copy.deepcopy(evidence_map)
        substitution["macos_evidence_substituted"] = True
        for changed in (rv_21, rv_22, product, release, network, macos, substitution):
            with self.subTest(changed=changed):
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
