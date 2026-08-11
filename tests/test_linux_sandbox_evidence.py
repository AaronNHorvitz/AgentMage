"""Tests for the bounded Sprint 9 Linux control evidence."""

from __future__ import annotations

import copy
import unittest

from scripts import linux_sandbox_evidence as evidence


class LinuxSandboxEvidenceTests(unittest.TestCase):
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
            ],
            "test_ids": ["S-009-UT01", "S-009-ST01"],
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
