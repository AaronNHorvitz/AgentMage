from __future__ import annotations

import copy
import unittest

from scripts.engineering_runtime_record_evidence import (
    MARKERS,
    TRUTH,
    expected_report,
    validate_raw,
    validate_report,
)


class EngineeringRuntimeRecordEvidenceTests(unittest.TestCase):
    def test_current_evidence_is_exact_and_fail_closed(self) -> None:
        self.assertEqual(validate_report(expected_report()), [])

    def test_artifact_hash_mutation_is_rejected(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["groups"][0]["artifacts"][0]["sha256"] = "0" * 64
        self.assertIn("stale", validate_report(changed)[0])

    def test_product_truth_cannot_claim_external_or_release_evidence(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["product_truth"]["release_readiness_claimed"] = True
        self.assertIn("widened", validate_report(changed)[0])
        self.assertFalse(TRUTH["native_platform_execution_claimed"])

    def test_raw_results_require_every_marker_and_reject_failure_output(self) -> None:
        valid = "\n".join(MARKERS)
        self.assertEqual(validate_raw(valid), [])
        self.assertTrue(validate_raw(valid.replace(MARKERS[0], "")))
        self.assertTrue(validate_raw(f"{valid}\ntest result: FAILED"))


if __name__ == "__main__":
    unittest.main()
