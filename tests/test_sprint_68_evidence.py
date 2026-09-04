"""Mutation tests for truthful Sprint 68 local evidence."""

import copy
import unittest
from unittest.mock import patch

from scripts import sprint_68_evidence as evidence
from scripts.sprint_evidence_recorder import build_report, validate_report


def commands() -> list[dict[str, object]]:
    return [{"id": identifier, "argv": list(argv), "exit_code": 0, "output_sha256": "a" * 64,
             "blocking_skip_count": 0 if identifier in evidence.FOCUSED_COMMANDS else None}
            for identifier, argv in evidence.COMMANDS]


def report() -> dict[str, object]:
    with patch("scripts.sprint_evidence_recorder.git_file", return_value=b"source"):
        return build_report(evidence.DEFINITION, "c" * 40, commands(), environment={})


class Sprint68EvidenceTests(unittest.TestCase):
    def validate(self, value: dict[str, object]) -> list[str]:
        with patch("scripts.sprint_evidence_recorder.git_file", return_value=b"source"):
            return validate_report(evidence.DEFINITION, value, verify_ancestry=False)

    def test_valid_report_preserves_blocked_truth(self) -> None:
        value = report()
        self.assertEqual(self.validate(value), [])
        self.assertTrue(value["summary"]["local_sprint_68_contract_passed"])
        self.assertEqual(value["summary"]["sprint_status"], "BLOCKED")

    def test_external_and_live_overclaims_fail(self) -> None:
        for field in ("upstream_sprint_67_closed", "native_parity_complete", "native_failure_campaign_complete", "independent_review_present", "manual_fuzzing_complete", "live_access_enabled", "release_approval"):
            changed = copy.deepcopy(report()); changed["summary"][field] = True
            self.assertTrue(self.validate(changed), field)

    def test_command_source_blocker_and_authority_drift_fails(self) -> None:
        mutations = (
            lambda value: value["commands"][0].update({"exit_code": 1}),
            lambda value: value["commands"].pop(),
            lambda value: value["source_sha256"].pop(next(iter(value["source_sha256"]))),
            lambda value: value["blockers"].pop(),
            lambda value: value["verification_evidence"].update({"live_connector_count": 1}),
            lambda value: value["verification_evidence"].update({"raw_sql_entry_point_count": 1}),
            lambda value: value["verification_evidence"].update({"accepted_network_effect_count": 1}),
            lambda value: value["verification_evidence"].update({"accepted_filesystem_effect_count": 1}),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report()); mutate(changed)
            self.assertTrue(self.validate(changed))


if __name__ == "__main__": unittest.main()
