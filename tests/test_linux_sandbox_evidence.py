"""Tests for the bounded Sprint 9 Linux control evidence."""

from __future__ import annotations

import copy
import unittest

from scripts import linux_sandbox_evidence as evidence


class LinuxSandboxEvidenceTests(unittest.TestCase):
    def test_current_control_inventory_is_explicit_and_complete(self) -> None:
        self.assertEqual(
            evidence.SANDBOX_STATIC_TESTS,
            (
                "directory_projection_never_contains_excluded_or_nested_workspace_content",
                "file_projection_is_the_approved_preimage_and_cannot_be_modified",
                "limits_and_manifests_fail_closed",
                "policy_compiles_to_nonempty_classic_bpf",
                "stale_held_directory_fails_projection_before_any_worker_process_can_start",
                "stale_held_file_fails_before_any_worker_process_can_start",
            ),
        )
        self.assertEqual(len(evidence.SANDBOX_TESTS), 17)
        self.assertEqual(len(evidence.SANDBOX_LIVE_TESTS), 11)
        self.assertEqual(len(evidence.IPC_TESTS), 8)
        self.assertEqual(len(evidence.SECRET_SERVICE_TESTS), 4)
        self.assertEqual(len(evidence.SECRET_SERVICE_LIVE_TESTS), 3)
        self.assertEqual(len(evidence.STARTUP_CONTROL_IDS), 7)
        self.assertEqual(len(evidence.STARTUP_MAPPING_TESTS), 2)
        self.assertEqual(len(evidence.KERNEL_STARTUP_TESTS), 1)
        self.assertEqual(len(evidence.STARTUP_LIVE_TESTS), 1)
        self.assertEqual(len(evidence.ATTACKS), 16)

    def valid_report(self) -> dict:
        return {
            "schema_version": 1,
            "artifact_id": "linux-ipc-sandbox-control-verification",
            "task_ids": [
                "9.1.1.5",
                "9.1.2.2",
                "9.1.2.3",
                "9.1.3.1",
                "9.1.3.2",
                "9.1.3.3",
            ],
            "test_ids": ["S-009-UT01", "S-009-ST01", "S-009-UT02"],
            "status": "pass-fedora-only",
            "source_revision": "a" * 40,
            "sources": [
                {"path": path, "sha256": "b" * 64} for path in evidence.SOURCE_PATHS
            ],
            "host": {
                "distribution": "fedora",
                "version": "44",
                "architecture": "x86_64",
                "cgroup_filesystem": "cgroup2",
            },
            "trusted_tools": [
                {
                    "id": name,
                    "path_class": "root-owned-usr-bin",
                    "owner_uid": 0,
                    "group_or_world_writable": False,
                    "sha256": "c" * 64,
                    "version": "synthetic",
                }
                for name in evidence.TOOLS
            ],
            "controls": copy.deepcopy(evidence.CONTROLS),
            "attack_coverage": {attack: "pass" for attack in evidence.ATTACKS},
            "sandbox_tests": [
                {"test": name, "status": "pass"} for name in evidence.SANDBOX_TESTS
            ],
            "ipc_tests": [
                {"test": name, "status": "pass"} for name in evidence.IPC_TESTS
            ],
            "secret_service_tests": [
                {"test": name, "status": "pass"}
                for name in evidence.SECRET_SERVICE_TESTS
            ],
            "secret_service_live_tests": [
                {"test": name, "status": "pass"}
                for name in evidence.SECRET_SERVICE_LIVE_TESTS
            ],
            "startup_control_ids": list(evidence.STARTUP_CONTROL_IDS),
            "startup_mutation_statuses": ["unavailable", "invalid"],
            "startup_platform_families": ["fedora", "ubuntu"],
            "startup_mapping_tests": [
                {"test": name, "status": "pass"}
                for name in evidence.STARTUP_MAPPING_TESTS
            ],
            "kernel_startup_tests": [
                {"test": name, "status": "pass"}
                for name in evidence.KERNEL_STARTUP_TESTS
            ],
            "startup_live_tests": [
                {"test": name, "status": "pass"}
                for name in evidence.STARTUP_LIVE_TESTS
            ],
            "platform_status": copy.deepcopy(evidence.PLATFORM_STATUS),
            "private_values_present": False,
            "macos_evidence_substituted": False,
            "release_claim": "none",
            "limitations": list(evidence.LIMITATIONS),
        }

    def test_expected_report_is_valid(self) -> None:
        self.assertEqual(evidence.validate_report(self.valid_report()), [])

    def test_platform_overclaim_fails(self) -> None:
        report = self.valid_report()
        report["platform_status"]["ubuntu_26_04_x86_64"] = "verified"
        self.assertIn(
            "Linux control platform status changed",
            evidence.validate_report(report),
        )

    def test_missing_attack_or_failed_test_fails(self) -> None:
        missing = self.valid_report()
        missing["attack_coverage"].pop("ambient_home")
        self.assertIn(
            "Linux attack coverage is incomplete",
            evidence.validate_report(missing),
        )
        failed = copy.deepcopy(self.valid_report())
        failed["sandbox_tests"][0]["status"] = "fail"
        self.assertIn(
            "sandbox test closure is incomplete",
            evidence.validate_report(failed),
        )

    def test_every_test_family_fails_closed_on_omission(self) -> None:
        families = (
            ("ipc_tests", "IPC test closure is incomplete"),
            (
                "secret_service_tests",
                "Secret Service unit test closure is incomplete",
            ),
            (
                "secret_service_live_tests",
                "Secret Service live test closure is incomplete",
            ),
            (
                "startup_mapping_tests",
                "Linux startup mapping tests are incomplete",
            ),
            (
                "kernel_startup_tests",
                "kernel startup refusal tests are incomplete",
            ),
            (
                "startup_live_tests",
                "Linux live startup preflight is incomplete",
            ),
        )
        for field, expected_failure in families:
            with self.subTest(field=field):
                report = self.valid_report()
                report[field].pop()
                self.assertIn(expected_failure, evidence.validate_report(report))

    def test_startup_control_status_and_platform_mutations_fail_closed(self) -> None:
        mutations = (
            ("startup_control_ids", ["bubblewrap"], "Linux startup control closure changed"),
            (
                "startup_mutation_statuses",
                ["unavailable"],
                "Linux startup mutation closure changed",
            ),
            (
                "startup_platform_families",
                ["fedora"],
                "Linux startup platform matrix changed",
            ),
        )
        for field, replacement, expected_failure in mutations:
            with self.subTest(field=field):
                report = self.valid_report()
                report[field] = replacement
                self.assertIn(expected_failure, evidence.validate_report(report))

    def test_stale_or_weakened_limitations_fail(self) -> None:
        report = self.valid_report()
        report["limitations"] = ["Ubuntu remains pending."]
        self.assertIn(
            "Linux control limitations are incomplete",
            evidence.validate_report(report),
        )

    def test_private_or_release_claim_fails(self) -> None:
        report = self.valid_report()
        report["private_values_present"] = True
        report["release_claim"] = "supported"
        self.assertIn(
            "Linux control report made a private or unsupported claim",
            evidence.validate_report(report),
        )


if __name__ == "__main__":
    unittest.main()
