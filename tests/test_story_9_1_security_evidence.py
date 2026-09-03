"""Tests for the bounded Story 9.1 product-security evidence map."""

from __future__ import annotations

import copy
import unittest
from unittest import mock

from scripts import story_9_1_security_evidence as evidence


class Story91SecurityEvidenceTests(unittest.TestCase):
    def valid_map(self) -> dict:
        return evidence.build_map("a" * 40)

    def test_current_inputs_and_bounded_map_are_valid(self) -> None:
        self.assertEqual(evidence.validate_inputs(), [])
        self.assertEqual(evidence.validate_map(self.valid_map()), [])

    def test_every_requirement_is_mapped_once_and_remains_incomplete(self) -> None:
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

    def test_missing_requirement_protocol_input_or_artifact_fails(self) -> None:
        cases = (
            ("requirements", "Story 9.1 security requirement closure is invalid"),
            ("reviewer_protocols", "Story 9.1 reviewer protocol states changed"),
            ("input_evidence", "Story 9.1 input evidence closure changed"),
            ("artifacts", "Story 9.1 retained evidence closure is stale"),
        )
        for field, expected in cases:
            with self.subTest(field=field):
                value = self.valid_map()
                value[field].pop()
                self.assertIn(expected, evidence.validate_map(value))

    def test_requirement_mapping_or_path_mutation_fails(self) -> None:
        changed_mapping = self.valid_map()
        changed_mapping["requirements"][0]["remaining"] = "nothing"
        self.assertIn(
            "Story 9.1 security mapping changed: SR-PLT-001",
            evidence.validate_map(changed_mapping),
        )

        unsafe_path = self.valid_map()
        unsafe_path["requirements"][0]["evidence"] = ["../outside"]
        failures = evidence.validate_map(unsafe_path)
        self.assertIn("Story 9.1 security mapping changed: SR-PLT-001", failures)
        self.assertIn(
            "Story 9.1 security evidence path is invalid: SR-PLT-001",
            failures,
        )

    def test_input_hash_or_revision_mutation_fails(self) -> None:
        for field, replacement in (("sha256", "0" * 64), ("evidence_revision", "0" * 40)):
            with self.subTest(field=field):
                value = self.valid_map()
                value["input_evidence"][0][field] = replacement
                self.assertIn(
                    "Story 9.1 input evidence closure changed",
                    evidence.validate_map(value),
                )

    def test_story_product_or_platform_promotion_fails(self) -> None:
        mutations = (
            ("external_independent_review_status", "passed"),
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
                    "Story 9.1 security evidence made an unsupported claim",
                    evidence.validate_map(value),
                )

        story_gate = self.valid_map()
        story_gate["summary"]["story_gate_complete"] = True
        self.assertIn(
            "Story 9.1 security summary is invalid",
            evidence.validate_map(story_gate),
        )

        requirement = self.valid_map()
        requirement["requirements"][0]["product_requirement_status"] = "complete"
        self.assertIn(
            "Story 9.1 security mapping changed: SR-PLT-001",
            evidence.validate_map(requirement),
        )

    def test_independent_review_blocker_is_exact(self) -> None:
        value = self.valid_map()
        self.assertEqual(
            value["external_independent_review_status"],
            "required-not-performed",
        )
        self.assertEqual(
            value["gate_blocker"],
            "G-DOD-12 independent critical-boundary review",
        )
        value["gate_blocker"] = "none"
        self.assertIn(
            "Story 9.1 security evidence made an unsupported claim",
            evidence.validate_map(value),
        )

    def test_parity_demotion_is_rejected_by_input_validation(self) -> None:
        values = copy.deepcopy(evidence.load_input_artifacts())
        values["linux-parity"]["summary"]["full_fedora_ubuntu_parity"] = False
        with mock.patch.object(evidence, "load_input_artifacts", return_value=values):
            self.assertIn(
                "Story 9.1 parity closure is incomplete",
                evidence.validate_inputs(),
            )

    def test_native_cleanup_or_private_value_failure_is_rejected(self) -> None:
        for field, replacement in (("cleanup_complete", False), ("test_count", 29)):
            with self.subTest(field=field):
                values = copy.deepcopy(evidence.load_input_artifacts())
                values["native-ubuntu-controls"]["summary"][field] = replacement
                with mock.patch.object(
                    evidence, "load_input_artifacts", return_value=values
                ):
                    self.assertIn(
                        "Story 9.1 native Ubuntu closure is incomplete",
                        evidence.validate_inputs(),
                    )

        private = copy.deepcopy(evidence.load_input_artifacts())
        private["native-ubuntu-controls"]["private_values_present"] = True
        with mock.patch.object(evidence, "load_input_artifacts", return_value=private):
            self.assertIn(
                "Story 9.1 native Ubuntu closure is incomplete",
                evidence.validate_inputs(),
            )


if __name__ == "__main__":
    unittest.main()
