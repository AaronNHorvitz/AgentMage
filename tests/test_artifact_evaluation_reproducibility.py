from __future__ import annotations

import copy
import json
import unittest

from scripts import artifact_evaluation_reproducibility as reproducibility


def reseal(value: dict[str, object]) -> dict[str, object]:
    value["report_sha256"] = reproducibility.ZERO_SHA256
    return reproducibility.sealed(value, "report_sha256")


class ArtifactEvaluationReproducibilityTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.report = json.loads(reproducibility.OUTPUT_PATH.read_text(encoding="utf-8"))

    def test_exactly_two_clean_roots_produce_one_identity(self) -> None:
        self.assertEqual(self.report["run_count"], 2)
        self.assertTrue(self.report["byte_identical"])
        self.assertEqual(len({item["output_set_sha256"] for item in self.report["runs"]}), 1)
        self.assertTrue(all(item["root_class"] == "ephemeral_empty_root" for item in self.report["runs"]))

    def test_all_declared_deterministic_outputs_are_hash_bound(self) -> None:
        self.assertEqual(self.report["output_count"], 11)
        self.assertEqual(len(self.report["outputs"]), 11)
        self.assertTrue(all(len(item["sha256"]) == 64 and item["byte_length"] > 0 for item in self.report["outputs"]))

    def test_fixture_golden_summary_and_provenance_roles_are_covered(self) -> None:
        self.assertEqual(
            self.report["category_counts"],
            {"fixture_identity": 4, "golden": 3, "provenance_ledger": 4, "summary": 6},
        )

    def test_sources_are_complete_relative_and_hash_bound(self) -> None:
        self.assertEqual(self.report["source_count"], len(reproducibility.SOURCE_PATHS))
        self.assertTrue(all(reproducibility.safe_relative(item["path"]) for item in self.report["source_provenance"]))
        self.assertTrue(all(len(item["sha256"]) == 64 for item in self.report["source_provenance"]))

    def test_resealed_output_identity_mutation_blocks(self) -> None:
        changed = copy.deepcopy(self.report)
        changed["outputs"][0]["sha256"] = "f" * 64
        failures = reproducibility.validate_report(reseal(changed))
        self.assertTrue(any("differs from clean-root identity" in failure for failure in failures))

    def test_report_never_claims_runtime_or_effect_execution(self) -> None:
        self.assertFalse(self.report["product_runtime_executed"])
        self.assertEqual(self.report["product_gate_claim"], "none")
        self.assertEqual(set(self.report["clean_run_side_effect_contract"].values()), {0})

    def test_checked_report_matches_fresh_clean_root_reproduction(self) -> None:
        self.assertTrue(reproducibility.valid_hash(self.report))
        self.assertEqual(reproducibility.check(), [])


if __name__ == "__main__":
    unittest.main()
