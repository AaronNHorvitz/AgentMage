from __future__ import annotations

import copy
import subprocess
import unittest
from types import SimpleNamespace
from unittest.mock import patch

from scripts import sprint_48_evidence as evidence


def commands() -> list[dict[str, object]]:
    return [
        {
            "id": identifier,
            "argv": list(argv),
            "exit_code": 0,
            "output_sha256": "a" * 64,
            "blocking_skip_count": (
                0 if identifier in evidence.FOCUSED_COMMANDS else None
            ),
        }
        for identifier, argv in evidence.COMMANDS
    ]


def report() -> dict[str, object]:
    with (
        patch.object(evidence, "git_file", return_value=b"source"),
        patch.object(evidence, "environment_manifest", return_value={}),
    ):
        return evidence.build_report("c" * 40, commands())


class Sprint48EvidenceTests(unittest.TestCase):
    def test_only_full_documentation_has_the_longer_finite_collection_deadline(self) -> None:
        completed = subprocess.CompletedProcess(
            ["fixture"], 0,
            stdout=b"test result: ok. 1 passed; 0 failed; 0 ignored;",
            stderr=b"",
        )
        with (
            patch.object(
                evidence.shutil, "which", side_effect=lambda name: f"/fixture/{name}"
            ),
            patch.object(evidence.subprocess, "run", return_value=completed) as run,
        ):
            records = evidence.run_commands()
        self.assertEqual(len(records), len(evidence.COMMANDS))
        self.assertEqual(run.call_count, len(evidence.COMMANDS))
        for (identifier, argv), call in zip(
            evidence.COMMANDS, run.call_args_list, strict=True
        ):
            self.assertEqual(call.args[0], (f"/fixture/{argv[0]}", *argv[1:]))
            self.assertEqual(
                call.kwargs["timeout"],
                2700 if identifier == "documentation-gate" else 1800,
            )
            self.assertTrue(call.kwargs["capture_output"])

    def test_collection_timeout_cannot_write_a_passing_report(self) -> None:
        with (
            patch.object(
                evidence, "parse_args",
                return_value=SimpleNamespace(write=True, source_revision="c" * 40),
            ),
            patch.object(
                evidence, "run_commands",
                side_effect=subprocess.TimeoutExpired(("npm", "run", "docs:check"), 2700),
            ),
            patch.object(evidence, "OUTPUT") as output,
        ):
            with self.assertRaises(subprocess.TimeoutExpired):
                evidence.main()
        output.write_text.assert_not_called()

    def validate(self, value: dict[str, object]) -> list[str]:
        with patch.object(evidence, "git_file", return_value=b"source"):
            return evidence.validate_report(value, verify_ancestry=False)

    def test_valid_local_report_remains_blocked_without_release_claim(self) -> None:
        value = report()
        self.assertEqual(self.validate(value), [])
        self.assertTrue(value["summary"]["local_sprint_48_contract_passed"])
        self.assertEqual(value["summary"]["sprint_status"], "BLOCKED")
        self.assertFalse(value["summary"]["release_approval"])

    def test_every_product_platform_and_manual_overclaim_fails(self) -> None:
        fields = (
            "upstream_sprint_47_closed",
            "authenticated_product_transport_active",
            "production_command_coordinators_active",
            "native_disconnect_process_tree_campaign_complete",
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

    def test_command_source_security_and_blocker_drift_fails(self) -> None:
        mutations = (
            lambda value: value["commands"][0].update({"exit_code": 1}),
            lambda value: value["commands"][1].update(
                {"blocking_skip_count": 1}
            ),
            lambda value: value["commands"][2]["argv"].append("--ignored"),
            lambda value: value["commands"].pop(),
            lambda value: value["source_sha256"].pop(
                next(iter(value["source_sha256"]))
            ),
            lambda value: value["security_requirement_ids"].pop(),
            lambda value: value["blockers"].pop(),
            lambda value: value["verification_evidence"].update(
                {"unauthorized_effect_acceptance_count": 1}
            ),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(self.validate(changed))

    def test_report_shape_contains_no_sensitive_or_raw_material(self) -> None:
        encoded = str(report()).lower()
        for prohibited in (
            "credential_value",
            "secret_value",
            "private_key",
            "raw_output",
            "stdout_content",
            "stderr_content",
            "remote_url",
            "repository_path",
            "socket_path",
            "prompt_text",
            "access_token",
        ):
            self.assertNotIn(prohibited, encoded)


if __name__ == "__main__":
    unittest.main()
