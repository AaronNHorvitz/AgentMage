from __future__ import annotations

import copy
import unittest
from pathlib import Path

from scripts.clean_build_evidence import (
    EXPECTED_COMMANDS,
    EXPECTED_CONTROLS,
    build_report,
    check_report,
    normalized_sha256_id,
    validate_policy,
    validate_report,
)
from scripts.clean_standard_build import input_tree_sha256, read_json


class CleanBuildEvidenceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.policy = read_json(Path("architecture/clean-build-policy.json"))
        source = {
            "input_sha256": input_tree_sha256(Path.cwd(), self.policy),
            "revision": "a" * 40,
        }
        self.runs = {}
        for platform_id, platform in self.policy["linux_platforms"].items():
            self.runs[platform_id] = {
                "status": "pass",
                "platform": {
                    "architecture": "x86_64",
                    "id": platform_id,
                    "os_id": platform["os_id"],
                    "pretty_name": "synthetic fixture",
                    "version_id": platform["version_id"],
                },
                "execution": {
                    "effective_gid": 10001,
                    "effective_uid": 10001,
                    "privileged": False,
                    "source_archive": "read-only",
                    "user_class": "standard-unprivileged",
                    "writable_storage": "fresh-temporary-filesystem",
                },
                "source": source,
                "toolchains": [],
                "commands": [
                    {"id": command, "status": "pass"}
                    for command in EXPECTED_COMMANDS
                ],
                "checks": {
                    "all_commands_passed": True,
                    "ambient_dependency_detected": False,
                    "clean_home": True,
                    "clean_npm_cache": True,
                    "clean_source_archive": True,
                    "clean_target": True,
                    "supply_chain_unchanged": True,
                },
                "macos_support_claim": "none",
                "base_image": platform["base_image"],
                "container_image_id": "sha256:" + "b" * 64,
                "container_controls": EXPECTED_CONTROLS,
            }
        self.report = build_report(self.runs, "a" * 40)

    def test_canonical_policy_is_valid(self) -> None:
        self.assertEqual(validate_policy(self.policy), [])

    def test_only_locked_dependency_bootstraps_may_use_network(self) -> None:
        bootstrap = [
            item for item in self.policy["commands"] if item["network"] == "bootstrap-only"
        ]
        self.assertEqual(
            [(item["id"], item["argv"]) for item in bootstrap],
            [
                (
                    "npm-clean-install",
                    ["npm", "ci", "--ignore-scripts", "--no-audit", "--no-fund"],
                ),
                ("cargo-fetch", ["cargo", "fetch", "--locked"]),
            ],
        )

    def test_checked_in_report_is_current(self) -> None:
        self.assertEqual(check_report(), [])

    def test_complete_linux_fixture_retains_blocked_macos(self) -> None:
        self.assertEqual(validate_report(self.report), [])
        self.assertEqual(self.report["status"], "blocked-macos")
        self.assertFalse(self.report["summary"]["cross_platform_task_complete"])

    def test_root_execution_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.report)
        mutated["platform_runs"]["fedora-x86_64"]["execution"][
            "effective_uid"
        ] = 0
        self.assertTrue(validate_report(mutated))

    def test_missing_command_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.report)
        mutated["platform_runs"]["ubuntu-x86_64"]["commands"].pop()
        self.assertTrue(validate_report(mutated))

    def test_ambient_dependency_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.report)
        mutated["platform_runs"]["fedora-x86_64"]["checks"][
            "ambient_dependency_detected"
        ] = True
        self.assertTrue(validate_report(mutated))

    def test_base_image_substitution_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.report)
        mutated["platform_runs"]["ubuntu-x86_64"]["base_image"] = (
            "docker.io/library/ubuntu:latest"
        )
        self.assertTrue(validate_report(mutated))

    def test_changed_input_hash_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.report)
        mutated["source"]["input_sha256"] = "0" * 64
        self.assertTrue(validate_report(mutated))

    def test_macos_promotion_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.report)
        mutated["macos"]["status"] = "pass"
        mutated["summary"]["cross_platform_task_complete"] = True
        self.assertTrue(validate_report(mutated))

    def test_bare_podman_image_id_is_normalized(self) -> None:
        value = "c" * 64
        self.assertEqual(normalized_sha256_id(value), f"sha256:{value}")

    def test_non_sha256_image_id_is_rejected(self) -> None:
        with self.assertRaises(OSError):
            normalized_sha256_id("latest")


if __name__ == "__main__":
    unittest.main()
