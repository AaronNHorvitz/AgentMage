from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import sprint_59_evidence as evidence


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


def report() -> dict[str, object]:
    with (
        patch.object(evidence, "git_file", return_value=b"source"),
        patch.object(evidence, "environment_manifest", return_value={}),
    ):
        return evidence.build_report("c" * 40, commands())


class Sprint59EvidenceTests(unittest.TestCase):
    def validate(self, value: dict[str, object]) -> list[str]:
        with patch.object(evidence, "git_file", return_value=b"source"):
            return evidence.validate_report(value, verify_ancestry=False)

    def test_valid_local_report_preserves_blocked_sprint_truth(self) -> None:
        value = report()
        self.assertEqual(self.validate(value), [])
        self.assertTrue(value["summary"]["local_sprint_59_contract_passed"])
        self.assertFalse(value["summary"]["external_effects_enabled"])
        self.assertFalse(value["verification_evidence"]["sprint_gate_closed"])
        self.assertFalse(value["verification_evidence"]["renderer_admitted"])
        self.assertEqual(value["verification_evidence"]["native_render_platform_count"], 0)
        self.assertEqual(value["summary"]["sprint_status"], "BLOCKED")

    def test_every_product_platform_manual_and_release_overclaim_fails(self) -> None:
        for field in (
            "upstream_sprint_58_closed",
            "word_workflow_integrated",
            "external_effects_enabled",
            "cross_platform_render_acceptance_passed",
            "trusted_package_execution_complete",
            "independent_review_present",
            "manual_fuzzing_complete",
            "release_approval",
        ):
            changed = copy.deepcopy(report())
            changed["summary"][field] = True
            self.assertTrue(self.validate(changed), field)

    def test_command_source_security_blocker_and_effect_drift_fails(self) -> None:
        mutations = (
            lambda value: value["commands"][0].update({"exit_code": 1}),
            lambda value: value["commands"][1].update({"blocking_skip_count": 1}),
            lambda value: value["commands"].pop(),
            lambda value: value["source_sha256"].pop(next(iter(value["source_sha256"]))),
            lambda value: value["security_requirement_ids"].pop(),
            lambda value: value["blockers"].pop(),
            lambda value: value["verification_evidence"].update({"renderer_admitted": True}),
            lambda value: value["verification_evidence"].update(
                {"native_render_platform_count": 1}
            ),
            lambda value: value["verification_evidence"].update(
                {"accepted_network_effect_count": 1}
            ),
            lambda value: value["verification_evidence"].update(
                {"accepted_execution_effect_count": 1}
            ),
            lambda value: value["verification_evidence"].update(
                {"accepted_filesystem_effect_count": 1}
            ),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(self.validate(changed))

    def test_report_contains_no_sensitive_raw_or_prompt_material(self) -> None:
        encoded = str(report()).lower()
        for prohibited in (
            "credential_value",
            "secret_value",
            "private_key",
            "raw_output",
            "raw_result",
            "prompt_text",
            "raw_document",
            "remote_url",
            "repository_path",
            "access_token",
        ):
            self.assertNotIn(prohibited, encoded)


if __name__ == "__main__":
    unittest.main()
