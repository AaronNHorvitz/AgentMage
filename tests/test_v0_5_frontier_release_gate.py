from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import v0_5_frontier_release_gate as gate


class V05FrontierReleaseGateTests(unittest.TestCase):
    def manifest(self) -> dict[str, object]:
        with patch.object(gate, "sha256", return_value="a" * 64):
            return gate.build_manifest()

    def report(self) -> dict[str, object]:
        manifest = self.manifest()
        with patch.object(gate, "sha256", return_value="b" * 64), patch.object(
            gate, "build_manifest", return_value=manifest
        ):
            return gate.build_report(manifest)

    def test_manifest_is_manual_strict_local_and_fail_closed(self) -> None:
        manifest = self.manifest()
        with patch.object(gate, "build_manifest", return_value=manifest):
            self.assertEqual(gate.validate_manifest(manifest), [])
        self.assertTrue(manifest["strict_local_mode_required"])
        self.assertTrue(manifest["manual_transfer_required"])
        self.assertFalse(manifest["outbound_network"])
        self.assertFalse(manifest["external_client"])
        self.assertFalse(manifest["import_authority"])
        self.assertFalse(manifest["gate_closed"])

    def test_every_release_authority_and_delivery_overclaim_fails(self) -> None:
        fields = (
            "gate_closed",
            "product_registration",
            "frontier_coordinator_integrated",
            "native_round_trip_complete",
            "package_artifacts_published",
            "signed_release",
            "outbound_network",
            "external_client",
            "credential_access",
            "automatic_delivery",
            "import_authority",
        )
        expected = self.manifest()
        for field in fields:
            changed = copy.deepcopy(expected)
            changed[field] = True
            with patch.object(gate, "build_manifest", return_value=expected):
                self.assertTrue(gate.validate_manifest(changed), field)

    def test_report_cannot_close_any_external_gate(self) -> None:
        expected = self.report()
        with patch.object(gate, "build_report", return_value=expected):
            self.assertEqual(gate.validate_report(expected), [])
        for field in (
            "frontier_coordinator_integrated",
            "native_round_trip_complete",
            "live_model_campaign_complete",
            "fedora_acceptance",
            "ubuntu_acceptance",
            "windows_acceptance",
            "lifecycle_accessibility_privacy_complete",
            "durable_recovery_complete",
            "independent_review_complete",
            "manual_fuzzing_complete",
            "package_signing_allowed",
            "gate_closed",
            "release_allowed",
        ):
            changed = copy.deepcopy(expected)
            changed[field] = True
            with patch.object(gate, "build_report", return_value=expected):
                self.assertTrue(gate.validate_report(changed), field)


if __name__ == "__main__":
    unittest.main()
