from __future__ import annotations

import copy
import json
import unittest
from unittest.mock import patch

from scripts import sprint_37_evidence as evidence


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
    return json.dumps({
        "schema_version": 1,
        "record_type": "sprint_37_protected_path_collision_corpus",
        "cases": [{"id": f"case-{index}"} for index in range(18)],
    }).encode()


def report() -> dict[str, object]:
    def git_file(_revision: str, path: str) -> bytes:
        if path.endswith("sprint-37-protected-path-corpus.json"):
            return corpus()
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


class Sprint37EvidenceTests(unittest.TestCase):
    def validate(self, value: dict[str, object]) -> list[str]:
        def git_file(_revision: str, path: str) -> bytes:
            if path.endswith("sprint-37-protected-path-corpus.json"):
                return corpus()
            return b"source"

        with patch.object(evidence, "git_file", side_effect=git_file):
            return evidence.validate_report(value, verify_current=False)

    def test_local_and_fedora_contracts_pass_without_sprint_overclaim(self) -> None:
        value = report()
        self.assertEqual(self.validate(value), [])
        self.assertEqual(value["summary"]["sprint_status"], "BLOCKED")
        self.assertTrue(value["implemented_contracts"]["linux_native_driver"])
        self.assertTrue(value["verification_evidence"]["s_030_ut01_complete"])
        self.assertTrue(value["verification_evidence"]["exact_resource_boundary_matrix"])
        self.assertTrue(value["verification_evidence"]["missing_and_wrong_type_matrix"])
        self.assertTrue(value["verification_evidence"]["closed_metadata_mode_matrix"])
        self.assertFalse(value["summary"]["cross_platform_evidence_passed"])

    def test_dependency_platform_race_crash_worker_review_fuzz_and_release_overclaims_fail(self) -> None:
        mutations = (
            lambda value: value["summary"].update({"sprint_status": "PASS"}),
            lambda value: value["summary"].update({"upstream_dependency_passed": True}),
            lambda value: value["summary"].update({"cross_platform_evidence_passed": True}),
            lambda value: value["summary"].update({
                "native_race_and_crash_evidence_passed": True
            }),
            lambda value: value["summary"].update({"isolated_write_worker_proven": True}),
            lambda value: value["summary"].update({"independent_review_passed": True}),
            lambda value: value["summary"].update({"manual_fuzzing_complete": True}),
            lambda value: value["summary"].update({"network_access_enabled": True}),
            lambda value: value["summary"].update({"release_approval": True}),
            lambda value: value["verification_evidence"].update({
                "complete_crash_durability_matrix": True
            }),
            lambda value: value["implemented_contracts"].update({
                "macos_native_driver": True
            }),
            lambda value: value["blockers"].pop(),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(self.validate(changed))

    def test_command_skip_security_environment_source_and_corpus_mutations_fail(self) -> None:
        mutations = (
            lambda value: value["commands"][0].update({"exit_code": 1}),
            lambda value: value["commands"][0].update({"blocking_skip_count": 1}),
            lambda value: value["commands"].pop(),
            lambda value: value["security_requirement_ids"].pop(),
            lambda value: value["environment"].pop("rustc"),
            lambda value: value["source_sha256"].pop(next(iter(value["source_sha256"]))),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(self.validate(changed))


if __name__ == "__main__":
    unittest.main()
