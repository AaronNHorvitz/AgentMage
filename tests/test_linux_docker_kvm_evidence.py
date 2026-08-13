"""Tests for the clean Fedora/Ubuntu Docker KVM bootstrap boundary."""

from __future__ import annotations

import contextlib
import io
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from scripts import linux_docker_kvm_evidence as evidence


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


if __name__ == "__main__":
    unittest.main()
