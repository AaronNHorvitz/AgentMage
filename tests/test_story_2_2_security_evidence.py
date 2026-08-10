from __future__ import annotations

import copy
import unittest

from scripts.story_2_2_security_evidence import (
    DISPOSITION_PATH,
    EXPECTED_REQUIREMENTS,
    MAP_PATH,
    PROTOCOL_PATH,
    build_disposition,
    build_map,
    build_protocol,
    check_all,
    read_json,
    validate_disposition,
    validate_map,
    validate_protocol,
)


class Story22SecurityEvidenceTests(unittest.TestCase):
    def test_checked_security_evidence_is_current(self) -> None:
        self.assertEqual(check_all(), [])
        self.assertEqual(read_json(PROTOCOL_PATH), build_protocol())
        self.assertEqual(read_json(DISPOSITION_PATH), build_disposition())
        self.assertEqual(read_json(MAP_PATH), build_map())

    def test_required_security_mappings_are_exact_and_not_complete(self) -> None:
        evidence_map = build_map()
        self.assertEqual(
            [item["requirement_id"] for item in evidence_map["requirements"]],
            list(EXPECTED_REQUIREMENTS),
        )
        self.assertTrue(
            all(
                item["product_requirement_status"] == "not-complete"
                for item in evidence_map["requirements"]
            )
        )

    def test_rv_15_foundation_is_complete_without_full_protocol_claim(self) -> None:
        protocol = build_protocol()
        self.assertEqual(protocol["foundation_execution_status"], "COMPLETE")
        self.assertEqual(protocol["full_protocol_status"], "NOT_COMPLETE")
        self.assertEqual(protocol["observations"]["registered_target_count"], 12)
        self.assertEqual(protocol["observations"]["blocking_gate_result_count"], 6)

    def test_artifact_hash_and_requirement_omission_fail_closed(self) -> None:
        evidence_map = build_map()
        artifact = copy.deepcopy(evidence_map)
        artifact["artifacts"][0]["sha256"] = "0" * 64
        omitted = copy.deepcopy(evidence_map)
        omitted["requirements"].pop()
        duplicated = copy.deepcopy(evidence_map)
        duplicated["requirements"][1] = copy.deepcopy(
            duplicated["requirements"][0]
        )
        for changed in (artifact, omitted, duplicated):
            self.assertTrue(validate_map(changed))

    def test_full_protocol_and_product_execution_overclaims_fail_closed(self) -> None:
        protocol = build_protocol()
        full = copy.deepcopy(protocol)
        full["full_protocol_status"] = "COMPLETE"
        product = copy.deepcopy(protocol)
        product["product_boundary_execution_claim"] = "pass"
        self.assertTrue(validate_protocol(full))
        self.assertTrue(validate_protocol(product))

    def test_independent_review_and_release_overclaims_fail_closed(self) -> None:
        disposition = build_disposition()
        review = copy.deepcopy(disposition)
        review["independent_review_performed"] = True
        release = copy.deepcopy(disposition)
        release["release_approval"] = True
        self.assertTrue(validate_disposition(review))
        self.assertTrue(validate_disposition(release))

    def test_product_requirement_and_macos_overclaims_fail_closed(self) -> None:
        evidence_map = build_map()
        product = copy.deepcopy(evidence_map)
        product["requirements"][0]["product_requirement_status"] = "complete"
        macos = copy.deepcopy(evidence_map)
        macos["macos"]["status"] = "pass"
        self.assertTrue(validate_map(product))
        self.assertTrue(validate_map(macos))


if __name__ == "__main__":
    unittest.main()
