"""Mutation tests for the Sprint 11 product-security evidence map."""

import copy
import unittest

from scripts import sprint_11_security_evidence as evidence


class Sprint11SecurityEvidenceTests(unittest.TestCase):
    def valid(self) -> dict:
        return {
            "artifact_id": "sprint-11-product-security-evidence-map",
            "artifacts": [{"bytes": 1, "path": path, "sha256": "b" * 64} for path in evidence.EVIDENCE_PATHS],
            "controls": [{"control_id": control_id, "product_status": "not-complete", **copy.deepcopy(evidence.MAPPINGS[control_id])} for control_id in evidence.EXPECTED_IDS],
            "external_network_used": False,
            "manual_fuzzing_executed": False,
            "private_user_data_used": False,
            "product_requirement_completion_claim": "none",
            "release_claim": "none",
            "schema_version": 1,
            "source_revision": "a" * 40,
            "status": "pass-story-security-mapping-with-open-product-controls",
            "summary": {"demonstrated_story_scope_count": 7, "mapped_control_count": 21, "partial_story_evidence_count": 14, "product_controls_complete": 0, "retained_artifact_count": len(evidence.EVIDENCE_PATHS)},
            "task_ids": ["11.1.3.5"],
            "verification_commands": evidence.expected_commands(),
        }

    def test_exact_map_is_closed_and_has_honest_counts(self) -> None:
        report = self.valid()
        self.assertEqual(evidence.validate_report(report), [])
        self.assertEqual(len(evidence.REQUIREMENT_IDS), 18)
        self.assertEqual(len(evidence.PROTOCOL_IDS), 3)
        self.assertEqual(sum(item["story_contribution"] == "demonstrated-story-scope" for item in report["controls"]), 7)

    def test_omission_reordering_and_mapping_mutation_fail(self) -> None:
        report = self.valid()
        report["controls"].pop()
        self.assertTrue(evidence.validate_report(report))
        report = self.valid()
        report["controls"][0], report["controls"][1] = report["controls"][1], report["controls"][0]
        self.assertTrue(evidence.validate_report(report))
        report = self.valid()
        report["controls"][0]["remaining"] = "none"
        self.assertTrue(evidence.validate_report(report))

    def test_product_release_and_fuzzing_overclaims_fail(self) -> None:
        for key, value in (("product_requirement_completion_claim", "complete"), ("release_claim", "release"), ("manual_fuzzing_executed", True)):
            report = self.valid()
            report[key] = value
            self.assertTrue(evidence.validate_report(report))
        report = self.valid()
        report["controls"][0]["product_status"] = "complete"
        self.assertTrue(evidence.validate_report(report))

    def test_artifact_path_hash_and_revision_mutations_fail(self) -> None:
        report = self.valid()
        report["artifacts"].pop()
        self.assertTrue(evidence.validate_report(report))
        report = self.valid()
        report["artifacts"][0]["sha256"] = "wrong"
        self.assertTrue(evidence.validate_report(report))
        report = self.valid()
        report["source_revision"] = "HEAD"
        self.assertTrue(evidence.validate_report(report))

    def test_private_data_network_and_command_mutations_fail(self) -> None:
        for key in ("private_user_data_used", "external_network_used"):
            report = self.valid()
            report[key] = True
            self.assertTrue(evidence.validate_report(report))
        report = self.valid()
        report["verification_commands"].pop()
        self.assertTrue(evidence.validate_report(report))


if __name__ == "__main__":
    unittest.main()
