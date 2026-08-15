from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import sprint_45_evidence as evidence


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
    return [{
        "id": "git",
        "name": "git",
        "size": 1,
        "sha256": "b" * 64,
        "root_owned": True,
        "group_or_world_writable": False,
    }]


def report() -> dict[str, object]:
    with (
        patch.object(evidence, "git_file", return_value=b"source"),
        patch.object(evidence, "environment_manifest", return_value={}),
    ):
        return evidence.build_report("c" * 40, commands(), artifacts())


class Sprint45EvidenceTests(unittest.TestCase):
    def validate(self, value: dict[str, object]) -> list[str]:
        with patch.object(evidence, "git_file", return_value=b"source"):
            return evidence.validate_report(value, verify_current=False)

    def test_valid_local_report_remains_blocked_without_release_claim(self) -> None:
        value = report()
        self.assertEqual(self.validate(value), [])
        self.assertTrue(value["summary"]["local_sprint_45_contract_passed"])
        self.assertEqual(value["summary"]["sprint_status"], "BLOCKED")
        self.assertFalse(value["summary"]["release_approval"])

    def test_every_integration_platform_review_fuzz_and_release_overclaim_fails(self) -> None:
        fields = (
            "upstream_sprint_44_closed",
            "production_chat_coding_coordinator_active",
            "production_language_service_sandbox_active",
            "controlled_package_scaffold_application_active",
            "trusted_installed_parent_execution_complete",
            "cross_platform_acceptance_passed",
            "trusted_package_execution_complete",
            "independent_review_present",
            "manual_fuzzing_complete",
            "release_approval",
        )
        for field in fields:
            changed = copy.deepcopy(report())
            changed["summary"][field] = True
            self.assertTrue(self.validate(changed), field)

    def test_command_skip_source_artifact_security_and_blocker_drift_fails(self) -> None:
        mutations = (
            lambda value: value["commands"][0].update({"exit_code": 1}),
            lambda value: value["commands"][1].update({"blocking_skip_count": 1}),
            lambda value: value["commands"].pop(),
            lambda value: value["native_fixture_artifacts"][0].update({"root_owned": False}),
            lambda value: value["native_fixture_artifacts"].pop(),
            lambda value: value["source_sha256"].pop(next(iter(value["source_sha256"]))),
            lambda value: value["security_requirement_ids"].pop(),
            lambda value: value["blockers"].pop(),
            lambda value: value["implemented_contracts"].update({
                "production_language_service_sandbox": True,
            }),
            lambda value: value["verification_evidence"].update({
                "unauthorized_mutation_acceptance_count": 1,
            }),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(self.validate(changed))

    def test_report_shape_contains_no_source_credentials_or_command_output(self) -> None:
        encoded = str(report())
        for prohibited in (
            "source_content",
            "command_output",
            "remote_url",
            "credential_value",
            "secret_canary_value",
            "repository_path",
        ):
            self.assertNotIn(prohibited, encoded)


if __name__ == "__main__":
    unittest.main()

