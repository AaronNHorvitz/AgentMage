from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import sprint_31_evidence as evidence


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


class Sprint31EvidenceTests(unittest.TestCase):
    def test_local_contract_passes_without_file_crypto_or_promotion_overclaim(self) -> None:
        value = report()
        self.assertEqual(evidence.validate_report(value, verify_current=False), [])
        self.assertEqual(value["summary"]["sprint_status"], "BLOCKED")
        self.assertTrue(value["implemented_contracts"]["source_backed_candidate_policy"])
        self.assertFalse(value["summary"]["automatic_memory_promotion"])

    def test_file_crypto_recovery_promotion_and_release_overclaims_fail(self) -> None:
        mutations = (
            lambda value: value["summary"].update({"sprint_status": "PASS"}),
            lambda value: value["summary"].update({"durable_file_write_enabled": True}),
            lambda value: value["summary"].update({"encrypted_export_import_available": True}),
            lambda value: value["summary"].update({"automatic_memory_promotion": True}),
            lambda value: value["summary"].update({"release_approval": True}),
            lambda value: value["verification_evidence"].update(
                {"encrypted_memory_export_import": True}
            ),
            lambda value: value["verification_evidence"].update(
                {"protected_file_recovery_and_migration": True}
            ),
            lambda value: value["implemented_contracts"].update(
                {"durable_memory_file_write": True}
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
