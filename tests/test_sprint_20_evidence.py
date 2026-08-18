from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import sprint_20_evidence as evidence


def commands() -> list[dict[str, object]]:
    return [
        {"id": identifier, "argv": list(argv), "exit_code": 0, "output_sha256": "a" * 64}
        for identifier, argv in evidence.COMMANDS
    ]


def report() -> dict[str, object]:
    with patch.object(evidence, "git_file", return_value=b"source"):
        return evidence.build_report("b" * 40, commands())


class Sprint20EvidenceTests(unittest.TestCase):
    def test_local_contract_passes_without_sprint_or_authority_overclaim(self) -> None:
        value = report()
        self.assertEqual(evidence.validate_report(value, verify_current=False), [])
        self.assertEqual(value["summary"]["sprint_status"], "BLOCKED")
        self.assertEqual(value["implemented_contracts"]["material_claim_state_count"], 4)
        self.assertEqual(value["implemented_contracts"]["unknown_blocked_reason_count"], 8)
        self.assertTrue(
            value["implemented_contracts"][
                "successful_answer_requires_evidence_assignment"
            ]
        )
        self.assertTrue(
            value["verification_evidence"][
                "production_answer_assignment_integration"
            ]
        )
        self.assertNotIn(
            "PRODUCTION-ANSWER-CLAIM-COMPOSITION-NOT-INTEGRATED",
            {item["code"] for item in value["blockers"]},
        )
        self.assertFalse(value["implemented_contracts"]["assignment_authority"])

    def test_sprint_review_and_release_overclaims_fail(self) -> None:
        mutations = (
            lambda value: value["summary"].update({"sprint_status": "PASS"}),
            lambda value: value["summary"].update({"release_approval": True}),
            lambda value: value["verification_evidence"].update(
                {"sprint_21_receipt_integrity_owned_downstream": False}
            ),
            lambda value: value["verification_evidence"].update(
                {"sprint_21_citation_freshness_owned_downstream": False}
            ),
            lambda value: value["verification_evidence"].update({"independent_review": True}),
            lambda value: value["blockers"].pop(),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(evidence.validate_report(changed, verify_current=False))

    def test_authority_confidence_command_security_and_source_mutations_fail(self) -> None:
        mutations = (
            lambda value: value["implemented_contracts"].update({"network_authority": True}),
            lambda value: value["implemented_contracts"].update({"completion_authority": True}),
            lambda value: value["implemented_contracts"].update(
                {"model_confidence_is_evidence_state": True}
            ),
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
