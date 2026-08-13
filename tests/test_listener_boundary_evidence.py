"""Tests for strict-local listener-boundary evidence."""

from __future__ import annotations

import copy
import unittest

from scripts import listener_boundary_evidence as evidence


class ListenerBoundaryEvidenceTests(unittest.TestCase):
    def valid_report(self) -> dict:
        return {
            "schema_version": 1,
            "artifact_id": "strict-local-listener-boundary",
            "source_revision": "a" * 40,
            "task_ids": ["10.1.1.3"],
            "status": "pass-linux-listener-reconciliation-and-docker-kvm-boundary",
            "policy_profile": copy.deepcopy(evidence.POLICY_PROFILE),
            "verification_commands": evidence.expected_commands(),
            "prerequisite_evidence": {
                "docker_topology": {
                    "path": evidence.TOPOLOGY_PATH.relative_to(evidence.ROOT).as_posix(),
                    "sha256": "b" * 64,
                    "source_revision": "c" * 40,
                    "targets": [
                        {
                            "target_id": target_id,
                            "host_listener_count": 0,
                            "private_raw_listener_count": 1,
                            "private_non_loopback_listener_count": 0,
                            "private_active_interface_count": 1,
                            "private_non_local_route_count": 0,
                            "cleanup_complete": True,
                        }
                        for target_id in (
                            "fedora-44-x86_64",
                            "ubuntu-26.04-x86_64",
                        )
                    ],
                },
                "docker_reachability": {
                    "path": evidence.REACHABILITY_PATH.relative_to(evidence.ROOT).as_posix(),
                    "sha256": "d" * 64,
                    "source_revision": "e" * 40,
                    "targets": [
                        {
                            "target_id": target_id,
                            "positions": [
                                {
                                    "position": position,
                                    "raw_tcp_connected": False,
                                    "status": "denied",
                                }
                                for position in ("host", "lan", "ordinary-container")
                            ],
                        }
                        for target_id in (
                            "fedora-44-x86_64",
                            "ubuntu-26.04-x86_64",
                        )
                    ],
                },
            },
            "claims": {
                "deterministic_listener_reconciliation_complete": True,
                "live_linux_loopback_and_wildcard_tested": True,
                "docker_private_listener_topology_tested": True,
                "continuous_process_confinement_tested": False,
                "packet_capture_performed": False,
                "inference_performed": False,
                "release_support": False,
            },
            "limitations": list(evidence.LIMITATIONS),
            "private_user_data_used": False,
            "external_network_used": False,
            "local_test_sockets_used": True,
            "sources": [
                {"path": path, "bytes": 1, "sha256": "f" * 64}
                for path in evidence.SOURCE_PATHS
            ],
        }

    def test_exact_report_is_valid(self) -> None:
        self.assertEqual(evidence.validate_report(self.valid_report()), [])

    def test_policy_or_command_mutation_fails(self) -> None:
        policy = self.valid_report()
        policy["policy_profile"]["maximum_declarations"] = 33
        self.assertIn("listener policy profile changed", evidence.validate_report(policy))
        command = self.valid_report()
        command["verification_commands"].pop()
        self.assertIn(
            "listener verification commands changed",
            evidence.validate_report(command),
        )

    def test_topology_or_reachability_broadening_fails(self) -> None:
        topology = self.valid_report()
        topology["prerequisite_evidence"]["docker_topology"]["targets"][0][
            "host_listener_count"
        ] = 1
        self.assertIn(
            "Docker listener topology evidence changed",
            evidence.validate_report(topology),
        )
        reachability = self.valid_report()
        reachability["prerequisite_evidence"]["docker_reachability"]["targets"][0][
            "positions"
        ][1]["status"] = "allowed"
        self.assertIn(
            "Docker listener reachability evidence changed",
            evidence.validate_report(reachability),
        )

    def test_claim_or_limitation_overstatement_fails(self) -> None:
        claim = self.valid_report()
        claim["claims"]["packet_capture_performed"] = True
        self.assertIn(
            "listener boundary evidence overclaimed",
            evidence.validate_report(claim),
        )
        limitation = self.valid_report()
        limitation["limitations"].pop()
        self.assertIn(
            "listener boundary limitations changed",
            evidence.validate_report(limitation),
        )

    def test_source_or_execution_classification_mutation_fails(self) -> None:
        source = self.valid_report()
        source["sources"].pop()
        self.assertIn(
            "listener boundary source evidence changed",
            evidence.validate_report(source),
        )
        execution = self.valid_report()
        execution["external_network_used"] = True
        self.assertIn(
            "listener boundary execution classification changed",
            evidence.validate_report(execution),
        )


if __name__ == "__main__":
    unittest.main()
