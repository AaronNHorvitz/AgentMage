from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import sprint_30_evidence as evidence


def commands() -> list[dict[str, object]]:
    return [
        {"id": identifier, "argv": list(argv), "exit_code": 0, "output_sha256": "a" * 64}
        for identifier, argv in evidence.COMMANDS
    ]


def report() -> dict[str, object]:
    with (
        patch.object(evidence, "git_file", return_value=b"source"),
        patch.object(evidence, "environment_manifest", return_value={
            "system": "Linux", "release": "fixture", "machine": "x86_64",
            "python": "fixture", "rustc": "fixture", "cargo": "fixture",
            "node": "fixture", "npm": "fixture",
        }),
    ):
        return evidence.build_report("b" * 40, commands())


class Sprint30EvidenceTests(unittest.TestCase):
    def test_local_contract_passes_without_model_runtime_or_release_overclaim(self) -> None:
        value = report()
        self.assertEqual(evidence.validate_report(value, verify_current=False), [])
        self.assertEqual(value["summary"]["sprint_status"], "BLOCKED")
        self.assertTrue(value["implemented_contracts"]["four_mode_integer_comparison_gate"])
        self.assertFalse(value["implemented_contracts"]["semantic_release_enabled"])

    def test_model_benchmark_remote_write_release_and_missing_local_proof_fail(self) -> None:
        mutations = (
            lambda value: value["summary"].update({"sprint_status": "PASS"}),
            lambda value: value["summary"].update({"semantic_release_enabled": True}),
            lambda value: value["summary"].update({"remote_semantic_capability": True}),
            lambda value: value["summary"].update({"source_write_authority": True}),
            lambda value: value["summary"].update({"release_approval": True}),
            lambda value: value["verification_evidence"].update(
                {"real_approved_semantic_profiles": True}
            ),
            lambda value: value["verification_evidence"].update(
                {"real_hardware_comparative_benchmark": True}
            ),
            lambda value: value["verification_evidence"].update(
                {"application_semantic_integration": False}
            ),
            lambda value: value["verification_evidence"].update({"independent_review": False}),
            lambda value: value["implemented_contracts"].update(
                {"real_approved_embedding_profile": True}
            ),
            lambda value: value["blockers"].pop(),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(evidence.validate_report(changed, verify_current=False))

    def test_command_security_environment_and_source_mutations_fail(self) -> None:
        mutations = (
            lambda value: value["commands"][0].update({"exit_code": 1}),
            lambda value: value["commands"].pop(),
            lambda value: value["security_requirement_ids"].pop(),
            lambda value: value["environment"].pop("rustc"),
            lambda value: value["source_sha256"].pop(next(iter(value["source_sha256"]))),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(evidence.validate_report(changed, verify_current=False))


if __name__ == "__main__":
    unittest.main()
