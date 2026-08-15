from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import v0_3_write_release_gate as gate


class V03WriteReleaseGateTests(unittest.TestCase):
    def test_canonical_blocked_candidate_is_current_and_non_releasing(self) -> None:
        report = gate.build_report()
        self.assertEqual(gate.validate_report(report), [])
        self.assertTrue(report["local_manifest_valid"])
        self.assertFalse(report["write_profile_active"])
        self.assertFalse(report["package_signing_allowed"])
        self.assertFalse(report["gate_closed"])
        self.assertFalse(report["release_allowed"])

    def test_every_release_and_evidence_overclaim_fails(self) -> None:
        fields = (
            "write_profile_active",
            "unsigned_linux_candidate_exercised",
            "fedora_write_acceptance",
            "ubuntu_write_acceptance",
            "macos_write_acceptance",
            "windows_write_acceptance",
            "upgrade_downgrade_restore_uninstall_complete",
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

    def test_manifest_activation_source_exclusion_and_blocker_drift_fails(self) -> None:
        manifest = gate.read_json(gate.MANIFEST_PATH)
        mutations = (
            lambda value: value.update({"gate_closed": True}),
            lambda value: value.update({"product_registration": True}),
            lambda value: value.update({"package_artifacts_published": True}),
            lambda value: value["capability_delta"].update({
                "effective_additions": ["workspace.write.controlled"]
            }),
            lambda value: value["controlled_primitives"].pop(),
            lambda value: value["required_transaction_stages"].pop(),
            lambda value: value["excluded_capabilities"].pop(),
            lambda value: value["source_files"][0].update({"sha256": "0" * 64}),
            lambda value: value["blockers"].pop(),
        )
        for mutate in mutations:
            changed = copy.deepcopy(manifest)
            mutate(changed)
            with patch.object(gate, "read_json", return_value=changed):
                self.assertTrue(gate.validate_manifest(changed))


if __name__ == "__main__":
    unittest.main()
