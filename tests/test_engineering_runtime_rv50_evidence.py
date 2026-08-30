from __future__ import annotations

import copy
import unittest

from scripts.engineering_runtime_rv50_evidence import (
    MARKERS,
    expected_report,
    validate_raw,
    validate_report,
)


class EngineeringRuntimeRv50EvidenceTests(unittest.TestCase):
    def test_current_record_is_partial_and_exact(self) -> None:
        report = expected_report()
        self.assertEqual(validate_report(report), [])
        self.assertFalse(report["protocol_complete"])
        self.assertEqual(report["status"], "PARTIAL_LOCAL_CONTRACT_EVIDENCE")

    def test_full_pass_or_later_acceptance_overclaim_is_rejected(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["protocol_complete"] = True
        changed["status"] = "PASS"
        self.assertTrue(validate_report(changed))
        changed = copy.deepcopy(expected_report())
        changed["acceptance_tests"][1]["status"] = "PASS"
        self.assertTrue(validate_report(changed))

    def test_platform_and_runtime_substitution_is_rejected(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["platform_runtime_evidence"][0]["substitutes_for"] = ["windows-installed"]
        self.assertTrue(validate_report(changed))

    def test_every_unexecuted_tuple_and_later_scenario_stays_visible(self) -> None:
        report = expected_report()
        self.assertEqual(len(report["later_runtime_scenarios"]), 5)
        self.assertEqual(
            sum(item["status"] == "BLOCKED_EXTERNAL" for item in report["platform_runtime_evidence"]),
            5,
        )
        self.assertTrue(all(not item["substitutes_for"] for item in report["platform_runtime_evidence"]))

    def test_raw_results_require_all_markers_and_no_failure(self) -> None:
        valid = "\n".join(MARKERS)
        self.assertEqual(validate_raw(valid), [])
        self.assertTrue(validate_raw(valid.replace(MARKERS[0], "")))
        self.assertTrue(validate_raw(f"{valid}\nFAILED (failures=1)"))


if __name__ == "__main__":
    unittest.main()
