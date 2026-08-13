"""Tests for strict-local normal-operation network policy evidence."""

from __future__ import annotations

import copy
import unittest

from scripts import strict_local_network_policy_evidence as evidence


class StrictLocalNetworkPolicyEvidenceTests(unittest.TestCase):
    def valid_report(self) -> dict:
        return {
            "schema_version": 1,
            "artifact_id": "strict-local-normal-operation-network-policy",
            "task_ids": ["10.1.1.1"],
            "status": "pass-policy-source-worker-no-live-packet-capture",
            "source_revision": "a" * 40,
            "policy_matrix": evidence.policy_matrix(),
            "source_audit": {
                "scan_root_count": 8,
                "linux_inference_included": True,
                "undeclared_network_paths": 0,
                "network_capable_runtime_dependencies": 0,
            },
            "verification_commands": evidence.expected_command_records(),
            "runtime_evidence": [
                {
                    "runtime_id": runtime_id,
                    "enabled_models": 0,
                    "inference_started": False,
                    "release_claim": "none",
                    "sha256": "d" * 64,
                    "source_revision": "e" * 40,
                }
                for runtime_id in ("native", "docker")
            ],
            "claims": {
                "normal_operation_policy_complete": True,
                "linux_worker_network_isolation_tested": True,
                "live_host_process_confinement_tested": False,
                "packet_capture_performed": False,
                "inference_performed": False,
                "release_support": False,
            },
            "limitations": list(evidence.LIMITATIONS),
            "private_user_data_used": False,
            "network_used": False,
            "sources": [
                {"path": path, "bytes": 1, "sha256": "f" * 64}
                for path in evidence.SOURCE_PATHS
            ],
        }

    def test_exact_policy_is_valid(self) -> None:
        self.assertEqual(evidence.validate_report(self.valid_report()), [])

    def test_all_twenty_component_decisions_are_closed(self) -> None:
        matrix = evidence.policy_matrix()
        self.assertEqual(len(matrix), 2)
        self.assertTrue(all(len(item["components"]) == 10 for item in matrix))
        self.assertTrue(
            all(
                sum(
                    entry["decision"] == "allow-exact-authenticated-local-only"
                    for entry in item["components"]
                )
                == 1
                for item in matrix
            )
        )

    def test_component_destination_or_topology_broadening_fails(self) -> None:
        for field, replacement in (
            ("decision", "allow"),
            ("component", "unknown-broadened-component"),
        ):
            with self.subTest(field=field):
                changed = self.valid_report()
                changed["policy_matrix"][0]["components"][0][field] = replacement
                self.assertIn(
                    "strict-local component policy matrix changed",
                    evidence.validate_report(changed),
                )
        changed = self.valid_report()
        changed["policy_matrix"][1]["denied_destination_classes"].pop()
        self.assertIn(
            "strict-local component policy matrix changed",
            evidence.validate_report(changed),
        )

    def test_source_runtime_command_or_claim_mutation_fails(self) -> None:
        mutations = []
        source = self.valid_report()
        source["source_audit"]["undeclared_network_paths"] = 1
        mutations.append(source)
        runtime = self.valid_report()
        runtime["runtime_evidence"][0]["enabled_models"] = 1
        mutations.append(runtime)
        command = self.valid_report()
        command["verification_commands"][0]["status"] = "failed"
        mutations.append(command)
        claim = self.valid_report()
        claim["claims"]["packet_capture_performed"] = True
        mutations.append(claim)
        limitations = self.valid_report()
        limitations["limitations"].pop()
        mutations.append(limitations)
        self.assertTrue(all(evidence.validate_report(item) for item in mutations))

    def test_source_omission_and_network_use_fail(self) -> None:
        changed = self.valid_report()
        changed["sources"].pop()
        self.assertIn("strict-local source evidence changed", evidence.validate_report(changed))
        for field in ("network_used", "private_user_data_used"):
            with self.subTest(field=field):
                changed = copy.deepcopy(self.valid_report())
                changed[field] = True
                self.assertIn(
                    "strict-local network evidence used prohibited data or network",
                    evidence.validate_report(changed),
                )


if __name__ == "__main__":
    unittest.main()
