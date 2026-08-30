from __future__ import annotations

import copy
import unittest

from scripts.workflow_rv52_evidence import (
    LINEAGE_MARKERS,
    expected_lineage,
    expected_report,
    validate_lineage,
    validate_raw,
    validate_report,
)


class WorkflowRv52EvidenceTests(unittest.TestCase):
    def test_lineage_is_complete_unique_and_authority_free(self) -> None:
        lineage = expected_lineage()
        self.assertEqual(validate_lineage(lineage), [])
        self.assertEqual([node["kind"] for node in lineage["nodes"]], list(LINEAGE_MARKERS))
        self.assertFalse(lineage["execution_truth"]["runtime_effect_executed"])
        self.assertFalse(lineage["execution_truth"]["lineage_is_runtime_receipt"])

    def test_reordered_missing_or_reused_lineage_is_rejected(self) -> None:
        changed = copy.deepcopy(expected_lineage())
        changed["nodes"].reverse()
        self.assertTrue(validate_lineage(changed))
        changed = copy.deepcopy(expected_lineage())
        changed["nodes"].pop()
        self.assertTrue(validate_lineage(changed))
        changed = copy.deepcopy(expected_lineage())
        changed["nodes"][1]["synthetic_identity"] = changed["nodes"][0]["synthetic_identity"]
        self.assertTrue(validate_lineage(changed))

    def test_report_keeps_complete_protocol_and_acceptance_open(self) -> None:
        report = expected_report()
        self.assertEqual(validate_report(report), [])
        self.assertFalse(report["protocol_complete"])
        self.assertEqual(report["status"], "PARTIAL_LOCAL_CONTRACT_EVIDENCE")
        self.assertTrue(all(item["status"] == "NOT_EXECUTED_OWNER_OPEN" for item in report["acceptance_tests"]))
        self.assertEqual(len(report["remaining_protocol_scenarios"]), 5)

    def test_full_pass_runtime_effect_or_later_acceptance_overclaim_is_rejected(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["protocol_complete"] = True
        changed["status"] = "PASS"
        self.assertTrue(validate_report(changed))
        changed = copy.deepcopy(expected_report())
        changed["product_truth"]["runtime_effect_executed"] = True
        self.assertTrue(validate_report(changed))
        changed = copy.deepcopy(expected_report())
        changed["acceptance_tests"][0]["status"] = "PASS"
        self.assertTrue(validate_report(changed))

    def test_raw_results_require_every_lineage_marker_and_no_failure(self) -> None:
        valid = "\n".join(LINEAGE_MARKERS.values())
        self.assertEqual(validate_raw(valid), [])
        self.assertTrue(validate_raw(valid.replace(next(iter(LINEAGE_MARKERS.values())), "")))
        self.assertTrue(validate_raw(f"{valid}\ntest result: FAILED"))


if __name__ == "__main__":
    unittest.main()
