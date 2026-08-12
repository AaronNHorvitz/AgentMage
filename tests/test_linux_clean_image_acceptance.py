"""Tests for clean Fedora and Ubuntu graphical package acceptance evidence."""

from __future__ import annotations

import copy
import json
import tempfile
import unittest
from pathlib import Path

from scripts import linux_clean_image_acceptance as acceptance


class LinuxCleanImageAcceptanceTests(unittest.TestCase):
    def valid_platform(self, target: acceptance.Target) -> dict:
        steps = []
        for step_id in acceptance.STEP_IDS:
            administrator = step_id in acceptance.ADMIN_STEPS
            absent = step_id == "package-record-absent"
            steps.append(
                {
                    "id": step_id,
                    "actor": (
                        "package-administrator" if administrator else "standard-user"
                    ),
                    "uid_gid": (
                        acceptance.PACKAGE_ADMIN
                        if administrator
                        else acceptance.STANDARD_USER
                    ),
                    "expected_exit": "nonzero" if absent else "zero",
                    "observed_exit": "nonzero" if absent else "zero",
                    "status": "pass",
                }
            )
        labels = {
            "org.agentmage.acceptance.base-image": target.base_image,
            "org.agentmage.acceptance.distribution": target.distribution,
            "org.agentmage.acceptance.vscode-version": acceptance.VSCODE_VERSION,
            "org.agentmage.acceptance.vscode-sha256": (
                acceptance.VSCODE_ARCHIVE_SHA256
            ),
        }
        return {
            "platform_id": target.platform_id,
            "distribution": target.distribution,
            "version": target.version,
            "architecture": "x86_64",
            "package_format": target.package_format,
            "base_image": target.base_image,
            "acceptance_image": {
                "tag": target.image,
                "id": "sha256:" + "a" * 64,
                "labels": labels,
            },
            "container_controls": copy.deepcopy(acceptance.CONTAINER_CONTROLS),
            "actors": {
                "package_administrator": acceptance.PACKAGE_ADMIN,
                "agentmage_and_vscode_user": acceptance.STANDARD_USER,
            },
            "steps": steps,
            "vscode": {
                "version": acceptance.VSCODE_VERSION,
                "commit": acceptance.VSCODE_COMMIT,
                "archive_sha256": acceptance.VSCODE_ARCHIVE_SHA256,
                "extension_id": acceptance.EXTENSION_ID,
                "extension_version": "0.0.0",
                "activation_event": (
                    "ExtensionService#_doActivateExtension "
                    + acceptance.EXTENSION_ID
                ),
                "extension_host_log_sha256": "b" * 64,
                "provider_probe": copy.deepcopy(acceptance.EXPECTED_PROBE),
                "chromium_sandbox": "disabled-inside-outer-test-container-only",
            },
            "process_boundary": {
                "standard_user_only": True,
                "process_counts": {
                    "Xvfb": 1,
                    "agentmage-host": 0,
                    "agentmage-native-inference": 0,
                    "at-spi-bus-launcher": 1,
                    "code": 5,
                    "dbus-daemon": 1,
                },
            },
            "package_file_count": 8,
            "inference_descriptor": copy.deepcopy(
                acceptance.EXPECTED_INFERENCE_DESCRIPTOR
            ),
            "residue": {
                "active_extension_registration": False,
                "package_record": False,
                "package_paths": False,
                "runtime_processes": False,
                "runtime_socket": False,
                "isolated_user_profile": False,
            },
            "container_removed": True,
        }

    def valid_report(self) -> dict:
        return {
            "schema_version": 1,
            "artifact_id": "linux-clean-image-graphical-acceptance",
            "task_ids": ["9.1.3.4"],
            "status": "pass-clean-fedora-ubuntu-images",
            "source": {
                "revision": "c" * 40,
                "tree": "d" * 40,
                "files": [
                    {"path": "Cargo.toml", "sha256": "e" * 64, "bytes": 1}
                ],
            },
            "published_procedure": {
                "path": acceptance.PROCEDURE_PATH.relative_to(
                    acceptance.ROOT
                ).as_posix(),
                "package_scripts": acceptance.EXPECTED_PACKAGE_SCRIPTS,
                "vscode_archive_url": acceptance.VSCODE_ARCHIVE_URL,
                "vscode_archive_sha256": acceptance.VSCODE_ARCHIVE_SHA256,
            },
            "packages": {
                kind: {
                    "filename": f"agentmage.{kind}",
                    "sha256": "f" * 64,
                    "bytes": 1,
                }
                for kind in ("deb", "rpm", "vsix")
            },
            "rootless_runtime": True,
            "platforms": [
                self.valid_platform(target) for target in acceptance.TARGETS
            ],
            "bootstrap_network_separate": True,
            "acceptance_network_used": False,
            "private_values_present": False,
            "enabled_models": 0,
            "inference_performed": False,
            "release_claim": "none",
            "limitations": list(acceptance.LIMITATIONS),
        }

    def test_expected_report_is_valid(self) -> None:
        self.assertEqual(acceptance.validate_report(self.valid_report()), [])

    def test_network_privilege_and_actor_mutations_fail(self) -> None:
        report = self.valid_report()
        platform = report["platforms"][0]
        platform["container_controls"]["network"] = "host"
        platform["steps"][0]["uid_gid"] = "0:0"
        failures = acceptance.validate_report(report)
        self.assertIn("container controls weakened: fedora-x86_64", failures)
        self.assertIn("acceptance step closure changed: fedora-x86_64", failures)

    def test_probe_activation_and_process_mutations_fail(self) -> None:
        report = self.valid_report()
        platform = report["platforms"][1]
        platform["vscode"]["provider_probe"]["status"] = "fail"
        platform["process_boundary"]["standard_user_only"] = False
        failures = acceptance.validate_report(report)
        self.assertIn("VS Code acceptance closure changed: ubuntu-x86_64", failures)
        self.assertIn(
            "standard-user process boundary changed: ubuntu-x86_64", failures
        )

    def test_missing_step_or_residue_overclaim_fails(self) -> None:
        report = self.valid_report()
        platform = report["platforms"][0]
        platform["steps"].pop()
        platform["residue"]["runtime_socket"] = True
        failures = acceptance.validate_report(report)
        self.assertIn("acceptance step closure changed: fedora-x86_64", failures)
        self.assertIn("residue closure changed: fedora-x86_64", failures)

    def test_source_scope_covers_harness_probe_and_procedure(self) -> None:
        required = {
            "docs/support/linux-clean-image-acceptance.md",
            "release/acceptance/Containerfile.linux-vscode",
            "release/acceptance/vscode-probe/extension.js",
            "scripts/linux_clean_image_acceptance.py",
            "tests/test_linux_clean_image_acceptance.py",
        }
        self.assertEqual(required - acceptance.SOURCE_EXACT, set())

    def test_published_procedure_is_machine_bound(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "docs/support").mkdir(parents=True)
            package = {"scripts": dict(acceptance.EXPECTED_PACKAGE_SCRIPTS)}
            (root / "package.json").write_text(json.dumps(package), encoding="utf-8")
            procedure = "\n".join(
                [
                    *(
                        f"npm run {name}"
                        for name in acceptance.EXPECTED_PACKAGE_SCRIPTS
                    ),
                    acceptance.VSCODE_ARCHIVE_URL,
                    acceptance.VSCODE_ARCHIVE_SHA256,
                    "--network=none",
                ]
            )
            (root / "docs/support/linux-clean-image-acceptance.md").write_text(
                procedure, encoding="utf-8"
            )
            self.assertEqual(acceptance.validate_published_procedure(root), [])
            package["scripts"][
                "evidence:story9.1-linux-acceptance:build"
            ] += " --weakened"
            (root / "package.json").write_text(json.dumps(package), encoding="utf-8")
            self.assertIn(
                "published package script drifted: "
                "evidence:story9.1-linux-acceptance:build",
                acceptance.validate_published_procedure(root),
            )

    def test_limitations_and_release_state_are_exact(self) -> None:
        report = self.valid_report()
        report["limitations"].pop()
        report["release_claim"] = "supported"
        failures = acceptance.validate_report(report)
        self.assertIn("clean-image acceptance limitations changed", failures)
        self.assertIn(
            "authority, network, privacy, model, or release state changed", failures
        )


if __name__ == "__main__":
    unittest.main()
