"""Tests for isolated firewall and packet-capture evidence."""

from __future__ import annotations

import copy
import unittest

from scripts import firewall_packet_capture_evidence as evidence


def valid_capture_result() -> dict:
    return {
        "schema_version": 1,
        "fixture": "isolated-user-network-namespace",
        "namespace_sha256": "a" * 64,
        "interfaces": [
            {"name": "am-egress0", "up": True, "loopback": False},
            {"name": "lo", "up": True, "loopback": True},
        ],
        "routes": [
            {
                "destination": "192.0.2.0/24",
                "device": "am-egress0",
                "scope": "link",
            }
        ],
        "firewall_policy_sha256": evidence.harness.canonical_sha256(
            evidence.harness.POLICY_SPEC
        ),
        "firewall_policy_exact": True,
        "loopback_allowed": True,
        "loopback_captured_frames": 4,
        "loopback_captured_bytes": 400,
        "synthetic_dns_attempted": True,
        "synthetic_dns_send_refused": True,
        "synthetic_egress_frames": 0,
        "synthetic_egress_bytes": 0,
        "firewall_drop_packets": 1,
        "firewall_drop_bytes": 48,
        "packet_payload_retained": False,
        "external_network_used": False,
    }


class FirewallPacketCaptureEvidenceTests(unittest.TestCase):
    def valid_report(self) -> dict:
        return {
            "schema_version": 1,
            "artifact_id": "strict-local-isolated-firewall-packet-capture",
            "source_revision": "a" * 40,
            "task_ids": ["10.1.2.3"],
            "status": "pass-isolated-linux-firewall-and-capture-harness",
            "harness_profile": copy.deepcopy(evidence.HARNESS_PROFILE),
            "capture_result": valid_capture_result(),
            "verification_commands": evidence.expected_commands(),
            "claims": copy.deepcopy(evidence.CLAIMS),
            "limitations": list(evidence.LIMITATIONS),
            "private_user_data_used": False,
            "external_network_used": False,
            "sources": [
                {"path": path, "bytes": 1, "sha256": "b" * 64}
                for path in evidence.SOURCE_PATHS
            ],
        }

    def test_exact_report_is_valid(self) -> None:
        self.assertEqual(evidence.validate_report(self.valid_report()), [])

    def test_profile_command_or_capture_mutation_fails(self) -> None:
        profile = self.valid_report()
        profile["harness_profile"]["default_route_count"] = 1
        self.assertIn(
            "firewall capture harness_profile changed",
            evidence.validate_report(profile),
        )
        command = self.valid_report()
        command["verification_commands"].pop()
        self.assertIn(
            "firewall capture verification_commands changed",
            evidence.validate_report(command),
        )
        capture = self.valid_report()
        capture["capture_result"]["synthetic_egress_frames"] = 1
        self.assertIn(
            "capture result changed: synthetic_egress_frames",
            evidence.validate_report(capture),
        )

    def test_claim_or_limitation_overstatement_fails(self) -> None:
        claim = self.valid_report()
        claim["claims"]["product_workflow_observed"] = True
        self.assertIn("firewall capture claims changed", evidence.validate_report(claim))
        limitation = self.valid_report()
        limitation["limitations"].pop()
        self.assertIn(
            "firewall capture limitations changed",
            evidence.validate_report(limitation),
        )

    def test_source_and_revision_mutation_fails(self) -> None:
        source = self.valid_report()
        source["sources"].pop()
        self.assertIn(
            "firewall capture source evidence changed",
            evidence.validate_report(source),
        )
        revision = self.valid_report()
        revision["source_revision"] = "HEAD"
        self.assertIn(
            "firewall capture source revision is invalid",
            evidence.validate_report(revision),
        )

    def test_private_data_or_external_network_mutation_fails(self) -> None:
        private = self.valid_report()
        private["private_user_data_used"] = True
        self.assertIn(
            "firewall capture private_user_data_used changed",
            evidence.validate_report(private),
        )
        network = self.valid_report()
        network["external_network_used"] = True
        self.assertIn(
            "firewall capture external_network_used changed",
            evidence.validate_report(network),
        )


if __name__ == "__main__":
    unittest.main()
