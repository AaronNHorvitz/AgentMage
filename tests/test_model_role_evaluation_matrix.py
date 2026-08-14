from __future__ import annotations

import copy
import importlib.util
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    "model_role_evaluation_matrix", ROOT / "scripts/model_role_evaluation_matrix.py"
)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class ModelRoleEvaluationMatrixTests(unittest.TestCase):
    def test_complete_frozen_population_and_muse_result_reconcile(self) -> None:
        report = MODULE.build()
        self.assertEqual(report["candidate_count"], 416)
        self.assertEqual(report["candidate_dispositions"], {"BLOCKED": 415, "INELIGIBLE": 1})
        self.assertEqual(report["enabled_model_count"], 0)
        self.assertEqual(report["exact_profile_results"][0]["disposition"], "REJECTED")
        self.assertFalse(report["exact_profile_results"][0]["source_inventory_result_reused"])
        MODULE.validate(report)

    def test_every_inventory_role_and_suite_is_attributable(self) -> None:
        inventory = MODULE.load(MODULE.ROLE_MATRIX)
        report = MODULE.build()
        expected = {
            entry["entry_id"]: (entry["roles"], entry["applicable_suites"])
            for entry in inventory["entries"]
        }
        actual = {
            entry["entry_id"]: (entry["roles"], entry["applicable_suites"])
            for entry in report["source_candidate_results"]
        }
        self.assertEqual(actual, expected)

    def test_omission_role_escalation_and_borrowed_results_fail(self) -> None:
        for mutate in (
            lambda value: value["source_candidate_results"].pop(),
            lambda value: value["source_candidate_results"][0]["roles"].append("admin"),
            lambda value: value["source_candidate_results"][0].update({"borrowed_result": True}),
        ):
            report = MODULE.build()
            mutate(report)
            with self.assertRaises(ValueError):
                MODULE.validate(report)

    def test_routing_and_unsupported_claim_mutations_fail(self) -> None:
        mutations = (
            lambda value: value.update({"enabled_model_count": 1}),
            lambda value: value.update({"automatic_fallback": True}),
            lambda value: value["claims"].update({"universal_determinism": True}),
            lambda value: value["source_candidate_results"][0].update({"selectable": True}),
        )
        for mutate in mutations:
            report = MODULE.build()
            mutate(report)
            with self.assertRaises(ValueError):
                MODULE.validate(report)

    def test_matrix_hash_changes_for_every_material_field(self) -> None:
        report = MODULE.build()
        original = report["matrix_sha256"]
        unsigned = copy.deepcopy(report)
        unsigned.pop("matrix_sha256")
        unsigned["admission_thresholds"]["minimum_trials"] = 6
        self.assertNotEqual(MODULE.canonical_sha256(unsigned), original)


if __name__ == "__main__":
    unittest.main()
