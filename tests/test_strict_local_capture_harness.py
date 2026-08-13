"""Tests for the isolated firewall and packet-capture acceptance harness."""

from __future__ import annotations

import copy
import unittest

from scripts import strict_local_capture_harness as harness


class StrictLocalCaptureHarnessTests(unittest.TestCase):
    def valid_result(self) -> dict:
        return {
            "schema_version": 1,
            "fixture": "isolated-user-network-namespace",
            "namespace_sha256": "a" * 64,
            "interfaces": [
                {"name": harness.EGRESS_INTERFACE, "up": True, "loopback": False},
                {"name": "lo", "up": True, "loopback": True},
            ],
            "routes": [
                {
                    "destination": "192.0.2.0/24",
                    "device": harness.EGRESS_INTERFACE,
                    "scope": "link",
                }
            ],
            "firewall_policy_sha256": harness.canonical_sha256(harness.POLICY_SPEC),
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

    def test_exact_result_is_valid(self) -> None:
        self.assertEqual(harness.validate_result(self.valid_result()), [])

    def test_namespace_interface_and_route_mutations_fail(self) -> None:
        extra = self.valid_result()
        extra["packet_payload"] = "forbidden"
        self.assertIn("capture result fields changed", harness.validate_result(extra))
        namespace = self.valid_result()
        namespace["namespace_sha256"] = "bad"
        self.assertIn(
            "capture result invalid: namespace_sha256",
            harness.validate_result(namespace),
        )
        interface = self.valid_result()
        interface["interfaces"].append(
            {"name": "unexpected", "up": True, "loopback": False}
        )
        self.assertIn("capture result changed: interfaces", harness.validate_result(interface))
        route = self.valid_result()
        route["routes"][0]["destination"] = "default"
        self.assertIn("capture result changed: routes", harness.validate_result(route))

    def test_firewall_policy_mutations_fail(self) -> None:
        identity = self.valid_result()
        identity["firewall_policy_sha256"] = "b" * 64
        self.assertIn(
            "capture result changed: firewall_policy_sha256",
            harness.validate_result(identity),
        )
        exact = self.valid_result()
        exact["firewall_policy_exact"] = False
        self.assertIn(
            "capture result changed: firewall_policy_exact",
            harness.validate_result(exact),
        )

    def test_missing_loopback_capture_fails(self) -> None:
        for key in ("loopback_captured_frames", "loopback_captured_bytes"):
            changed = self.valid_result()
            changed[key] = 0
            self.assertIn(
                f"capture result invalid: {key}", harness.validate_result(changed)
            )

    def test_any_synthetic_egress_frame_or_byte_fails(self) -> None:
        for key in ("synthetic_egress_frames", "synthetic_egress_bytes"):
            changed = self.valid_result()
            changed[key] = 1
            self.assertIn(
                f"capture result changed: {key}", harness.validate_result(changed)
            )
        refusal = self.valid_result()
        refusal["synthetic_dns_send_refused"] = False
        self.assertIn(
            "capture result changed: synthetic_dns_send_refused",
            harness.validate_result(refusal),
        )

    def test_missing_drop_counter_or_retained_payload_fails(self) -> None:
        for key in ("firewall_drop_packets", "firewall_drop_bytes"):
            changed = self.valid_result()
            changed[key] = 0
            self.assertIn(
                f"capture result invalid: {key}", harness.validate_result(changed)
            )
        payload = self.valid_result()
        payload["packet_payload_retained"] = True
        self.assertIn(
            "capture result changed: packet_payload_retained",
            harness.validate_result(payload),
        )

    def test_policy_identity_is_deterministic_and_fact_bound(self) -> None:
        baseline = harness.canonical_sha256(harness.POLICY_SPEC)
        self.assertEqual(baseline, harness.canonical_sha256(harness.POLICY_SPEC))
        changed = copy.deepcopy(harness.POLICY_SPEC)
        changed["policy"] = "accept"
        self.assertNotEqual(baseline, harness.canonical_sha256(changed))


if __name__ == "__main__":
    unittest.main()
