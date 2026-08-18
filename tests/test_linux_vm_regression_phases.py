from __future__ import annotations

import copy
import unittest

from scripts import linux_docker_kvm_evidence as docker_vm
from scripts.linux_vm_regression_phases import PHASES, validate_report


def valid_report() -> dict:
    return {
        "schema_version": 1,
        "record_type": "linux-vm-regression-phase-evidence",
        "task_id": "9.1.4.3",
        "source_revision": "0" * 40,
        "status": "pass-three-phase-local-kvm",
        "phase_order": list(PHASES),
        "qemu": {"launcher_class": "fedora-toolbox", "version": "QEMU", "sha256": "1" * 64},
        "targets": [
            {
                "target_id": target.target_id,
                "phases": {
                    "dependency-acquisition": {"line_count": 1, "sha256": "2" * 64},
                    "connected-adapter": {"line_count": 2, "sha256": "3" * 64},
                    "strict-offline": {
                        "external_connection_denied": True,
                        "process": {"line_count": 1, "sha256": "4" * 64},
                        "sockets": {"line_count": 1, "sha256": "5" * 64},
                        "packet": {"line_count": 1, "sha256": "6" * 64},
                        "packages": [],
                        "tests": {"command_count": 2, "status": "pass", "network_used": False},
                    },
                },
                "guest_cleanup": {
                    "connected": {"qemu_process_absent": True, "loopback_ssh_listener_absent": True},
                    "offline": {"qemu_process_absent": True, "loopback_ssh_listener_absent": True},
                    "transient_source_absent": True,
                },
                "overlay_cleanup_verified": True,
            }
            for target in docker_vm.TARGETS
        ],
        "repository_credentials_injected": False,
        "private_host_data_retained": False,
        "release_claim": False,
        "sources": [],
    }


class LinuxVmRegressionPhaseTests(unittest.TestCase):
    def test_exact_report_passes(self) -> None:
        self.assertEqual(validate_report(valid_report()), [])

    def test_phase_target_cleanup_and_authority_mutations_fail(self) -> None:
        mutations = []
        for mutate in (
            lambda value: value["phase_order"].reverse(),
            lambda value: value["targets"].reverse(),
            lambda value: value["targets"][0]["phases"]["strict-offline"].update(external_connection_denied=False),
            lambda value: value["targets"][0].update(overlay_cleanup_verified=False),
            lambda value: value.update(repository_credentials_injected=True),
            lambda value: value.update(private_host_data_retained=True),
            lambda value: value.update(release_claim=True),
        ):
            value = valid_report()
            mutate(value)
            mutations.append(value)
        for mutation in mutations:
            self.assertTrue(validate_report(mutation))


if __name__ == "__main__":
    unittest.main()
