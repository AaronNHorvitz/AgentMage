from __future__ import annotations

import copy
import unittest

from scripts import sprint_16_linux_worker_evidence as evidence


def command() -> dict[str, object]:
    return {
        "id": "guest-command-1",
        "executable": "cargo",
        "exit_code": 0,
        "output_bytes": 1,
        "output_sha256": "a" * 64,
    }


def target(target_id: str, distribution: str) -> dict[str, object]:
    return {
        "target_id": target_id,
        "source_revision": "b" * 40,
        "prepared_base_sha256": "c" * 64,
        "source_bundle_sha256": "d" * 64,
        "strict_offline": True,
        "guest_result": {
            "schema_version": 1,
            "record_type": "sprint-16-installed-worker-guest-result",
            "distribution": distribution,
            "status": "pass",
            "strict_offline": True,
            "package_sha256": "e" * 64,
            "worker": {
                "path_class": "root-owned-package-libexec",
                "mode": "0755",
                "sha256": "f" * 64,
            },
            "verified_operations": evidence.VERIFIED_OPERATIONS,
            "receipt_count": len(evidence.VERIFIED_OPERATIONS),
            "workspace_invariant": True,
            "worker_process_residue": False,
            "transient_unit_residue": False,
            "package_residue": False,
            "network_used_during_execution": False,
            "private_data_used": False,
            "commands": [command()],
        },
        "observation_sha256": {"processes": "1" * 64, "sockets": "2" * 64},
        "cleanup": {
            "connected": {
                "qemu_process_absent": True,
                "loopback_ssh_listener_absent": True,
            },
            "offline": {
                "qemu_process_absent": True,
                "loopback_ssh_listener_absent": True,
            },
            "overlay_absent": True,
            "transient_source_absent": True,
            "credential_material_absent": True,
        },
    }


def report() -> dict[str, object]:
    return {
        "schema_version": 1,
        "record_type": "sprint-16-installed-linux-worker-matrix",
        "task_ids": ["16.1.1.5", "16.1.2.3"],
        "source_revision": "b" * 40,
        "status": "pass-installed-linux-worker-operation-matrix",
        "qemu": {
            "launcher_class": "toolbox",
            "version": "qemu-test",
            "sha256": "3" * 64,
        },
        "targets": [
            target("fedora-44-x86_64", "fedora"),
            target("ubuntu-26.04-x86_64", "ubuntu"),
        ],
        "verified_operations": evidence.VERIFIED_OPERATIONS,
        "complete_ten_tool_matrix": True,
        "attack_matrix_complete": False,
        "cleanup_campaign_complete": False,
        "macos_evidence_substituted": False,
        "private_host_data_used": False,
        "repository_credentials_injected": False,
        "release_claim": False,
        "sources": [
            {"path": path, "bytes": 1, "sha256": "4" * 64}
            for path in evidence.SOURCE_PATHS
        ],
    }


class Sprint16LinuxWorkerEvidenceTests(unittest.TestCase):
    def test_exact_operation_matrix_report_is_valid(self) -> None:
        self.assertEqual(evidence.validate_report(report()), [])

    def test_target_receipt_and_cleanup_mutations_fail(self) -> None:
        mutations = (
            lambda value: value["targets"].pop(),
            lambda value: value["targets"][0]["guest_result"].update({"receipt_count": 9}),
            lambda value: value["targets"][0]["guest_result"]["commands"][0].update(
                {"exit_code": 1}
            ),
            lambda value: value["targets"][1]["cleanup"].update({"overlay_absent": False}),
            lambda value: value["targets"][1]["guest_result"].update(
                {"worker_process_residue": True}
            ),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(evidence.validate_report(changed))

    def test_platform_privacy_and_completion_overclaims_fail(self) -> None:
        fields = (
            "attack_matrix_complete",
            "cleanup_campaign_complete",
            "macos_evidence_substituted",
            "private_host_data_used",
            "repository_credentials_injected",
            "release_claim",
        )
        for field in fields:
            changed = copy.deepcopy(report())
            changed[field] = True
            self.assertTrue(evidence.validate_report(changed))
        changed = copy.deepcopy(report())
        changed["complete_ten_tool_matrix"] = False
        self.assertTrue(evidence.validate_report(changed))

    def test_source_and_qemu_identity_mutations_fail(self) -> None:
        changed = copy.deepcopy(report())
        changed["sources"].pop()
        changed["qemu"]["sha256"] = "invalid"
        failures = evidence.validate_report(changed)
        self.assertIn("installed worker source closure drifted", failures)
        self.assertIn("installed worker QEMU identity drifted", failures)


if __name__ == "__main__":
    unittest.main()
