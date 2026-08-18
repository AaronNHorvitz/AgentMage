from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import sprint_23_evidence as evidence


def commands() -> list[dict[str, object]]:
    return [{
        "id": identifier, "argv": list(argv), "exit_code": 0, "output_sha256": "a" * 64,
    } for identifier, argv in evidence.COMMANDS]


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


class Sprint23EvidenceTests(unittest.TestCase):
    def test_local_contracts_pass_without_native_or_release_overclaim(self) -> None:
        value = report()
        self.assertEqual(evidence.validate_report(value, verify_current=False), [])
        self.assertEqual(value["summary"]["sprint_status"], "BLOCKED")
        self.assertTrue(value["implemented_contracts"]["exact_profile_projection"])
        self.assertFalse(value["implemented_contracts"]["production_model_inference"])

    def test_production_native_accessibility_review_and_release_overclaims_fail(self) -> None:
        mutations = (
            lambda value: value["summary"].update({"sprint_status": "PASS"}),
            lambda value: value["summary"].update({"release_approval": True}),
            lambda value: value["verification_evidence"].update(
                {"signed_catalog_production_integration": False}),
            lambda value: value["verification_evidence"].update(
                {"production_model_invocation": True}),
            lambda value: value["verification_evidence"].update(
                {"installed_native_vscode_workflow": True}),
            lambda value: value["verification_evidence"].update(
                {"linux_native_accessibility": True}),
            lambda value: value["verification_evidence"].update({"independent_review": True}),
            lambda value: value["blockers"].pop(),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(evidence.validate_report(changed, verify_current=False))

    def test_fallback_command_environment_and_source_mutations_fail(self) -> None:
        mutations = (
            lambda value: value["implemented_contracts"].update(
                {"automatic_model_substitution": True}),
            lambda value: value["implemented_contracts"].update(
                {"model_token_streaming": True}),
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
