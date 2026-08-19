from __future__ import annotations

import copy
import json
import unittest
from unittest.mock import patch

from scripts import sprint_39_evidence as evidence


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


def corpus() -> bytes:
    cases = []
    for category, count in evidence.CORPUS_CLASSES.items():
        cases.extend(
            {
                "id": f"{category}-{index}",
                "class": category,
                "expected_instruction": "fixture",
                "repeat_completed_write": False,
            }
            for index in range(count)
        )
    return json.dumps({
        "schema_version": 1,
        "record_type": "sprint_39_recovery_corpus",
        "privacy": "public-synthetic-only",
        "cases": cases,
    }).encode()


def audit() -> bytes:
    return json.dumps({
        "redacted_fields": 1,
        "report": "File: [REDACTED]",
    }).encode()


def report() -> dict[str, object]:
    def git_file(_revision: str, path: str) -> bytes:
        if path == evidence.CORPUS_PATH:
            return corpus()
        if path == evidence.AUDIT_PATH:
            return audit()
        return b"source"

    with (
        patch.object(evidence, "git_file", side_effect=git_file),
        patch.object(evidence, "environment_manifest", return_value={
            "system": "Linux",
            "release": "fixture",
            "machine": "x86_64",
            "python": "fixture",
            "rustc": "fixture",
            "cargo": "fixture",
            "node": "fixture",
            "npm": "fixture",
        }),
    ):
        return evidence.build_report("b" * 40, commands())


class Sprint39EvidenceTests(unittest.TestCase):
    def validate(self, value: dict[str, object]) -> list[str]:
        def git_file(_revision: str, path: str) -> bytes:
            if path == evidence.CORPUS_PATH:
                return corpus()
            if path == evidence.AUDIT_PATH:
                return audit()
            return b"source"

        with patch.object(evidence, "git_file", side_effect=git_file):
            return evidence.validate_report(value, verify_current=False)

    def test_local_contracts_pass_without_sprint_overclaim(self) -> None:
        value = report()
        self.assertEqual(self.validate(value), [])
        self.assertTrue(value["summary"]["local_write_recovery_contract_passed"])
        self.assertEqual(value["summary"]["sprint_status"], "BLOCKED")
        self.assertFalse(value["implemented_contracts"]["completed_write_replay_allowed"])
        self.assertTrue(value["summary"]["native_end_to_end_passed"])
        self.assertTrue(value["summary"]["native_derived_index_recovery_passed"])
        self.assertTrue(value["summary"]["native_write_producer_privacy_passed"])

    def test_dependency_native_roots_launcher_platform_review_fuzz_and_release_overclaims_fail(
        self,
    ) -> None:
        mutations = (
            lambda value: value["summary"].update({"sprint_status": "PASS"}),
            lambda value: value["summary"].update({"upstream_dependency_passed": True}),
            lambda value: value["summary"].update({"native_end_to_end_passed": False}),
            lambda value: value["summary"].update({
                "native_derived_index_recovery_passed": False
            }),
            lambda value: value["summary"].update({
                "native_write_producer_privacy_passed": False
            }),
            lambda value: value["summary"].update({"native_crash_concurrency_passed": True}),
            lambda value: value["summary"].update({"all_runtime_roots_scanned": True}),
            lambda value: value["summary"].update({"trusted_launcher_environment_passed": True}),
            lambda value: value["summary"].update({"cross_platform_evidence_passed": True}),
            lambda value: value["summary"].update({"independent_review_passed": True}),
            lambda value: value["summary"].update({"manual_fuzzing_complete": True}),
            lambda value: value["summary"].update({"network_access_enabled": True}),
            lambda value: value["summary"].update({"release_approval": True}),
            lambda value: value["verification_evidence"].update({
                "native_end_to_end_recovery": False
            }),
            lambda value: value["verification_evidence"].update({
                "native_derived_index_recovery": False
            }),
            lambda value: value["verification_evidence"].update({
                "native_write_producer_privacy": False
            }),
            lambda value: value["implemented_contracts"].update({
                "native_end_to_end_recovery_wiring": False
            }),
            lambda value: value["implemented_contracts"].update({
                "native_derived_index_checkpoint_publication": False
            }),
            lambda value: value["implemented_contracts"].update({
                "native_write_producer_privacy_gate": False
            }),
            lambda value: value["blockers"].pop(),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(self.validate(changed))

    def test_command_skip_security_source_corpus_audit_and_replay_mutations_fail(self) -> None:
        mutations = (
            lambda value: value["commands"][0].update({"exit_code": 1}),
            lambda value: value["commands"][0].update({"blocking_skip_count": 1}),
            lambda value: value["commands"].pop(),
            lambda value: value["security_requirement_ids"].pop(),
            lambda value: value["source_sha256"].pop(next(iter(value["source_sha256"]))),
            lambda value: value["implemented_contracts"].update({
                "completed_write_replay_allowed": True
            }),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(self.validate(changed))


if __name__ == "__main__":
    unittest.main()
