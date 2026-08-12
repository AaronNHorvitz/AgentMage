"""Tests for bounded Fedora/Ubuntu Linux sandbox attack evidence."""

from __future__ import annotations

import copy
import unittest

from scripts import linux_cross_distribution_sandbox_evidence as evidence


class CrossDistributionSandboxEvidenceTests(unittest.TestCase):
    def valid_report(self) -> dict:
        tests = [{"test": name, "status": "pass"} for name in evidence.LIVE_TESTS]
        return {
            "schema_version": 1,
            "artifact_id": "linux-cross-distribution-sandbox-attacks",
            "task_ids": ["9.1.3.2"],
            "test_ids": ["S-009-ST01"],
            "status": "pass-bounded-cross-distribution",
            "source_revision": "a" * 40,
            "sources": [
                {"path": path, "sha256": "b" * 64}
                for path in evidence.SOURCE_PATHS
            ],
            "controls": copy.deepcopy(evidence.EXPECTED_CONTROLS),
            "platforms": {
                "fedora-44-x86_64": {
                    "architecture": "x86_64",
                    "cgroup_filesystem": "cgroup2",
                    "distribution": "fedora",
                    "execution_context": "native-standard-user",
                    "native_platform_claim": True,
                    "tests": copy.deepcopy(tests),
                    "uid": 1000,
                    "version": "44",
                },
                "ubuntu-26.04-x86_64": {
                    "architecture": "x86_64",
                    "base_image": evidence.UBUNTU_BASE_IMAGE,
                    "binary_sha256": "c" * 64,
                    "build_image_id": "sha256:" + "f" * 64,
                    "cgroup_filesystem": "cgroup2",
                    "container_controls": {
                        "cgroup_namespace": "private",
                        "memory_limit_bytes": 2 * 1024 * 1024 * 1024,
                        "network": "none",
                        "outer_privileged_flag": True,
                        "pids_limit": 512,
                        "podman_rootless": True,
                    },
                    "container_image_id": "sha256:" + "d" * 64,
                    "distribution": "ubuntu",
                    "execution_context": "standard-user-in-rootless-privileged-test-envelope",
                    "native_platform_claim": False,
                    "tests": copy.deepcopy(tests),
                    "trusted_tools": [
                        {
                            "canonical_path_class": "root-owned-system-path",
                            "group_or_world_writable": False,
                            "id": tool,
                            "owner_uid": 0,
                            "sha256": "e" * 64,
                            "version": "test-version",
                        }
                        for tool in ("bubblewrap", "systemd-run")
                    ],
                    "uid": 10001,
                    "version": "26.04",
                },
            },
            "attack_results": [
                {
                    "attack": attack,
                    "fedora": "pass-zero-escape",
                    "test": test,
                    "ubuntu": "pass-zero-escape-in-declared-envelope",
                }
                for attack, test in evidence.ATTACK_TESTS.items()
            ],
            "summary": {
                "attack_class_count": len(evidence.ATTACK_TESTS),
                "cross_distribution_zero_escape_within_declared_boundaries": True,
                "live_test_count_per_platform": len(evidence.LIVE_TESTS),
                "native_ubuntu_isolation_verified": False,
            },
            "private_values_present": False,
            "macos_evidence_substituted": False,
            "release_claim": "none",
            "limitations": list(evidence.LIMITATIONS),
        }

    def test_exact_bounded_report_is_valid(self) -> None:
        self.assertEqual(evidence.validate_report(self.valid_report()), [])

    def test_native_ubuntu_or_unprivileged_envelope_overclaim_fails(self) -> None:
        report = self.valid_report()
        ubuntu = report["platforms"]["ubuntu-26.04-x86_64"]
        ubuntu["native_platform_claim"] = True
        ubuntu["container_controls"]["outer_privileged_flag"] = False
        self.assertIn(
            "bounded Ubuntu sandbox evidence is incomplete or overclaimed",
            evidence.validate_report(report),
        )

    def test_missing_test_or_attack_failure_fails(self) -> None:
        missing = self.valid_report()
        missing["platforms"]["fedora-44-x86_64"]["tests"].pop()
        self.assertIn(
            "native Fedora sandbox evidence is incomplete",
            evidence.validate_report(missing),
        )
        failed = self.valid_report()
        failed["attack_results"][0]["ubuntu"] = "escape"
        self.assertIn(
            "cross-distribution attack closure changed",
            evidence.validate_report(failed),
        )

    def test_private_release_or_native_summary_claim_fails(self) -> None:
        for field, replacement in (
            ("private_values_present", True),
            ("release_claim", "supported"),
            ("macos_evidence_substituted", True),
        ):
            with self.subTest(field=field):
                report = self.valid_report()
                report[field] = replacement
                self.assertIn(
                    "cross-distribution sandbox report made an unsupported claim",
                    evidence.validate_report(report),
                )
        summary = self.valid_report()
        summary["summary"]["native_ubuntu_isolation_verified"] = True
        self.assertIn(
            "cross-distribution sandbox summary changed or overclaimed",
            evidence.validate_report(summary),
        )

    def test_test_output_parser_requires_exact_live_closure(self) -> None:
        output = "\n".join(
            f"test sandbox::tests::{name} ... ok" for name in evidence.LIVE_TESTS
        )
        self.assertEqual(
            evidence.parse_test_output(output),
            [{"test": name, "status": "pass"} for name in evidence.LIVE_TESTS],
        )
        with self.assertRaises(evidence.CrossDistributionSandboxError):
            evidence.parse_test_output(output.rsplit("\n", 1)[0])


if __name__ == "__main__":
    unittest.main()
