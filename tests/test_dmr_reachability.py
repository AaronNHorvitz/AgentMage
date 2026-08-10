from __future__ import annotations

import copy
import unittest

from scripts.dmr_reachability import classify_failure, validate_result


class DMRReachabilityTests(unittest.TestCase):
    def setUp(self) -> None:
        probes = {
            "guarded_kernel_adapter": {"passed": True},
            "unrelated_same_user_process": {"passed": True},
            "tool_container": {"passed": True},
            "separate_user_and_network_namespace": {"passed": True},
            "emulated_lan_peer": {"passed": True},
        }
        self.result = {
            "schema_version": 1,
            "record_type": "dmr_guarded_reachability_probe",
            "source_revision": "0" * 40,
            "runner_sha256": "1" * 64,
            "data_classification": "public_synthetic_metadata_only",
            "dmr_image_digest": "sha256:bd94095bbc1ddc4266c3a88f582a92562c6b63eceb175572c9a60045663727c9",
            "probe_image_digest": "sha256:ff71127c215572121f1991bacf17f39ec5fcfd2de1f1c01a595835495bb9adfc",
            "deployment": {
                "mode": "exact_image_rootfs_binary_in_private_user_mount_network_namespace",
                "raw_transport": "private_tmpfs_unix_domain_socket",
                "guard_transport": "mode_0600_authenticated_unix_domain_socket",
                "fresh_in_memory_token": True,
                "supervisor_and_dmr_dumpable": False,
                "production_support_claim": False,
            },
            "runtime": {
                "private_network_before": {
                    "interfaces": [{"name": "lo", "rx_bytes": 0, "tx_bytes": 0}],
                    "route_count": 0,
                    "tcp_listener_count": 0,
                },
                "private_network_after": {
                    "interfaces": [{"name": "lo", "rx_bytes": 0, "tx_bytes": 0}],
                    "route_count": 0,
                    "tcp_listener_count": 0,
                },
                "raw_socket_location": "private_tmpfs_only",
                "guard_socket_mode": "0o600",
            },
            "probes": probes,
            "private_network_isolated": True,
            "supervisor_exit": 0,
            "supervisor_stderr_empty": True,
            "status": "PASS",
            "limitations": ["one", "two", "three", "four"],
        }

    def test_valid_complete_probe_matrix_passes(self) -> None:
        self.assertEqual(validate_result(self.result, check_revision=False), [])

    def test_failed_peer_or_guard_probe_is_rejected(self) -> None:
        for name in self.result["probes"]:
            with self.subTest(probe=name):
                changed = copy.deepcopy(self.result)
                changed["probes"][name]["passed"] = False
                changed["status"] = "FAIL"
                self.assertTrue(validate_result(changed, check_revision=False))

    def test_production_claim_and_dumpable_supervisor_are_rejected(self) -> None:
        changed = copy.deepcopy(self.result)
        changed["deployment"]["production_support_claim"] = True
        changed["deployment"]["supervisor_and_dmr_dumpable"] = True
        changed["status"] = "FAIL"
        failures = validate_result(changed, check_revision=False)
        self.assertTrue(any("overstated" in item for item in failures))

    def test_failure_classifier_is_content_bounded(self) -> None:
        self.assertEqual(classify_failure("Traceback PermissionError: nope"), "PermissionError")
        self.assertEqual(classify_failure("unexpected detail"), "OTHER_FAILURE")


if __name__ == "__main__":
    unittest.main()
