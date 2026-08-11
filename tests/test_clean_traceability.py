from __future__ import annotations

import copy
import json
import unittest

from scripts.verify_clean_traceability import (
    ROOT,
    CleanTraceabilityError,
    verify_clean_checkout,
    verify_traceability_closure,
)


class CleanTraceabilityTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.report = json.loads(
            (ROOT / "requirements" / "traceability-report.json").read_text(
                encoding="utf-8"
            )
        )
        cls.normative_map = json.loads(
            (ROOT / "requirements" / "normative-map.json").read_text(encoding="utf-8")
        )

    def test_complete_normative_chain_passes(self) -> None:
        summary = verify_traceability_closure(self.report, self.normative_map)

        self.assertEqual(summary["normative_statements"], 26)
        self.assertGreater(summary["resolved_requirement_links"], 26)
        self.assertEqual(summary["traceability_records"], 227)

    def test_orphan_requirement_and_test_are_rejected(self) -> None:
        orphan_requirement = copy.deepcopy(self.normative_map)
        orphan_requirement["mappings"][0]["requirement_ids"] = ["AM-MISSING-001"]
        with self.assertRaisesRegex(CleanTraceabilityError, "orphan requirement"):
            verify_traceability_closure(self.report, orphan_requirement)

        orphan_test = copy.deepcopy(self.report)
        product = next(
            record for record in orphan_test["requirements"]
            if record["normative_statements"]
        )
        product["acceptance_tests"] = ["AT-MISSING-001"]
        with self.assertRaisesRegex(CleanTraceabilityError, "orphan acceptance test"):
            verify_traceability_closure(orphan_test, self.normative_map)

    def test_missing_issue_release_status_or_evidence_is_rejected(self) -> None:
        mutations = {
            "planned issue": lambda record: record["implementation"].update(
                {"planning_items": []}
            ),
            "release or status": lambda record: record.update({"release": ""}),
            "evidence state": lambda record: record.update({"evidence": {}}),
        }
        for expected, mutate in mutations.items():
            with self.subTest(field=expected):
                changed = copy.deepcopy(self.report)
                product = next(
                    record for record in changed["requirements"]
                    if record["normative_statements"]
                )
                mutate(product)
                with self.assertRaisesRegex(CleanTraceabilityError, expected):
                    verify_traceability_closure(changed, self.normative_map)

    def test_verification_does_not_mutate_documents(self) -> None:
        report_before = copy.deepcopy(self.report)
        map_before = copy.deepcopy(self.normative_map)

        verify_traceability_closure(self.report, self.normative_map)

        self.assertEqual(self.report, report_before)
        self.assertEqual(self.normative_map, map_before)

    def test_committed_head_rebuilds_in_an_isolated_checkout(self) -> None:
        result = verify_clean_checkout("HEAD")

        self.assertTrue(result["ok"])
        self.assertTrue(all(item["exit_code"] == 0 for item in result["commands"]))
        self.assertEqual(result["closure"]["normative_statements"], 26)


if __name__ == "__main__":
    unittest.main()
