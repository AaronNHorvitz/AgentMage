from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import sprint_21_evidence as evidence


def commands() -> list[dict[str, object]]:
    return [{
        "id": identifier, "argv": list(argv), "exit_code": 0, "output_sha256": "a" * 64,
    } for identifier, argv in evidence.COMMANDS]


def report() -> dict[str, object]:
    with patch.object(evidence, "git_file", return_value=b"source"):
        return evidence.build_report("b" * 40, commands())


class Sprint21EvidenceTests(unittest.TestCase):
    def test_local_core_passes_without_sprint_or_integration_overclaim(self) -> None:
        value = report()
        self.assertEqual(evidence.validate_report(value, verify_current=False), [])
        self.assertEqual(value["summary"]["sprint_status"], "BLOCKED")
        self.assertTrue(value["implemented_contracts"]["receipt_chain"])
        self.assertFalse(value["implemented_contracts"]["integrity_key_stored_in_ledger"])

    def test_integration_downgrade_review_and_release_overclaims_fail(self) -> None:
        mutations = (
            lambda value: value["summary"].update({"sprint_status": "PASS"}),
            lambda value: value["summary"].update({"release_approval": True}),
            lambda value: value["verification_evidence"].update(
                {"production_held_file_integration": False}),
            lambda value: value["verification_evidence"].update(
                {"durable_anchor_integration": False}),
            lambda value: value["verification_evidence"].update({"clock_anomaly_matrix": False}),
            lambda value: value["verification_evidence"].update(
                {"native_linux_source_execution": False}),
            lambda value: value["verification_evidence"].update(
                {"runtime_answer_integration": False}),
            lambda value: value["verification_evidence"].update({"independent_review": True}),
            lambda value: value["blockers"].pop(),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(evidence.validate_report(changed, verify_current=False))

    def test_key_authority_command_security_and_source_mutations_fail(self) -> None:
        mutations = (
            lambda value: value["implemented_contracts"].update(
                {"integrity_key_stored_in_ledger": True}),
            lambda value: value["implemented_contracts"].update({"network_authority": True}),
            lambda value: value["commands"][0].update({"exit_code": 1}),
            lambda value: value["commands"].pop(),
            lambda value: value["security_requirement_ids"].pop(),
            lambda value: value["source_sha256"].pop(next(iter(value["source_sha256"]))),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(evidence.validate_report(changed, verify_current=False))


if __name__ == "__main__":
    unittest.main()
