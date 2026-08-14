from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import sprint_19_evidence as evidence


def commands() -> list[dict[str, object]]:
    return [
        {"id": identifier, "argv": list(argv), "exit_code": 0, "output_sha256": "a" * 64}
        for identifier, argv in evidence.COMMANDS
    ]


def grammar_bom() -> dict[str, object]:
    return {
        "schema_version": 1,
        "grammar_set_sha256": "b" * 64,
        "grammars": [{"language": value} for value in range(6)],
    }


def report() -> dict[str, object]:
    with patch.object(evidence, "git_file", return_value=b"source"):
        return evidence.build_report("c" * 40, commands(), grammar_bom())


class Sprint19EvidenceTests(unittest.TestCase):
    def test_local_contracts_pass_without_sprint_or_platform_overclaim(self) -> None:
        value = report()
        self.assertEqual(evidence.validate_report(value, verify_current=False), [])
        self.assertEqual(value["summary"]["sprint_status"], "BLOCKED")
        self.assertEqual(value["implemented_contracts"]["render_priority_tiers"], 6)
        self.assertTrue(value["verification_evidence"]["disposable_git_invariance"])

    def test_platform_fuzz_review_and_release_overclaims_fail(self) -> None:
        mutations = (
            lambda value: value["summary"].update({"sprint_status": "PASS"}),
            lambda value: value["platform_evidence"].update({"linux_packaged_worker": True}),
            lambda value: value["platform_evidence"].update({"macos_native_map": True}),
            lambda value: value["platform_evidence"].update({"windows_native_map": True}),
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

    def test_command_bom_security_and_golden_mutations_fail(self) -> None:
        mutations = (
            lambda value: value["commands"][0].update({"exit_code": 1}),
            lambda value: value["commands"].pop(),
            lambda value: value["security_requirement_ids"].pop(),
            lambda value: value["grammar_bom"]["grammars"].pop(),
            lambda value: value["implemented_contracts"].update(
                {"golden_map_sha256": "d" * 64}
            ),
            lambda value: value["implemented_contracts"].update({"network_authority": True}),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(evidence.validate_report(changed, verify_current=False))


if __name__ == "__main__":
    unittest.main()
