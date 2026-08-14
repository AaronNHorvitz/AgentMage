from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import muse_profile_evaluation_evidence as evidence


def summary() -> dict[str, object]:
    cases = []
    for index, (case_id, category) in enumerate(evidence.EXPECTED_CASES.items()):
        cases.append(
            {
                "case_id": case_id,
                "category": category,
                "trial_count": 3,
                "closed_proposal_count": 0,
                "passed_count": 0,
                "false_completion_count": 0,
                "response_sha256": [f"{index + 1:064x}"] * 3,
                "output_tokens": 192,
                "elapsed_ms": 3000,
            }
        )
    return {
        "schema_version": 1,
        "record_type": "muse_live_profile_evaluation_summary",
        "quality_profile_id": evidence.QUALITY_PROFILE,
        "quality_manifest_sha256": evidence.QUALITY_MANIFEST,
        "repeatability_profile_id": evidence.REPEAT_PROFILE,
        "repeatability_manifest_sha256": evidence.REPEAT_MANIFEST,
        "quality_trial_count": 12,
        "quality_closed_proposal_count": 0,
        "quality_pass_count": 0,
        "false_completion_count": 0,
        "case_summaries": cases,
        "repeatability_trial_count": 5,
        "repeatability_unique_response_count": 1,
        "repeatability_response_sha256": ["f" * 64] * 5,
        "maximum_resident_memory_bytes": 1024,
        "maximum_accelerator_memory_bytes": 2048,
        "socket_residue_count": 0,
        "raw_output_retained": False,
    }


def report() -> dict[str, object]:
    with patch.object(evidence, "git_file", return_value=b"committed-source"):
        return evidence.build_report(summary(), "a" * 40)


class MuseProfileEvaluationEvidenceTests(unittest.TestCase):
    def test_executed_summary_is_closed_and_reconciled(self) -> None:
        self.assertEqual(evidence.validate_summary(summary()), [])
        self.assertEqual(evidence.wilson_interval(0, 12), [0.0, 0.242494])

    def test_summary_identity_counts_hashes_and_residue_fail_closed(self) -> None:
        mutations = (
            lambda value: value.update({"quality_profile_id": "changed"}),
            lambda value: value.update({"quality_trial_count": 11}),
            lambda value: value["case_summaries"][0].update({"category": "changed"}),
            lambda value: value["case_summaries"][0]["response_sha256"].pop(),
            lambda value: value.update({"repeatability_unique_response_count": 2}),
            lambda value: value.update({"socket_residue_count": 1}),
            lambda value: value.update({"raw_output_retained": True}),
            lambda value: value.update({"unknown": True}),
        )
        for mutate in mutations:
            changed = copy.deepcopy(summary())
            mutate(changed)
            self.assertTrue(evidence.validate_summary(changed))

    def test_report_rejects_failed_quality_without_overclaiming_repeatability(self) -> None:
        value = report()
        self.assertEqual(evidence.validate_report(value), [])
        self.assertEqual(value["disposition"]["status"], "REJECTED")
        self.assertFalse(
            value["diagnostic_repeatability"]["universal_determinism_claim"]
        )
        self.assertFalse(value["retention"]["raw_output_retained"])

    def test_report_activation_release_and_determinism_mutations_fail(self) -> None:
        value = report()
        mutations = (
            lambda value: value["disposition"].update({"status": "PASS-EVALUATION"}),
            lambda value: value["disposition"].update({"product_profile_enabled": True}),
            lambda value: value["disposition"].update({"release_approval": True}),
            lambda value: value["diagnostic_repeatability"].update(
                {"universal_determinism_claim": True}
            ),
            lambda value: value["resources"].update({"socket_residue_count": 1}),
        )
        for mutate in mutations:
            changed = copy.deepcopy(value)
            mutate(changed)
            self.assertTrue(evidence.validate_report(changed))


if __name__ == "__main__":
    unittest.main()
