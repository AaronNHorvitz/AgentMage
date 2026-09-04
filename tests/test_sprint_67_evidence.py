"""Mutation tests for the truthful Sprint 67 local evidence record."""

from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import sprint_67_evidence as evidence
from scripts.sprint_evidence_recorder import build_report, validate_report


def commands() -> list[dict[str, object]]:
    return [{
        "id": identifier, "argv": list(argv), "exit_code": 0,
        "output_sha256": "a" * 64,
        "blocking_skip_count": 0 if identifier in evidence.FOCUSED_COMMANDS else None,
    } for identifier, argv in evidence.COMMANDS]


def report() -> dict[str, object]:
    with patch("scripts.sprint_evidence_recorder.git_file", return_value=b"source"):
        return build_report(evidence.DEFINITION, "c" * 40, commands(), environment={})


class Sprint67EvidenceTests(unittest.TestCase):
    def validate(self, value: dict[str, object]) -> list[str]:
        with patch("scripts.sprint_evidence_recorder.git_file", return_value=b"source"):
            return validate_report(evidence.DEFINITION, value, verify_ancestry=False)

    def test_valid_report_preserves_blocked_truth(self) -> None:
        value = report()
        self.assertEqual(self.validate(value), [])
        self.assertTrue(value["summary"]["local_sprint_67_contract_passed"])
        self.assertFalse(value["verification_evidence"]["sprint_gate_closed"])
        self.assertEqual(value["summary"]["sprint_status"], "BLOCKED")

    def test_external_audio_integration_review_and_release_overclaims_fail(self) -> None:
        for field in (
            "upstream_sprint_66_closed", "audio_transcription_complete", "labeled_accuracy_complete",
            "every_type_receipts_complete", "native_parity_complete", "independent_review_present",
            "manual_fuzzing_complete", "release_approval",
        ):
            changed = copy.deepcopy(report())
            changed["summary"][field] = True
            self.assertTrue(self.validate(changed), field)

    def test_command_source_blocker_and_effect_drift_fails(self) -> None:
        mutations = (
            lambda value: value["commands"][0].update({"exit_code": 1}),
            lambda value: value["commands"][0].update({"blocking_skip_count": 1}),
            lambda value: value["commands"].pop(),
            lambda value: value["source_sha256"].pop(next(iter(value["source_sha256"]))),
            lambda value: value["blockers"].pop(),
            lambda value: value["verification_evidence"].update({"audio_engine_run_count": 1}),
            lambda value: value["verification_evidence"].update({"accepted_network_effect_count": 1}),
            lambda value: value["verification_evidence"].update({"accepted_execution_effect_count": 1}),
            lambda value: value["verification_evidence"].update({"accepted_filesystem_effect_count": 1}),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(self.validate(changed))


if __name__ == "__main__":
    unittest.main()
