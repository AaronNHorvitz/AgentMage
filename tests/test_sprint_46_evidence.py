from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import sprint_46_evidence as evidence


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
        "id": "cargo-toolchain-dispatcher",
        "name": "rustup",
        "size": 1,
        "sha256": "b" * 64,
        "owner_is_current_user": True,
        "group_or_world_writable": False,
    }]


def report() -> dict[str, object]:
    with (
        patch.object(evidence, "git_file", return_value=b"source"),
        patch.object(evidence, "environment_manifest", return_value={}),
    ):
        return evidence.build_report("c" * 40, commands(), artifacts())


class Sprint46EvidenceTests(unittest.TestCase):
    def validate(self, value: dict[str, object]) -> list[str]:
        with patch.object(evidence, "git_file", return_value=b"source"):
            return evidence.validate_report(value, verify_current=False)

    def test_valid_local_report_remains_blocked_without_release_claim(self) -> None:
        value = report()
        self.assertEqual(self.validate(value), [])
        self.assertTrue(value["summary"]["local_sprint_46_contract_passed"])
        self.assertEqual(value["summary"]["sprint_status"], "BLOCKED")
        self.assertFalse(value["summary"]["release_approval"])

    def test_every_external_product_and_manual_overclaim_fails(self) -> None:
        fields = (
            "upstream_sprint_45_closed",
            "production_chat_validation_coordinator_active",
            "trusted_installed_parent_execution_complete",
            "native_validation_worker_campaign_complete",
            "protected_raw_log_integration_complete",
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

    def test_command_source_artifact_security_and_blocker_drift_fails(self) -> None:
        mutations = (
            lambda value: value["commands"][0].update({"exit_code": 1}),
            lambda value: value["commands"][1].update({"blocking_skip_count": 1}),
            lambda value: value["commands"][2]["argv"].append("--ignored"),
            lambda value: value["commands"].pop(),
            lambda value: value["native_fixture_artifacts"][0].update({
                "owner_is_current_user": False,
            }),
            lambda value: value["source_sha256"].pop(next(iter(value["source_sha256"]))),
            lambda value: value["security_requirement_ids"].pop(),
            lambda value: value["blockers"].pop(),
            lambda value: value["verification_evidence"].update({
                "unauthorized_command_acceptance_count": 1,
            }),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(self.validate(changed))

    def test_report_shape_contains_no_raw_secret_remote_or_path_material(self) -> None:
        encoded = str(report())
        for prohibited in (
            "credential_value",
            "secret_value",
            "raw_output",
            "stdout_content",
            "stderr_content",
            "remote_url",
            "repository_path",
        ):
            self.assertNotIn(prohibited, encoded)


if __name__ == "__main__":
    unittest.main()
