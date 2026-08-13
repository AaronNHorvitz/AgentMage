"""Tests for native Ubuntu-kernel Linux control evidence."""

from __future__ import annotations

import copy
import inspect
import socket
import unittest

from scripts import linux_native_ubuntu_control_evidence as evidence


class NativeUbuntuControlEvidenceTests(unittest.TestCase):
    def valid_report(self) -> dict:
        tests = {
            "sandbox_live": evidence.LIVE_SANDBOX_TESTS,
            "ipc": evidence.IPC_TESTS,
            "secret_service_static": evidence.SECRET_SERVICE_TESTS,
            "secret_service_live": evidence.SECRET_SERVICE_LIVE_TESTS,
            "startup_mapping": evidence.STARTUP_MAPPING_TESTS,
            "startup_live": evidence.STARTUP_LIVE_TESTS,
            "kernel_startup_refusal": evidence.KERNEL_STARTUP_TESTS,
        }
        prepared_sha = "c" * 64
        return {
            "schema_version": 1,
            "artifact_id": "linux-native-ubuntu-control-verification",
            "task_ids": list(evidence.TASK_IDS),
            "test_ids": list(evidence.TEST_IDS),
            "status": "pass-native-ubuntu-kernel-controls",
            "source_revision": "a" * 40,
            "sources": [
                {"path": path, "sha256": "b" * 64}
                for path in evidence.SOURCE_PATHS
            ],
            "bootstrap": {
                "schema_version": 1,
                "image_url": evidence.OFFICIAL_IMAGE_URL,
                "official_image_sha256": evidence.OFFICIAL_IMAGE_SHA256,
                "prepared_image_sha256": prepared_sha,
                "prepared_virtual_size_bytes":
                evidence.PREPARED_VIRTUAL_SIZE_BYTES,
                "packages": [
                    {"id": name, "version": "test-version"}
                    for name in evidence.BOOTSTRAP_PACKAGES
                ],
                "bootstrap_network": "qemu-user-network-bootstrap-only",
                "cloud_init_cleaned": True,
                "bootstrap_ssh_key_retained": False,
                "private_user_data_used": False,
                "qemu_launcher_class": "fedora-toolbox",
                "qemu_version": "QEMU emulator version test",
                "qemu_sha256": "d" * 64,
            },
            "execution": {
                "build": {
                    "container_image_id": "sha256:" + "e" * 64,
                    "network_used": False,
                    "source_revision": "a" * 40,
                    "toolchains": {
                        "rustc": "rustc test",
                        "cargo": "cargo test",
                    },
                    "binaries": [
                        {"id": name, "sha256": "f" * 64}
                        for name in ("platform-tests", "kernel-engine-tests")
                    ],
                },
                "platform": {
                    "distribution": "ubuntu",
                    "version": "26.04",
                    "architecture": "x86_64",
                    "kernel_release": "7.0.0-test-generic",
                    "virtualization": "kvm",
                    "cgroup_filesystem": "cgroup2",
                    "uid": evidence.TEST_UID,
                    "gid": evidence.TEST_GID,
                },
                "hypervisor": {
                    "acceleration": "kvm",
                    "launcher_class": "fedora-toolbox",
                    "qemu_version": "QEMU emulator version test",
                    "qemu_sha256": "d" * 64,
                },
                "network": {
                    "qemu_restrict_mode": True,
                    "host_forward": "loopback-ssh-only",
                    "external_connection_denied": True,
                },
                "trusted_tools": [
                    {
                        "id": identity,
                        "declared_path": path,
                        "canonical_path": f"/usr/lib/test/{identity}",
                        "bootstrap_package": package,
                        "owning_package": package,
                        "version": "test-version",
                        "owner_uid": 0,
                        "group_or_world_writable": False,
                        "sha256": "1" * 64,
                    }
                    for identity, path, package in evidence.TRUSTED_TOOLS
                ],
                "tests": {
                    name: [{"test": test, "status": "pass"} for test in expected]
                    for name, expected in tests.items()
                },
                "attack_results": [
                    {
                        "attack": attack,
                        "test": test,
                        "ubuntu": "pass-zero-escape-native-kernel",
                    }
                    for attack, test in evidence.ATTACK_TESTS.items()
                ],
                "startup_controls": {
                    "control_ids": list(evidence.STARTUP_CONTROL_IDS),
                    "mutation_statuses": ["unavailable", "invalid"],
                    "no_degraded_fallback": True,
                },
                "syscall_trace": {
                    "no_new_privileges": 1,
                    "seccomp_mode": 2,
                    "policy_id": "agentmage.linux.worker.deny.v1",
                    "observation_test":
                    "worker_kernel_status_confirms_no_new_privileges_and_seccomp",
                },
                "resource_trace": {
                    "cgroup_filesystem": "cgroup2",
                    "properties": list(evidence.CGROUP_PROPERTIES),
                    "runtime_limit_enforced": True,
                    "observation_test":
                    "transient_service_terminates_an_unbounded_worker",
                },
                "secret_service_lifecycle": {
                    "fresh_synthetic_keyring": True,
                    "live_round_trip_count": len(evidence.SECRET_SERVICE_LIVE_TESTS),
                    "agentmage_items_absent": True,
                    "synthetic_keyring_files_absent": True,
                    "host_keyring_touched": False,
                },
                "prepared_image_sha256": prepared_sha,
                "cleanup": {
                    "qemu_process_absent": True,
                    "loopback_ssh_listener_absent": True,
                    "disposable_overlay_removed": True,
                    "ephemeral_ssh_key_removed": True,
                    "test_binaries_removed_with_temporary_tree": True,
                },
            },
            "summary": {
                "native_ubuntu_kernel_controls_verified": True,
                "physical_host_certification": False,
                "test_count": 30,
                "attack_class_count": len(evidence.ATTACK_TESTS),
                "parity_dimensions_unblocked": 3,
                "cleanup_complete": True,
            },
            "private_values_present": False,
            "macos_evidence_substituted": False,
            "release_claim": "none",
            "limitations": list(evidence.LIMITATIONS),
        }

    def test_exact_native_kernel_report_is_valid(self) -> None:
        self.assertEqual(evidence.validate_report(self.valid_report()), [])

    def test_missing_test_or_control_trace_fails(self) -> None:
        missing = self.valid_report()
        missing["execution"]["tests"]["sandbox_live"].pop()
        self.assertIn(
            "native Ubuntu test closure changed",
            evidence.validate_report(missing),
        )
        weakened = self.valid_report()
        weakened["execution"]["syscall_trace"]["seccomp_mode"] = 0
        self.assertIn(
            "native Ubuntu syscall trace changed",
            evidence.validate_report(weakened),
        )

    def test_network_cleanup_or_physical_host_overclaim_fails(self) -> None:
        mutations = (
            ("network", "external_connection_denied", False),
            ("cleanup", "qemu_process_absent", False),
        )
        for group, field, replacement in mutations:
            with self.subTest(group=group, field=field):
                report = self.valid_report()
                report["execution"][group][field] = replacement
                self.assertTrue(evidence.validate_report(report))
        physical = self.valid_report()
        physical["summary"]["physical_host_certification"] = True
        self.assertIn(
            "native Ubuntu summary changed or overclaimed",
            evidence.validate_report(physical),
        )

    def test_private_release_or_macos_claim_fails(self) -> None:
        for field, replacement in (
            ("private_values_present", True),
            ("macos_evidence_substituted", True),
            ("release_claim", "supported"),
        ):
            with self.subTest(field=field):
                report = self.valid_report()
                report[field] = replacement
                self.assertIn(
                    "native Ubuntu report made an unsupported claim",
                    evidence.validate_report(report),
                )

    def test_test_output_parser_requires_exact_closure(self) -> None:
        expected = evidence.STARTUP_MAPPING_TESTS
        output = "\n".join(
            f"test module::tests::{name} ... ok" for name in expected
        )
        self.assertEqual(
            evidence.parse_test_output(output, expected),
            [{"test": name, "status": "pass"} for name in expected],
        )
        with self.assertRaises(evidence.NativeUbuntuEvidenceError):
            evidence.parse_test_output(output.rsplit("\n", 1)[0], expected)

    def test_observation_parser_requires_marker_and_one_passing_test(self) -> None:
        output = (
            f"test name ... {evidence.KERNEL_MARKER}\n"
            "ok\n\n"
            "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; "
            "68 filtered out; finished in 0.01s\n"
        )
        self.assertTrue(
            evidence.observation_test_passed(output, evidence.KERNEL_MARKER)
        )
        self.assertFalse(evidence.observation_test_passed(output, "wrong-marker"))
        self.assertFalse(
            evidence.observation_test_passed(
                output.replace("1 passed", "0 passed"), evidence.KERNEL_MARKER
            )
        )

    def test_cloud_init_retains_only_ephemeral_public_key(self) -> None:
        rendered = evidence.render_user_data("ssh-ed25519 synthetic-public", bootstrap=True)
        self.assertIn("ssh-ed25519 synthetic-public", rendered)
        self.assertIn("lock_passwd: true", rendered)
        self.assertNotIn("PRIVATE KEY", rendered)
        self.assertNotIn("password:", rendered)

    def test_tool_identity_and_bootstrap_mutations_fail(self) -> None:
        tool = self.valid_report()
        tool["execution"]["trusted_tools"][0]["owner_uid"] = 10001
        self.assertIn(
            "native Ubuntu trusted tool closure changed",
            evidence.validate_report(tool),
        )
        bootstrap = copy.deepcopy(self.valid_report())
        bootstrap["bootstrap"]["official_image_sha256"] = "0" * 64
        self.assertIn(
            "native Ubuntu bootstrap evidence is incomplete",
            evidence.validate_report(bootstrap),
        )

    def test_keyring_cleanup_uses_portable_single_path_unlink(self) -> None:
        source = inspect.getsource(evidence.initialize_synthetic_keyring)
        self.assertIn('"unlink /tmp/agentmage-keyring-env\\n"', source)
        self.assertIn('"unlink /tmp/agentmage-keyring-start\\n"', source)
        self.assertNotIn(
            "unlink /tmp/agentmage-keyring-env /tmp/agentmage-keyring-start",
            source,
        )

    def test_loopback_listener_probe_observes_present_and_absent_states(self) -> None:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
            listener.bind(("127.0.0.1", 0))
            listener.listen(1)
            port = listener.getsockname()[1]
            self.assertFalse(evidence.loopback_listener_absent(port))
        self.assertTrue(evidence.loopback_listener_absent(port))


if __name__ == "__main__":
    unittest.main()
