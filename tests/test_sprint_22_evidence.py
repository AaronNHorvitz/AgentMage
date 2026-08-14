from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import sprint_22_evidence as evidence


def commands() -> list[dict[str, object]]:
    return [{
        "id": identifier, "argv": list(argv), "exit_code": 0, "output_sha256": "a" * 64,
    } for identifier, argv in evidence.COMMANDS]


def report() -> dict[str, object]:
    with patch.object(evidence, "git_file", return_value=b"source"):
        return evidence.build_report("b" * 40, commands())


class Sprint22EvidenceTests(unittest.TestCase):
    def test_local_core_passes_without_sprint_or_release_overclaim(self) -> None:
        value = report()
        self.assertEqual(evidence.validate_report(value, verify_current=False), [])
        self.assertEqual(value["summary"]["sprint_status"], "BLOCKED")
        self.assertEqual(value["implemented_contracts"]["subprocess_crash_runs"], 126)
        self.assertFalse(value["implemented_contracts"]["resume_authority"])

    def test_integration_native_canary_review_and_release_overclaims_fail(self) -> None:
        mutations = (
            lambda value: value["summary"].update({"sprint_status": "PASS"}),
            lambda value: value["summary"].update({"release_approval": True}),
            lambda value: value["verification_evidence"].update(
                {"production_context_integration": True}),
            lambda value: value["verification_evidence"].update(
                {"native_platform_crash_matrix": True}),
            lambda value: value["verification_evidence"].update(
                {"product_wide_canary_sweep": True}),
            lambda value: value["verification_evidence"].update({"independent_review": True}),
            lambda value: value["blockers"].pop(),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(evidence.validate_report(changed, verify_current=False))

    def test_authority_persistence_command_security_and_source_mutations_fail(self) -> None:
        mutations = (
            lambda value: value["implemented_contracts"].update({"resume_authority": True}),
            lambda value: value["implemented_contracts"].update(
                {"ephemeral_checkpoint_persistence": True}),
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
