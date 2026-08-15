from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import v0_4_coding_release_gate as gate


class V04CodingReleaseGateTests(unittest.TestCase):
    def test_canonical_candidate_is_current_and_non_releasing(self) -> None:
        manifest = gate.build_manifest()
        report = gate.build_report()
        self.assertEqual(gate.validate_manifest(manifest), [])
        self.assertEqual(gate.validate_report(report), [])
        self.assertTrue(report["local_manifest_valid"])
        self.assertTrue(report["coding_skill_contracts_passed"])
        self.assertFalse(report["gate_closed"])
        self.assertFalse(report["release_allowed"])

    def test_every_release_and_evidence_overclaim_fails(self) -> None:
        fields = (
            "coding_coordinator_integrated",
            "native_chat_workflow_complete",
            "native_cli_workflow_complete",
            "cross_interface_parity_complete",
            "admitted_live_model_available",
            "fedora_acceptance",
            "ubuntu_acceptance",
            "windows_acceptance",
            "lifecycle_accessibility_recovery_complete",
            "independent_review_complete",
            "manual_fuzzing_complete",
            "package_signing_allowed",
            "gate_closed",
            "release_allowed",
        )
        for field in fields:
            changed = copy.deepcopy(gate.build_report())
            changed[field] = True
            self.assertTrue(gate.validate_report(changed), field)

    def test_manifest_registration_source_exclusion_and_blocker_drift_fails(self) -> None:
        mutations = (
            lambda value: value.update({"gate_closed": True}),
            lambda value: value.update({"product_registration": True}),
            lambda value: value.update({"coding_coordinator_integrated": True}),
            lambda value: value.update({"authenticated_cli_transport": True}),
            lambda value: value["coding_skills"].pop(),
            lambda value: value["excluded_capabilities"].pop(),
            lambda value: value["source_files"][0].update({"sha256": "0" * 64}),
            lambda value: value["blockers"].pop(),
        )
        for mutate in mutations:
            changed = copy.deepcopy(gate.build_manifest())
            mutate(changed)
            self.assertTrue(gate.validate_manifest(changed))


if __name__ == "__main__":
    unittest.main()
