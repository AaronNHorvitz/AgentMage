from __future__ import annotations

import copy
import json
import unittest
from pathlib import Path

from scripts import linux_docker_kvm_evidence as docker_vm
from scripts.linux_vm_promoted_guest import LANE_ORDER, bounded_failure
from scripts.linux_vm_promoted_matrix import bounded_diagnostic, install_script, validate_report


def valid_report() -> dict:
    lanes = [
        {"id": lane, "status": "pass", "command_count": 1, "receipt_sha256": "1" * 64}
        for lane in LANE_ORDER
    ]
    for item in lanes:
        if item["id"] == "native-runtime":
            item["adapter"] = "native-llama-cpp"
        elif item["id"] == "docker-compatibility":
            item.update(adapter="docker-model-runner-compatibility", separate_from_native=True)
        elif item["id"] == "accessibility":
            item["human_review"] = False
    targets = []
    for target in docker_vm.TARGETS:
        targets.append(
            {
                "target_id": target.target_id,
                "source_revision": "0" * 40,
                "strict_offline": True,
                "guest_result": {
                    "distribution": target.distribution,
                    "status": "pass",
                    "network_used": False,
                    "lanes": copy.deepcopy(lanes),
                    "native_and_docker_results_distinct": True,
                    "model_inference_executed": False,
                    "human_accessibility_review_claim": False,
                    "release_claim": False,
                    "command_receipts": [
                        {
                            "lane": lane,
                            "expected_exit": "zero",
                            "observed_exit": "zero",
                            "output_sha256": "4" * 64,
                        }
                        for lane in LANE_ORDER
                    ],
                },
                "observation_sha256": {"processes": "2" * 64, "sockets": "3" * 64},
                "cleanup": {
                    "connected": {"qemu_process_absent": True, "loopback_ssh_listener_absent": True},
                    "offline": {"qemu_process_absent": True, "loopback_ssh_listener_absent": True},
                    "overlay_absent": True,
                    "transient_source_absent": True,
                    "credential_material_absent": True,
                },
            }
        )
    return {
        "schema_version": 1,
        "record_type": "linux-vm-promoted-matrix-evidence",
        "task_id": "9.1.4.4",
        "source_revision": "0" * 40,
        "status": "pass-independent-fedora-ubuntu-matrices",
        "lane_order": list(LANE_ORDER),
        "targets": targets,
        "target_results_interchangeable": False,
        "adapter_results_interchangeable": False,
        "repository_credentials_injected": False,
        "private_host_data_retained": False,
        "model_inference_claim": False,
        "human_accessibility_review_claim": False,
        "physical_host_claim": False,
        "release_claim": False,
    }


class PromotedLinuxVmMatrixTests(unittest.TestCase):
    def test_inner_guest_diagnostic_is_bounded_and_path_redacted(self) -> None:
        diagnostic = bounded_failure("\n".join(["/home/agentmage/source/private"] * 20))
        self.assertNotIn("/home/agentmage", diagnostic)
        self.assertLessEqual(len(diagnostic), 8000)
        self.assertEqual(diagnostic.count("<GUEST_SOURCE>"), 12)

    def test_acquisition_diagnostic_is_bounded_and_path_redacted(self) -> None:
        output = "\n".join(["/home/agentmage/source/private"] * 20)
        diagnostic = bounded_diagnostic(output)
        self.assertNotIn("/home/agentmage", diagnostic)
        self.assertLessEqual(len(diagnostic), 10000)
        self.assertEqual(diagnostic.count("<GUEST_SOURCE>"), 12)

    def test_bootstrap_gives_npm_its_pinned_node_path(self) -> None:
        for target in docker_vm.TARGETS:
            script = install_script(target)
            self.assertIn(
                "sudo env PATH=/opt/node/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
                script,
            )
            self.assertIn("/opt/node/bin/npm install --global", script)
            self.assertIn(" unzip", script)

    def test_guest_checkout_is_fail_fast_and_detached(self) -> None:
        source = Path("scripts/linux_vm_promoted_matrix.py").read_text(encoding="utf-8")
        self.assertIn('"set -eu\\n"', source)
        self.assertIn("checkout --quiet --detach FETCH_HEAD", source)
        self.assertNotIn("git clone --quiet /home/agentmage/source.bundle", source)

    def test_canonical_product_check_builds_before_source_audit(self) -> None:
        package = json.loads(Path("package.json").read_text(encoding="utf-8"))
        command = package["scripts"]["product:check"]
        self.assertLess(command.index("product:build"), command.index("product:lint"))

    def test_exact_independent_matrix_passes(self) -> None:
        self.assertEqual(validate_report(valid_report()), [])

    def test_distribution_substitution_fails(self) -> None:
        value = valid_report()
        value["targets"][1]["guest_result"] = copy.deepcopy(value["targets"][0]["guest_result"])
        self.assertTrue(validate_report(value))

    def test_lane_and_adapter_substitution_fails(self) -> None:
        for mutate in (
            lambda value: value["targets"][0]["guest_result"]["lanes"].reverse(),
            lambda value: value["targets"][0]["guest_result"]["lanes"][8].update(adapter="docker-model-runner-compatibility"),
            lambda value: value["targets"][0]["guest_result"].update(native_and_docker_results_distinct=False),
            lambda value: value.update(adapter_results_interchangeable=True),
        ):
            value = valid_report()
            mutate(value)
            self.assertTrue(validate_report(value))

    def test_cleanup_network_and_claim_mutations_fail(self) -> None:
        for mutate in (
            lambda value: value["targets"][0].update(strict_offline=False),
            lambda value: value["targets"][0]["cleanup"].update(overlay_absent=False),
            lambda value: value["targets"][0]["guest_result"].update(network_used=True),
            lambda value: value.update(repository_credentials_injected=True),
            lambda value: value.update(human_accessibility_review_claim=True),
            lambda value: value.update(release_claim=True),
        ):
            value = valid_report()
            mutate(value)
            self.assertTrue(validate_report(value))


if __name__ == "__main__":
    unittest.main()
