from __future__ import annotations

import copy
import unittest

from scripts import sprint_17_linux_git_evidence as evidence


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
            "record_type": "sprint-17-installed-linux-git-guest-result",
            "distribution": distribution,
            "status": "pass",
            "strict_offline": True,
            "package_sha256": "e" * 64,
            "host_sha256": "f" * 64,
            "runtime_sha256": {
                "bubblewrap": "1" * 64,
                "git": "2" * 64,
                "systemctl": "3" * 64,
                "systemd_run": "4" * 64,
            },
            "git_operations": evidence.GIT_OPERATIONS,
            "fixture_states": evidence.FIXTURE_STATES,
            "attack_cases": evidence.ATTACK_CASES,
            "instruction_classes": evidence.INSTRUCTION_CLASSES,
            "workspace_invariant": True,
            "loopback_contact_observed": False,
            "canary_execution_count": 0,
            "source_content_in_discovery_records": False,
            "transient_unit_residue": False,
            "package_residue": False,
            "network_used_during_execution": False,
            "private_data_used": False,
            "commands": [command()],
        },
        "observation_sha256": {"processes": "5" * 64, "sockets": "6" * 64},
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
        "record_type": "sprint-17-installed-linux-git-matrix",
        "task_ids": ["17.1.1.2", "17.1.1.3", "17.1.3.1", "17.1.3.3"],
        "source_revision": "b" * 40,
        "status": "pass-installed-linux-git-and-instruction-matrix",
        "qemu": {
            "launcher_class": "toolbox",
            "version": "qemu-test",
            "sha256": "7" * 64,
        },
        "targets": [
            target("fedora-44-x86_64", "fedora"),
            target("ubuntu-26.04-x86_64", "ubuntu"),
        ],
        "git_operations": evidence.GIT_OPERATIONS,
        "fixture_states": evidence.FIXTURE_STATES,
        "attack_cases": evidence.ATTACK_CASES,
        "instruction_classes": evidence.INSTRUCTION_CLASSES,
        "linux_git_matrix_complete": True,
        "linux_instruction_discovery_complete": True,
        "linux_live_network_observation_complete": True,
        "cross_platform_matrix_complete": False,
        "manual_parser_fuzzing_complete": False,
        "independent_review_complete": False,
        "macos_evidence_substituted": False,
        "private_host_data_used": False,
        "repository_credentials_injected": False,
        "release_claim": False,
        "sources": [
            {"path": path, "bytes": 1, "sha256": "8" * 64}
            for path in evidence.SOURCE_PATHS
        ],
    }


class Sprint17LinuxGitEvidenceTests(unittest.TestCase):
    def test_exact_native_matrix_is_valid(self) -> None:
        self.assertEqual(evidence.validate_report(report()), [])

    def test_target_result_runtime_and_cleanup_mutations_fail(self) -> None:
        mutations = (
            lambda value: value["targets"].pop(),
            lambda value: value["targets"][0]["guest_result"].update(
                {"loopback_contact_observed": True}
            ),
            lambda value: value["targets"][0]["guest_result"]["runtime_sha256"].pop(
                "git"
            ),
            lambda value: value["targets"][1]["cleanup"].update(
                {"overlay_absent": False}
            ),
            lambda value: value["targets"][1]["guest_result"]["commands"][0].update(
                {"exit_code": 1}
            ),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(evidence.validate_report(changed))

    def test_cross_platform_fuzz_review_private_and_release_overclaims_fail(self) -> None:
        fields = (
            "cross_platform_matrix_complete",
            "manual_parser_fuzzing_complete",
            "independent_review_complete",
            "macos_evidence_substituted",
            "private_host_data_used",
            "repository_credentials_injected",
            "release_claim",
        )
        for field in fields:
            changed = copy.deepcopy(report())
            changed[field] = True
            self.assertTrue(evidence.validate_report(changed))

    def test_inventory_source_and_linux_completion_drift_fail(self) -> None:
        mutations = (
            lambda value: value["git_operations"].pop(),
            lambda value: value["fixture_states"].pop(),
            lambda value: value["attack_cases"].pop(),
            lambda value: value["instruction_classes"].pop(),
            lambda value: value["sources"].pop(),
            lambda value: value.update({"linux_git_matrix_complete": False}),
            lambda value: value.update({"linux_instruction_discovery_complete": False}),
            lambda value: value.update({"linux_live_network_observation_complete": False}),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(evidence.validate_report(changed))


if __name__ == "__main__":
    unittest.main()
