"""Assessment coverage must not manufacture completion or execution authority."""

import hashlib
import json
import unittest
from unittest import mock

from scripts import remaining_plan_blocker_audit as audit


class AmendmentWorkSelectionTests(unittest.TestCase):
    def row(self, report: dict, identity: str) -> dict:
        return next(row for row in report["rows"] if row["row_id"] == identity)

    def test_qualified_checked_reference_remains_an_unresolved_assessment(self) -> None:
        line = "| [ ] | AMR-02.1 | AMR-01 exact native profile | Deliver without AMR-07 activation. |"
        text = "| [x] | AMR-01 | CAP-02 | Existing source | Historical completed row. |\n" + line
        report = audit.build_from_text(text)
        row = self.row(report, "AMR-02.1")
        self.assertEqual(row["classification"], "unknown")
        self.assertEqual(row["prerequisite_row_ids"], [])
        self.assertEqual(row["amendment_assessment"], {
            "dependency_text": "AMR-01 exact native profile",
            "referenced_row_ids": ["AMR-01"],
            "completion_gate_edges_resolved": False,
            "execution_authorized": False,
        })
        self.assertEqual(row["evidence"]["sha256"], hashlib.sha256(line.encode()).hexdigest())
        self.assertEqual((row["evidence"]["start_line"], row["evidence"]["end_line"]), (2, 2))
        self.assertEqual(report["coverage"]["amendment_table_rows"], 2)
        self.assertEqual(report["coverage"]["amendment_open_rows"], 1)
        self.assertEqual(report["next_action_kind"], "assess-unknown")
        self.assertFalse(report["ready_for_unattended_execution"])

    def test_description_words_do_not_establish_local_or_external_dispositions(self) -> None:
        for description in (
            "**Execution:** local; owner=runtime; venue=repository-local.",
            "Test BLOCKED_EXTERNAL(platform=example; action=admit a profile).",
        ):
            report = audit.build_from_text(f"| [ ] | AMR-01.1 | Existing owners | {description} |")
            row = report["rows"][0]
            self.assertEqual(row["classification"], "unknown")
            self.assertEqual(row["substitution_set"], [])
            self.assertEqual(row["execution_venue"], "unassessed")
            self.assertEqual(row["owner"], "amr-01.1")
            self.assertEqual(row["external_fields"], {})
            self.assertIsNone(report["next_executable_row_id"])

    def test_missing_reference_is_retained_without_false_gate_resolution(self) -> None:
        report = audit.build_from_text("| [ ] | AMR-01.1 | AMR-09 native boundary | Assess. |")
        row = report["rows"][0]
        self.assertEqual(row["unresolved_reference_ids"], ["AMR-09"])
        self.assertEqual(row["classification"], "unknown")
        self.assertFalse(report["ready_for_unattended_execution"])

    def test_coding_stays_first_then_package_components_precede_next_package(self) -> None:
        tables = """| [x] | AMR-01 | CAP-02 | Existing source | Complete. |
| [ ] | AMR-02 | CAP-05 | AMR-01 | Assess. |
| [ ] | AMR-01.1 | Existing source | Assess. |
"""
        coding = """#### [ ] Story 48.2 - Coding
- [ ] **Task 48.2.4 - Connect**
  - [ ] **Sub-task 48.2.4.1:** Inventory. **Execution:** local.
"""
        desktop = """#### [ ] Story 76.2 - Shell
- [ ] **Task 76.2.1 - Shell**
  - [ ] **Sub-task 76.2.1.1:** Implement. **Execution:** local.
"""
        self.assertEqual(audit.build_from_text(tables + coding + desktop)["next_action_row_id"], "48.2.4.1")
        report = audit.build_from_text(tables + desktop)
        self.assertEqual(report["next_action_row_id"], "AMR-01.1")
        self.assertEqual(report["next_action_kind"], "assess-unknown")
        self.assertFalse(report["ready_for_unattended_execution"])

    def test_table_description_does_not_reclassify_preceding_legacy_work(self) -> None:
        report = audit.build_from_text("""- [ ] **Task 88.1.1 - Inspect** **Execution:** local.

| [ ] | AMR-01.1 | Existing source | Test BLOCKED_EXTERNAL(platform=example). |
""")
        self.assertEqual(self.row(report, "88.1.1")["classification"], "local")

    def test_duplicate_malformed_and_wrong_table_shapes_refuse(self) -> None:
        valid = "| [ ] | AMR-01.1 | Existing owners | Assess. |"
        for malformed in (
            valid + "\n" + valid.replace("[ ]", "[x]"),
            valid.replace("[ ]", "[?]"),
            valid.replace("AMR-01.1", "AMR-01.01"),
            valid.replace("AMR-01.1", "AMR-01"),
            valid.replace("Existing owners", ""),
            valid.replace("Existing owners", "AMR-02.invalid"),
            valid[:-1],
            valid + " extra |",
        ):
            with self.subTest(malformed=malformed), self.assertRaises(ValueError):
                audit.build_from_text(malformed)

    def test_cycle_and_checked_rows_do_not_create_execution_authority(self) -> None:
        report = audit.build_from_text("""| [ ] | AMR-01.1 | AMR-01.2 | Assess first. |
| [ ] | AMR-01.2 | AMR-01.1 | Assess second. |
| [X] | AMR-02.1 | Existing owners | Historical completed row. |
""")
        self.assertEqual(report["unchecked_row_count"], 2)
        self.assertEqual(report["classification_counts"]["unknown"], 2)
        self.assertTrue(all(not row["amendment_assessment"]["execution_authorized"] for row in report["rows"]))
        self.assertFalse(report["ready_for_unattended_execution"])

    def test_actual_register_covers_every_amendment_row_without_replacing_legacy_counts(self) -> None:
        report = json.loads(audit.build())
        self.assertEqual(report["coverage"]["amendment_table_rows"], 79)
        self.assertEqual(report["unchecked_row_count"],
                         report["coverage"]["legacy_open_rows"] + report["coverage"]["amendment_open_rows"])
        self.assertEqual(report["tasks_sha256"], hashlib.sha256(audit.TASKS.read_bytes()).hexdigest())

    def test_completed_table_rows_are_not_reopened_or_kept_in_another_ledger(self) -> None:
        report = audit.build_from_text("""| [x] | AMR-01 | CAP-02 | Existing source | Completed. |
| [x] | AMR-01.1 | Existing source | Completed. |
- [ ] **Task 76.2.1 - Local work** **Execution:** local.
""")
        self.assertEqual(report["coverage"]["amendment_open_rows"], 0)
        self.assertEqual(report["next_action_row_id"], "76.2.1")
        self.assertEqual(report["next_action_kind"], "execute-local")
        self.assertTrue(all(not row["row_id"].startswith("AMR-") for row in report["rows"]))

    def test_production_build_refuses_silently_missing_or_unregistered_rows(self) -> None:
        text = audit.TASKS.read_text()
        for changed in (
            "\n".join(line for line in text.splitlines() if "| AMR-03.1.3 |" not in line),
            text + "\n| [ ] | AMR-08 | CAP-48 | Existing source | Unregistered. |\n",
        ):
            with mock.patch.object(audit, "TASKS") as tasks, self.assertRaises(ValueError):
                tasks.read_text.return_value = changed
                audit.build()


if __name__ == "__main__":
    unittest.main()
