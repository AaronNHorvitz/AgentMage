from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import sprint_41_evidence as evidence


def commands() -> list[dict[str, object]]:
    return [
        {
            "id": identifier,
            "argv": list(argv),
            "exit_code": 0,
            "output_sha256": "a" * 64,
            "blocking_skip_count": 0 if identifier in evidence.FOCUSED_COMMANDS else None,
        }
        for identifier, argv in evidence.COMMANDS
    ]


def artifacts() -> list[dict[str, object]]:
    return [
        {
            "id": identifier,
            "name": path.name,
            "size": 1,
            "sha256": character * 64,
            "root_owned": True,
            "group_or_world_writable": False,
        }
        for (identifier, path), character in zip(evidence.ARTIFACTS, "abcde", strict=True)
    ]


def report() -> dict[str, object]:
    with (
        patch.object(evidence, "git_file", return_value=b"source"),
        patch.object(evidence, "environment_manifest", return_value={
            "system": "Linux",
            "release": "fixture",
            "machine": "x86_64",
            "python": "fixture",
            "rustc": "fixture",
            "cargo": "fixture",
            "node": "fixture",
            "npm": "fixture",
            "systemd": "fixture",
            "bubblewrap": "fixture",
        }),
    ):
        return evidence.build_report("b" * 40, commands(), artifacts())


class Sprint41EvidenceTests(unittest.TestCase):
    def validate(self, value: dict[str, object]) -> list[str]:
        with patch.object(evidence, "git_file", return_value=b"source"):
            return evidence.validate_report(value, verify_current=False)

    def test_local_command_contract_passes_without_activation_or_release_claim(self) -> None:
        value = report()
        self.assertEqual(self.validate(value), [])
        self.assertTrue(value["summary"]["local_sprint_41_contract_passed"])
        self.assertEqual(value["summary"]["sprint_status"], "BLOCKED")
        self.assertFalse(value["summary"]["command_profile_active"])
        self.assertFalse(value["summary"]["generic_shell_present"])
        self.assertFalse(value["summary"]["release_approval"])

    def test_every_platform_resource_review_fuzz_and_release_overclaim_fails(self) -> None:
        fields = (
            "upstream_g_v0_3_closed",
            "command_profile_active",
            "network_access_enabled",
            "cross_platform_acceptance_passed",
            "peak_resource_accounting_complete",
            "hostile_descendant_campaign_complete",
            "trusted_package_execution_complete",
            "independent_review_present",
            "manual_fuzzing_complete",
            "release_approval",
        )
        for field in fields:
            changed = copy.deepcopy(report())
            changed["summary"][field] = True
            self.assertTrue(self.validate(changed), field)

    def test_command_skip_artifact_source_security_and_blocker_mutations_fail(self) -> None:
        mutations = (
            lambda value: value["commands"][0].update({"exit_code": 1}),
            lambda value: value["commands"][3].update({"blocking_skip_count": 1}),
            lambda value: value["commands"].pop(),
            lambda value: value["native_fixture_artifacts"][0].update({"root_owned": False}),
            lambda value: value["native_fixture_artifacts"].pop(),
            lambda value: value["source_sha256"].pop(next(iter(value["source_sha256"]))),
            lambda value: value["security_requirement_ids"].pop(),
            lambda value: value["blockers"].pop(),
            lambda value: value["implemented_contracts"].update({
                "production_command_profile_registered": True,
            }),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(self.validate(changed))


if __name__ == "__main__":
    unittest.main()
