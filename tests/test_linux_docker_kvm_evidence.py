"""Tests for the clean Fedora/Ubuntu Docker KVM bootstrap boundary."""

from __future__ import annotations

import contextlib
import io
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from scripts import linux_docker_kvm_evidence as evidence
from scripts import linux_docker_kvm_guest as guest_probe


class LinuxDockerKvmEvidenceTests(unittest.TestCase):
    def test_target_and_artifact_closures_are_exact(self) -> None:
        self.assertEqual(
            [target.target_id for target in evidence.TARGETS],
            ["fedora-44-x86_64", "ubuntu-26.04-x86_64"],
        )
        self.assertEqual(evidence.VIRTUAL_SIZE_BYTES, 32 * 1024 * 1024 * 1024)
        self.assertRegex(evidence.RUNNER_DIGEST, r"^sha256:[0-9a-f]{64}$")
        self.assertRegex(evidence.MODEL_DIGEST, r"^[0-9a-f]{64}$")
        self.assertEqual(
            set(evidence.MODEL_FILES),
            {"gemma-4-E4B-it-Q4_K_M.gguf", "mmproj-F16.gguf"},
        )
        for target in evidence.TARGETS:
            self.assertRegex(target.official_sha256, r"^[0-9a-f]{64}$")
            self.assertTrue(target.official_url.startswith("https://"))
            self.assertIn(target.ssh_service, {"ssh.service", "sshd.service"})
            self.assertTrue(set(target.docker_packages).issubset(target.packages))

    def test_seed_has_one_fixed_user_and_bootstrap_only_packages(self) -> None:
        key = "ssh-ed25519 synthetic-public-key"
        for target in evidence.TARGETS:
            bootstrap = evidence.render_seed(target, key, bootstrap=True)
            acceptance = evidence.render_seed(target, key, bootstrap=False)
            for rendered in (bootstrap, acceptance):
                self.assertIn("uid: 10001", rendered)
                self.assertIn("ssh_pwauth: false", rendered)
                self.assertIn(key, rendered)
                self.assertIn("docker.service", rendered)
                self.assertNotIn("password:", rendered)
            self.assertIn(target.packages[0], bootstrap)
            self.assertNotIn(target.packages[0], acceptance)

    def test_missing_prepared_metadata_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            target = evidence.Target(
                target_id=evidence.FEDORA.target_id,
                distribution=evidence.FEDORA.distribution,
                version=evidence.FEDORA.version,
                official_url=evidence.FEDORA.official_url,
                official_sha256=evidence.FEDORA.official_sha256,
                official_path=root / "official.qcow2",
                prepared_path=root / "prepared.qcow2",
                metadata_path=root / "prepared.json",
                packages=evidence.FEDORA.packages,
                admin_group=evidence.FEDORA.admin_group,
                ssh_service=evidence.FEDORA.ssh_service,
                docker_packages=evidence.FEDORA.docker_packages,
            )
            with self.assertRaisesRegex(
                evidence.DockerKvmEvidenceError, "metadata is unavailable"
            ):
                evidence.validate_prepared(target)

    def test_force_requires_explicit_bootstrap(self) -> None:
        with mock.patch.object(sys, "argv", ["evidence", "--force-bootstrap"]):
            with contextlib.redirect_stderr(io.StringIO()):
                self.assertEqual(evidence.main(), 1)

    def test_report_validator_rejects_missing_targets_and_overclaims(self) -> None:
        report = {
            "schema_version": 1,
            "artifact_id": "linux-docker-kvm-topology",
            "source_revision": "a" * 40,
            "task_ids": ["9.2.2.1"],
            "status": "pass-live-topology-no-inference",
            "targets": [],
            "claims": {
                "docker_engine_directly_tested": True,
                "live_topology_inspected": True,
                "native_adapter_live_inference": False,
                "docker_inference_performed": False,
                "model_quality_evaluated": False,
                "release_support": False,
            },
            "sources": [
                {"path": path, "bytes": 1, "sha256": "b" * 64}
                for path in evidence.SOURCE_PATHS
            ],
        }
        self.assertIn("target closure", "; ".join(evidence.validate_report(report)))
        report["claims"]["release_support"] = True
        self.assertIn("overclaim", "; ".join(evidence.validate_report(report)))

    def test_guest_bootstrap_is_exact_and_not_json_serialized(self) -> None:
        runtime = {
            "pid": 4242,
            "start_time_ticks": 99,
            "executable_sha256": "1" * 64,
            "cgroup_sha256": "2" * 64,
        }
        with tempfile.TemporaryDirectory() as name:
            path = Path(name) / "bootstrap.bin"
            with mock.patch.object(guest_probe.os, "chown"):
                guest_probe.write_bootstrap(runtime, path)
            frame = path.read_bytes()
            self.assertEqual(len(frame), 126)
            self.assertEqual(frame[:8], b"AMDG0001")
            self.assertNotIn(frame[94:].hex(), json.dumps(runtime))

    def test_guest_cleanup_is_attempted_after_collection_failure(self) -> None:
        with (
            mock.patch.object(guest_probe.os, "geteuid", return_value=0),
            mock.patch.object(guest_probe.sys, "argv", ["probe", "a" * 40]),
            mock.patch.object(
                guest_probe,
                "collect",
                side_effect=guest_probe.GuestEvidenceError("synthetic"),
            ),
            mock.patch.object(guest_probe, "cleanup", return_value={"clean": True}) as cleanup,
            contextlib.redirect_stderr(io.StringIO()),
        ):
            self.assertEqual(guest_probe.main(), 1)
        cleanup.assert_called_once_with()


if __name__ == "__main__":
    unittest.main()
