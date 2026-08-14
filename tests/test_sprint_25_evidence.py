from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import sprint_25_evidence as evidence


def commands() -> list[dict[str, object]]:
    return [
        {
            "id": identifier,
            "argv": list(argv),
            "exit_code": 0,
            "output_sha256": "a" * 64,
        }
        for identifier, argv in evidence.COMMANDS
    ]


def report() -> dict[str, object]:
    with (
        patch.object(evidence, "git_file", return_value=b"source"),
        patch.object(
            evidence,
            "environment_manifest",
            return_value={
                "system": "Linux",
                "release": "fixture",
                "machine": "x86_64",
                "python": "fixture",
                "rustc": "fixture",
                "cargo": "fixture",
                "node": "fixture",
                "npm": "fixture",
            },
        ),
    ):
        return evidence.build_report("b" * 40, commands())


class Sprint25EvidenceTests(unittest.TestCase):
    def test_local_readiness_passes_without_release_overclaim(self) -> None:
        value = report()
        self.assertEqual(evidence.validate_report(value, verify_current=False), [])
        self.assertEqual(value["summary"]["sprint_status"], "BLOCKED")
        self.assertTrue(value["implemented_readiness"]["detached_signature_mechanics"])
        self.assertFalse(value["implemented_readiness"]["signed_release_packages"])

    def test_release_native_tabletop_and_review_overclaims_fail(self) -> None:
        mutations = (
            lambda value: value["summary"].update({"sprint_status": "PASS"}),
            lambda value: value["summary"].update({"release_approval": True}),
            lambda value: value["summary"].update({"supported_release": True}),
            lambda value: value["verification_evidence"].update(
                {"production_signer_and_trust_root": True}
            ),
            lambda value: value["verification_evidence"].update(
                {"native_cross_platform_workflow": True}
            ),
            lambda value: value["verification_evidence"].update(
                {"independent_incident_tabletop": True}
            ),
            lambda value: value["verification_evidence"].update(
                {"independent_release_review": True}
            ),
            lambda value: value["blockers"].pop(),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(evidence.validate_report(changed, verify_current=False))

    def test_command_requirement_environment_and_source_mutations_fail(self) -> None:
        mutations = (
            lambda value: value["commands"][0].update({"exit_code": 1}),
            lambda value: value["commands"].pop(),
            lambda value: value["requirement_families"].pop(),
            lambda value: value["environment"].pop("rustc"),
            lambda value: value["source_sha256"].pop(
                next(iter(value["source_sha256"]))
            ),
            lambda value: value["implemented_readiness"].update(
                {"supported_v0_1_release": True}
            ),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(evidence.validate_report(changed, verify_current=False))


if __name__ == "__main__":
    unittest.main()
