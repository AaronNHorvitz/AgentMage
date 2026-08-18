from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import sprint_17_evidence as evidence


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
                "artifact": "artifacts/sprints/sprint-17/installed-linux-git-matrix.json",
                "artifact_sha256": "c" * 64,
                "source_revision": "d" * 40,
                "target_ids": ["fedora-44-x86_64", "ubuntu-26.04-x86_64"],
                "git_operations": evidence.linux_evidence.GIT_OPERATIONS,
                "fixture_states": evidence.linux_evidence.FIXTURE_STATES,
                "attack_cases": evidence.linux_evidence.ATTACK_CASES,
                "instruction_classes": evidence.linux_evidence.INSTRUCTION_CLASSES,
            },
        )


class Sprint17EvidenceTests(unittest.TestCase):
    def test_local_contracts_pass_without_platform_or_release_overclaim(self) -> None:
        value = report()
        self.assertEqual(evidence.validate_report(value, verify_current=False), [])
        self.assertTrue(value["summary"]["local_contract_passed"])
        self.assertEqual(value["summary"]["sprint_status"], "BLOCKED")
        self.assertEqual(value["implemented_contracts"]["fixed_git_operations"], 13)
        self.assertEqual(value["implemented_contracts"]["injection_cases"], 200)
        self.assertEqual(value["implemented_contracts"]["git_mutation_operations"], 0)
        self.assertTrue(value["platform_evidence"]["linux_sandboxed_git_worker"])
        self.assertTrue(value["platform_evidence"]["linux_live_network_observation"])

    def test_platform_fuzz_review_and_release_overclaims_fail(self) -> None:
        mutations = (
            lambda value: value["summary"].update({"sprint_status": "PASS"}),
            lambda value: value["platform_evidence"].update(
                {"linux_sandboxed_git_worker": False}
            ),
            lambda value: value["platform_evidence"].update({"macos_git_worker": True}),
            lambda value: value["platform_evidence"].update(
                {"linux_live_network_observation": False}
            ),
            lambda value: value["verification_evidence"].update(
                {"manual_parser_fuzzing": True}
            ),
            lambda value: value["verification_evidence"].update({"independent_review": True}),
            lambda value: value["blockers"].pop(),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(evidence.validate_report(changed, verify_current=False))

    def test_command_security_and_contract_mutations_fail(self) -> None:
        mutations = (
            lambda value: value["commands"][0].update({"exit_code": 1}),
            lambda value: value["commands"].pop(),
            lambda value: value["security_requirement_ids"].pop(),
            lambda value: value["implemented_contracts"].update(
                {"git_mutation_operations": 1}
            ),
            lambda value: value["implemented_contracts"].update(
                {"authority_broadening_fields": 1}
            ),
            lambda value: value["implemented_contracts"][
                "linux_installed_git_matrix"
            ].update({"target_ids": ["fedora-44-x86_64"]}),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(evidence.validate_report(changed, verify_current=False))


if __name__ == "__main__":
    unittest.main()
