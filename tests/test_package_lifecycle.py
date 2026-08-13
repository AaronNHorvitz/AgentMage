"""Tests for the clean Fedora and Ubuntu package lifecycle policy."""

from __future__ import annotations

import copy
import tempfile
import unittest
from pathlib import Path

from scripts import package_lifecycle as lifecycle


class PackageLifecycleTests(unittest.TestCase):
    def test_inference_descriptor_binds_the_inactive_runtime_profile(self) -> None:
        self.assertEqual(
            lifecycle.EXPECTED_INFERENCE_DESCRIPTOR["native_runtime_package"],
            "agentmage-llama-cpp-b10333-cpu-linux-x86_64",
        )
        self.assertRegex(
            lifecycle.EXPECTED_INFERENCE_DESCRIPTOR[
                "native_runtime_profile_sha256"
            ],
            r"^[0-9a-f]{64}$",
        )
        self.assertEqual(
            lifecycle.EXPECTED_INFERENCE_DESCRIPTOR["process_boundary_version"],
            5,
        )
        self.assertFalse(
            lifecycle.EXPECTED_INFERENCE_DESCRIPTOR[
                "docker_compatibility_available"
            ]
        )
        for key in (
            "docker_runtime_profile_sha256",
            "docker_guard_profile_sha256",
            "docker_model_runner_image_digest",
            "docker_model_artifact_digest",
            "docker_preflight_contract_version",
        ):
            if key == "docker_preflight_contract_version":
                self.assertEqual(lifecycle.EXPECTED_INFERENCE_DESCRIPTOR[key], 1)
            else:
                self.assertRegex(
                    lifecycle.EXPECTED_INFERENCE_DESCRIPTOR[key],
                    r"^(?:sha256:)?[0-9a-f]{64}$",
                )

    def valid_lifecycle(self) -> dict:
        checks = {
            "clean_install": True,
            "standard_user_launch": True,
            "component_manifest_exact": True,
            "root_owned_runtime_read_only": True,
            "corrupt_upgrade_refused": True,
            "prior_valid_state_preserved": True,
            "upgrade": True,
            "rollback": True,
            "uninstall": True,
            "reinstall_recovery": True,
            "final_package_record_absent": True,
            "final_filesystem_residue_absent": True,
            "network_disabled": True,
        }
        platforms = []
        for platform_id, package_format in (
            ("fedora-x86_64", "rpm"),
            ("ubuntu-x86_64", "deb"),
        ):
            steps = []
            for step_id in lifecycle.EXPECTED_STEP_IDS:
                administrator = step_id in lifecycle.ADMIN_STEP_IDS
                nonzero = step_id in lifecycle.NONZERO_STEP_IDS
                steps.append(
                    {
                        "id": step_id,
                        "actor": (
                            "package-administrator"
                            if administrator
                            else "standard-user"
                        ),
                        "uid_gid": (
                            lifecycle.ADMIN_USER
                            if administrator
                            else lifecycle.STANDARD_USER
                        ),
                        "expected_exit": "nonzero" if nonzero else "zero",
                        "observed_exit": "nonzero" if nonzero else "zero",
                        "output_bytes": 0,
                        "output_sha256": "a" * 64,
                        "status": "pass",
                    }
                )
            image_reference = "registry.invalid/image@sha256:" + "b" * 64
            platforms.append(
                {
                    "platform_id": platform_id,
                    "status": "pass",
                    "image": {
                        "reference": image_reference,
                        "id": "sha256:" + "c" * 64,
                        "architecture": "amd64",
                        "os": "linux",
                        "repo_digests": [image_reference],
                    },
                    "platform": {
                        "os_id": "fedora" if package_format == "rpm" else "ubuntu",
                        "version_id": "44" if package_format == "rpm" else "26.04",
                        "architecture": "x86_64",
                    },
                    "controls": {
                        "runtime": "rootless-podman",
                        "network": "none",
                        "privileged": False,
                        "capabilities_added": [],
                        "capabilities_dropped": ["CAP_CHOWN"],
                        "no_new_privileges": True,
                        "selinux_label_isolated_mount": True,
                        "pids_limit": 64,
                        "memory_limit_bytes": 512 * 1024 * 1024,
                        "root_filesystem": "ephemeral-writable-for-package-manager",
                        "package_mount": "read-only",
                    },
                    "package_format": package_format,
                    "package_administrator": lifecycle.ADMIN_USER,
                    "runtime_user": lifecycle.STANDARD_USER,
                    "managed_paths": lifecycle.expected_managed_paths(package_format),
                    "steps": steps,
                    "checks": copy.deepcopy(checks),
                }
            )
        return {
            "schema_version": 1,
            "status": "pass",
            "rootless_runtime": True,
            "network_used": False,
            "platforms": platforms,
        }

    def test_container_argv_closes_network_privilege_and_package_mount(self) -> None:
        target = lifecycle.ContainerTarget(
            "fedora-x86_64", "example.invalid/fedora@sha256:" + "d" * 64, "fedora", "44", "rpm"
        )
        with tempfile.TemporaryDirectory() as directory:
            argv = lifecycle.container_create_argv(target, Path(directory))
        self.assertIn("--pull=never", argv)
        self.assertIn("--network=none", argv)
        self.assertIn("--user=0:0", argv)
        self.assertIn("--cap-drop=all", argv)
        self.assertIn("--security-opt=no-new-privileges", argv)
        self.assertNotIn("--privileged", argv)
        mount = argv[argv.index("--mount") + 1]
        self.assertIn("dst=/packages", mount)
        self.assertTrue(mount.endswith("ro=true"))

    def test_runtime_exec_is_numeric_standard_user_with_fixed_environment(self) -> None:
        step = lifecycle.LifecycleStep("launch", "standard-user", ("id", "-u"))
        argv = lifecycle.container_exec_argv("a" * 64, step)
        self.assertIn("--user=10001:10001", argv)
        self.assertIn("--env=HOME=/tmp", argv)
        self.assertIn("--env=LANG=C", argv)
        self.assertNotIn("--user=0:0", argv)

    def test_mutable_image_reference_is_refused_before_inspection(self) -> None:
        with self.assertRaisesRegex(
            lifecycle.PackageLifecycleError,
            "package.lifecycle.image_reference_not_pinned",
        ):
            lifecycle._image_identity("docker.io/library/fedora:44")

    def test_package_manifests_are_closed_per_distribution(self) -> None:
        rpm = lifecycle.expected_managed_paths("rpm")
        deb = lifecycle.expected_managed_paths("deb")
        self.assertEqual(
            rpm,
            sorted((*lifecycle.INSTALLED_FILES, *lifecycle.INSTALLED_DIRECTORIES)),
        )
        self.assertEqual(set(lifecycle.INSTALLED_FILES) - set(deb), set())
        self.assertNotIn("/", deb)
        self.assertEqual(len(rpm), 8)
        self.assertEqual(len(deb), 12)

    def test_expected_lifecycle_is_valid(self) -> None:
        self.assertEqual(lifecycle.validate_container_lifecycle(self.valid_lifecycle()), [])

    def test_validator_rejects_network_privilege_or_runtime_root(self) -> None:
        report = self.valid_lifecycle()
        report["network_used"] = True
        report["platforms"][0]["controls"]["privileged"] = True
        report["platforms"][1]["runtime_user"] = lifecycle.ADMIN_USER
        failures = lifecycle.validate_container_lifecycle(report)
        self.assertIn("container lifecycle isolation changed", failures)
        self.assertIn("fedora-x86_64 controls changed", failures)
        self.assertIn("ubuntu-x86_64 identity changed", failures)

    def test_validator_rejects_missing_check_or_failed_step(self) -> None:
        report = self.valid_lifecycle()
        del report["platforms"][0]["checks"]["rollback"]
        report["platforms"][1]["steps"][0]["status"] = "fail"
        failures = lifecycle.validate_container_lifecycle(report)
        self.assertIn("fedora-x86_64 checks changed", failures)
        self.assertIn("ubuntu-x86_64 step closure changed", failures)

    def test_validator_rejects_malformed_nested_values_without_raising(self) -> None:
        report = self.valid_lifecycle()
        report["platforms"][0]["image"] = "not-an-image-record"
        report["platforms"][1]["steps"][0] = "not-a-step-record"
        failures = lifecycle.validate_container_lifecycle(report)
        self.assertIn("fedora-x86_64 structure changed", failures)
        self.assertIn("ubuntu-x86_64 step closure changed", failures)


if __name__ == "__main__":
    unittest.main()
