import copy
import unittest

from scripts import story_6_1_security_evidence as evidence


class Story61SecurityEvidenceTests(unittest.TestCase):
    def test_exact_requirements_remain_product_incomplete(self) -> None:
        value = evidence.build_map("0" * 40)
        self.assertEqual(
            [item["requirement_id"] for item in value["requirements"]],
            list(evidence.EXPECTED_REQUIREMENTS),
        )
        self.assertEqual(value["summary"]["product_requirements_complete"], 0)
        self.assertFalse(value["summary"]["story_gate_complete"])
        self.assertEqual(value["reviewer_protocol"]["protocol_id"], "RV-04")

    def test_omission_completion_and_unknown_evidence_fail_closed(self) -> None:
        value = evidence.build_map("0" * 40)
        omitted = copy.deepcopy(value)
        omitted["requirements"].pop()
        completed = copy.deepcopy(value)
        completed["requirements"][0]["product_requirement_status"] = "complete"
        unknown = copy.deepcopy(value)
        unknown["requirements"][0]["evidence"][0] = "unknown/path.json"
        for changed in (omitted, completed, unknown):
            with self.subTest():
                self.assertTrue(evidence.validate_map(changed))

    def test_hash_fuzzing_review_release_and_macos_overclaims_fail_closed(self) -> None:
        value = evidence.build_map("0" * 40)
        changes = []
        changed_hash = copy.deepcopy(value)
        changed_hash["artifacts"][0]["sha256"] = "0" * 64
        changes.append(changed_hash)
        for field, replacement in (
            ("fuzzing_claim", "coverage-guided-pass"),
            ("external_human_review_status", "pass"),
            ("product_requirement_completion_claim", "complete"),
            ("release_claim", "approved"),
            ("macos_execution_status", "pass"),
            ("macos_evidence_substituted", True),
        ):
            changed = copy.deepcopy(value)
            changed[field] = replacement
            changes.append(changed)
        for changed in changes:
            with self.subTest():
                self.assertTrue(evidence.validate_map(changed))


if __name__ == "__main__":
    unittest.main()
