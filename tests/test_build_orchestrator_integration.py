from __future__ import annotations

import copy
import unittest

from scripts.build_orchestrator_integration import load_report, validate_report


class BuildOrchestratorIntegrationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.report = load_report()

    def test_current_report_is_bound_and_valid(self) -> None:
        self.assertEqual(validate_report(self.report), [])

    def test_identity_status_and_unknown_fields_fail_closed(self) -> None:
        for key, value in (
            ("component_id", "kernel"),
            ("lifecycle_evidence", "implementation"),
            ("status", "release-verified"),
        ):
            changed = copy.deepcopy(self.report)
            changed[key] = value
            self.assertTrue(validate_report(changed), key)
        changed = copy.deepcopy(self.report)
        changed["unknown"] = True
        self.assertTrue(validate_report(changed))

    def test_missing_neighbor_check_and_source_binding_fail_closed(self) -> None:
        changed = copy.deepcopy(self.report)
        changed["neighbors_exercised"].pop()
        self.assertTrue(validate_report(changed))
        changed = copy.deepcopy(self.report)
        changed["checks"].pop("manifest_mutation_refused")
        self.assertTrue(validate_report(changed))
        changed = copy.deepcopy(self.report)
        changed["source_bindings"][0]["sha256"] = "0" * 64
        self.assertTrue(validate_report(changed))
        changed = copy.deepcopy(self.report)
        changed["source_bindings"][0] = "malformed"
        self.assertTrue(validate_report(changed))

    def test_artifact_and_authority_overclaims_fail_closed(self) -> None:
        changed = copy.deepcopy(self.report)
        changed["artifact_sha256"]["deb"] = "short"
        self.assertTrue(validate_report(changed))
        changed = copy.deepcopy(self.report)
        changed["artifact_sha256"]["deb"] = "z" * 64
        self.assertTrue(validate_report(changed))
        for key in (
            "network_authority",
            "platform_support_claim",
            "release_claim",
            "product_integration_claim",
        ):
            changed = copy.deepcopy(self.report)
            changed[key] = "claimed"
            self.assertTrue(validate_report(changed), key)
        changed = copy.deepcopy(self.report)
        changed["retained_private_key_material"] = True
        self.assertTrue(validate_report(changed))


if __name__ == "__main__":
    unittest.main()
