from __future__ import annotations

import unittest

from scripts.remaining_plan_blocker_audit import build_from_text


class RemainingPlanBlockerRegisterTests(unittest.TestCase):
    def row(self, value: dict, row_id: str) -> dict:
        return next(row for row in value["rows"] if row["row_id"] == row_id)

    def test_unrelated_sprint_blocker_does_not_block_local_row(self) -> None:
        value = build_from_text(
            """### [ ] Sprint 1 - Test

**Blocked:** unrelated hardware is absent.

#### [ ] Story 1.1 - Local

- [ ] **Task 1.1.1 - Implement**
  - [ ] **Sub-task 1.1.1.1:** Add validator. **Execution:** local; owner=test; venue=repository-local.

#### [ ] Story 1.2 - Native

- [ ] **Task 1.2.1 - Verify**
  - [ ] **Sub-task 1.2.1.1:** Run native test. BLOCKED_EXTERNAL(platform=Windows 11; action=run test; credential=none; payment=none).
"""
        )
        self.assertEqual(self.row(value, "1.1.1.1")["classification"], "local")
        self.assertEqual(self.row(value, "1.2.1.1")["classification"], "external")

    def test_transitive_dependency_paths_are_retained(self) -> None:
        value = build_from_text(
            """#### [ ] Story 1.1 - A
- [ ] **Task 1.1.1 - A** Blocked on Task 2.1.1.
#### [ ] Story 2.1 - B
- [ ] **Task 2.1.1 - B** Blocked on Task 3.1.1.
#### [ ] Story 3.1 - C
- [ ] **Task 3.1.1 - C**
  - [ ] **Sub-task 3.1.1.1:** Implement. **Execution:** local; owner=test; venue=repository-local.
"""
        )
        paths = self.row(value, "1.1.1")["dependency_paths"]
        self.assertIn(["1.1.1", "2.1.1", "3.1.1", "3.1.1.1"], paths)

    def test_branching_cycle_paths_are_bounded_and_deterministic(self) -> None:
        value = build_from_text(
            """#### [ ] Story 1.1 - A
- [ ] **Task 1.1.1 - A** Blocked on Tasks 2.1.1 and 3.1.1.
#### [ ] Story 2.1 - B
- [ ] **Task 2.1.1 - B** Blocked on Task 1.1.1.
#### [ ] Story 3.1 - C
- [ ] **Task 3.1.1 - C** Blocked on Task 2.1.1.
"""
        )
        self.assertEqual(
            self.row(value, "1.1.1")["dependency_paths"],
            [
                ["1.1.1", "2.1.1", "1.1.1"],
                ["1.1.1", "3.1.1", "2.1.1", "1.1.1"],
            ],
        )

    def test_unknown_is_distinct_and_not_executable(self) -> None:
        value = build_from_text(
            """#### [ ] Story 4.1 - Unknown
- [ ] **Task 4.1.1 - Unassessed leaf**
"""
        )
        self.assertEqual(self.row(value, "4.1.1")["classification"], "unknown")
        self.assertIsNone(value["next_executable_row_id"])
        self.assertEqual(value["next_action_kind"], "assess-unknown")
        self.assertFalse(value["ready_for_unattended_execution"])

    def test_decision_0051_local_work_is_selected_in_critical_order(self) -> None:
        value = build_from_text(
            """#### [ ] Story 25.3 - Gate
- [ ] **Task 25.3.1 - Gate**
  - [ ] **Sub-task 25.3.1.1:** Implement. **Execution:** local; owner=gate; venue=repository-local.
#### [ ] Story 76.2 - Shell
- [ ] **Task 76.2.1 - Shell**
  - [ ] **Sub-task 76.2.1.1:** Implement. **Execution:** local; owner=shell; venue=repository-local.
"""
        )
        self.assertEqual(value["next_executable_row_id"], "76.2.1.1")

    def test_codec_correction_precedes_later_foundational_unknowns(self) -> None:
        value = build_from_text(
            """#### [ ] Story 13.3 - Codec
- [ ] **Task 13.3.4 - Correct codec**
  - [ ] **Sub-task 13.3.4.1:** Implement correction. **Execution:** local; owner=codec; venue=repository-local.
#### [ ] Story 50.3 - Evaluation
- [ ] **Task 50.3.4 - Evaluate**
  - [ ] **Sub-task 50.3.4.1:** Compare admitted profiles.
"""
        )
        self.assertEqual(value["next_action_kind"], "execute-local")
        self.assertEqual(value["next_action_row_id"], "13.3.4.1")
        self.assertTrue(value["ready_for_unattended_execution"])

    def test_every_open_row_has_action_owner_venue_and_evidence(self) -> None:
        value = build_from_text(
            """#### [ ] Story 76.2 - Shell
- [ ] **Task 76.2.1 - Shell**
  - [ ] **Sub-task 76.2.1.1:** Implement. **Execution:** local; owner=shell; venue=repository-local.
"""
        )
        for row in value["rows"]:
            self.assertTrue(row["owner"])
            self.assertTrue(row["resolution_action"])
            self.assertTrue(row["execution_venue"])
            self.assertEqual(row["evidence"]["path"], "TASKS.md")
            self.assertEqual(row["substitution_set"], [])
        leaf = self.row(value, "76.2.1.1")
        self.assertEqual(leaf["owner"], "shell")
        self.assertEqual(leaf["execution_venue"], "repository-local")


if __name__ == "__main__":
    unittest.main()
