"""Tests for the bounded Story 9.2 product-security evidence map."""

from __future__ import annotations

import copy
import unittest
from unittest import mock

from scripts import story_9_2_security_evidence as evidence


class Story92SecurityEvidenceTests(unittest.TestCase):
    def valid_map(self) -> dict:
        return evidence.build_map("a" * 40)

    def test_current_inputs_and_bounded_map_are_valid(self) -> None:
        self.assertEqual(evidence.validate_inputs(), [])
        self.assertEqual(evidence.validate_map(self.valid_map()), [])

    def test_requirements_are_exact_and_product_incomplete(self) -> None:
        value = self.valid_map()
        self.assertEqual(
            [item["requirement_id"] for item in value["requirements"]],
            list(evidence.EXPECTED_REQUIREMENTS),
        )
        self.assertTrue(
            all(
                item["product_requirement_status"] == "not-complete"
                for item in value["requirements"]
            )
        )

    def test_missing_map_sections_fail(self) -> None:
        cases = (
            ("requirements", "Story 9.2 security requirement closure is invalid"),
            ("reviewer_protocols", "Story 9.2 reviewer protocol states changed"),
            ("input_evidence", "Story 9.2 input evidence closure changed"),
            ("artifacts", "Story 9.2 retained evidence closure is stale"),
        )
        for field, expected in cases:
            with self.subTest(field=field):
                value = self.valid_map()
                value[field].pop()
                self.assertIn(expected, evidence.validate_map(value))

    def test_mapping_path_and_root_collector_gap_are_fixed(self) -> None:
        value = self.valid_map()
        self.assertIn("effective UID 0", value["known_blockers"][0])
        value["requirements"][0]["remaining"] = "nothing"
        self.assertIn(
            "Story 9.2 security mapping changed: SR-PLT-001",
            evidence.validate_map(value),
        )
        unsafe = self.valid_map()
        unsafe["requirements"][0]["evidence"] = ["../outside"]
        failures = evidence.validate_map(unsafe)
        self.assertIn("Story 9.2 security evidence path is invalid: SR-PLT-001", failures)

    def test_parity_demotion_fails(self) -> None:
        value = self.valid_map()
        value["parity_report"]["dimensions"][0]["status"] = "blocked"
        self.assertIn("Story 9.2 parity report changed", evidence.validate_map(value))

        inputs = copy.deepcopy(evidence.load_inputs())
        inputs["docker-controls"]["targets"][1]["observation"]["cases"][0][
            "observed_refusal"
        ] = "docker-preflight.observation.identity"
        with mock.patch.object(evidence, "load_inputs", return_value=inputs):
            changed = evidence.parity_report(inputs)
        self.assertFalse(changed["full_declared_kvm_parity"])

    def test_product_platform_inference_or_review_promotion_fails(self) -> None:
        mutations = (
            ("gate_blocker", "none"),
            ("inference_claim", "performed"),
            ("product_requirement_completion_claim", "complete"),
            ("physical_host_certification_claim", "certified"),
            ("release_claim", "supported"),
            ("macos_evidence_substituted", True),
            ("private_user_data_used", True),
            ("network_used_while_mapping", True),
        )
        for field, replacement in mutations:
            with self.subTest(field=field):
                value = self.valid_map()
                value[field] = replacement
                self.assertIn(
                    "Story 9.2 security evidence made an unsupported claim",
                    evidence.validate_map(value),
                )

    def test_story_gate_and_blocker_removal_fail(self) -> None:
        value = self.valid_map()
        value["summary"]["story_gate_complete"] = True
        self.assertIn("Story 9.2 security summary is invalid", evidence.validate_map(value))

        value = self.valid_map()
        value["known_blockers"] = []
        self.assertIn("Story 9.2 known blockers changed", evidence.validate_map(value))


if __name__ == "__main__":
    unittest.main()
