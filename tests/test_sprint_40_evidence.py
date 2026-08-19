from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import sprint_40_evidence as evidence


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
            "kind": kind,
            "name": name,
            "size": 1,
            "sha256": character * 64,
            "status": "unsigned-local-candidate",
            "retained_in_repository": False,
            "published": False,
            "signed": False,
        }
        for kind, name, character in (
            ("deb", "agentmage_0.3.0_amd64.deb", "a"),
            ("rpm", "agentmage-0.3.0-1.fc44.x86_64.rpm", "b"),
            ("vsix", "agentmage-vscode-0.3.0.vsix", "c"),
        )
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
            "rpmbuild": "fixture",
        }),
    ):
        return evidence.build_report("b" * 40, commands(), artifacts())


class Sprint40EvidenceTests(unittest.TestCase):
    def validate(self, value: dict[str, object]) -> list[str]:
        with patch.object(evidence, "git_file", return_value=b"source"):
            return evidence.validate_report(value, verify_current=False)

    def test_unsigned_candidate_passes_locally_without_release_claim(self) -> None:
        value = report()
        self.assertEqual(self.validate(value), [])
        self.assertTrue(value["summary"]["local_v0_3_candidate_contract_passed"])
        self.assertEqual(value["summary"]["sprint_status"], "BLOCKED")
        self.assertFalse(value["summary"]["write_profile_active"])
        self.assertFalse(value["summary"]["release_approval"])

    def test_every_activation_platform_lifecycle_signing_review_and_release_overclaim_fails(
        self,
    ) -> None:
        fields = (
            "upstream_dependencies_passed",
            "write_profile_active",
            "cross_platform_acceptance_passed",
            "signed_packages_present",
            "independent_release_decision_present",
            "manual_fuzzing_complete",
            "network_access_enabled",
            "g_v0_3_closed",
            "release_approval",
        )
        for field in fields:
            changed = copy.deepcopy(report())
            changed["summary"][field] = True
            self.assertTrue(self.validate(changed), field)

        changed = copy.deepcopy(report())
        changed["summary"]["lifecycle_acceptance_passed"] = False
        self.assertTrue(self.validate(changed), "lifecycle_acceptance_passed")

    def test_command_skip_package_source_security_and_blocker_mutations_fail(self) -> None:
        mutations = (
            lambda value: value["commands"][0].update({"exit_code": 1}),
            lambda value: value["commands"][3].update({"blocking_skip_count": 1}),
            lambda value: value["commands"].pop(),
            lambda value: value["unsigned_candidate_artifacts"][0].update({"signed": True}),
            lambda value: value["unsigned_candidate_artifacts"].pop(),
            lambda value: value["source_sha256"].pop(next(iter(value["source_sha256"]))),
            lambda value: value["security_requirement_ids"].pop(),
            lambda value: value["blockers"].pop(),
            lambda value: value["implemented_contracts"].update({"g_v0_3_closed": True}),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(self.validate(changed))


if __name__ == "__main__":
    unittest.main()
