from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import sprint_16_evidence as evidence


def commands() -> list[dict[str, object]]:
    return [
        {"id": identifier, "argv": list(argv), "exit_code": 0, "output_sha256": "a" * 64}
        for identifier, argv in evidence.COMMANDS
    ]


def report() -> dict[str, object]:
    with patch.object(evidence, "git_file", return_value=b"source"):
        return evidence.build_report(
            "b" * 40,
            commands(),
            {
                "artifact": "artifacts/sprints/sprint-16/installed-linux-worker-matrix.json",
                "artifact_sha256": "c" * 64,
                "source_revision": "d" * 40,
                "target_ids": ["fedora-44-x86_64", "ubuntu-26.04-x86_64"],
                "verified_operations": evidence.worker_evidence.VERIFIED_OPERATIONS,
                "linux_attack_cases": evidence.worker_evidence.ATTACK_CASES,
                "linux_lifecycle_cases": evidence.worker_evidence.LIFECYCLE_CASES,
            },
        )


class Sprint16EvidenceTests(unittest.TestCase):
    def test_local_contracts_pass_without_platform_or_release_overclaim(self) -> None:
        value = report()
        self.assertEqual(evidence.validate_report(value, verify_current=False), [])
        self.assertTrue(value["summary"]["local_contract_passed"])
        self.assertEqual(value["summary"]["sprint_status"], "BLOCKED")
        self.assertEqual(value["implemented_contracts"]["catalog_tools"], 10)
        self.assertEqual(value["implemented_contracts"]["write_capable_tools"], 0)
        self.assertTrue(value["implemented_contracts"]["packaged_worker_payload_declared"])
        self.assertTrue(value["platform_evidence"]["linux_packaged_live_worker"])

    def test_platform_cleanup_disclosure_and_review_claim_drift_fails(self) -> None:
        mutations = (
            lambda value: value["summary"].update({"sprint_status": "PASS"}),
            lambda value: value["platform_evidence"].update(
                {"linux_packaged_live_worker": False}
            ),
            lambda value: value["platform_evidence"].update(
                {"linux_complete_operation_matrix": False}
            ),
            lambda value: value["platform_evidence"].update(
                {"linux_live_attack_matrix": False}
            ),
            lambda value: value["platform_evidence"].update(
                {"linux_live_lifecycle_campaign": False}
            ),
            lambda value: value["platform_evidence"].update({"macos_xpc_worker": True}),
            lambda value: value["verification_evidence"].update({"live_cleanup_campaign": True}),
            lambda value: value["verification_evidence"].update(
                {"model_context_disclosure_redaction": False}
            ),
            lambda value: value["verification_evidence"].update({"independent_review": True}),
            lambda value: value["blockers"].pop(),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(evidence.validate_report(changed, verify_current=False))

    def test_command_security_and_catalog_mutations_fail(self) -> None:
        mutations = (
            lambda value: value["commands"][0].update({"exit_code": 1}),
            lambda value: value["commands"].pop(),
            lambda value: value["security_requirement_ids"].pop(),
            lambda value: value["implemented_contracts"].update({"write_capable_tools": 1}),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(evidence.validate_report(changed, verify_current=False))


if __name__ == "__main__":
    unittest.main()
