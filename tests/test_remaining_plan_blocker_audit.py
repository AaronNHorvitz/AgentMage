from __future__ import annotations

import unittest

from scripts.remaining_plan_blocker_audit import _coding_dependency_ranks, build_from_text


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

    def test_decision_0052_local_work_is_selected_in_critical_order(self) -> None:
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

    def test_registered_serving_work_precedes_later_foundational_unknowns(self) -> None:
        value = build_from_text(
            """#### [ ] Story 13.1 - Model contracts
- [ ] **Task 13.1.5 - Bind serving capabilities**
  - [ ] **Sub-task 13.1.5.1:** Implement observation. **Execution:** local; owner=model-runtime; venue=repository-local.
#### [ ] Story 50.3 - Evaluation
- [ ] **Task 50.3.4 - Evaluate**
  - [ ] **Sub-task 50.3.4.1:** Compare admitted profiles.
"""
        )
        self.assertEqual(value["next_action_kind"], "execute-local")
        self.assertEqual(value["next_action_row_id"], "13.1.5.1")
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

    def test_registered_task_dependencies_override_prose_mentions(self) -> None:
        text = """#### [ ] Story 13.1 - Context
- [ ] **Task 13.1.4 - Register**
  - [ ] **Sub-task 13.1.4.2:** Prove no cycle with Task 13.4.5. **Execution:** local.
#### [ ] Story 13.4 - Dispatch
- [ ] **Task 13.4.5 - Prepare**
  - [ ] **Sub-task 13.4.5.1:** Prepare. **Execution:** local.
"""
        value = build_from_text(
            text,
            {"13.1.4.2": [], "13.4.5.1": ["13.1.4"]},
        )
        self.assertEqual(self.row(value, "13.1.4.2")["prerequisite_row_ids"], [])
        self.assertEqual(
            self.row(value, "13.4.5.1")["prerequisite_row_ids"], ["13.1.4"]
        )

    def test_coding_integration_precedes_unrelated_desktop_work(self) -> None:
        value = build_from_text(
            """#### [ ] Story 48.2 - Coding
- [ ] **Task 48.2.4 - Connect**
  - [ ] **Sub-task 48.2.4.1:** Assess launch. **Execution:** local.
#### [ ] Story 76.2 - Desktop
- [ ] **Task 76.2.1 - Desktop**
  - [ ] **Sub-task 76.2.1.1:** Build shell. **Execution:** local.
"""
        )
        self.assertEqual(value["next_action_row_id"], "48.2.4.1")
        self.assertTrue(value["ready_for_unattended_execution"])

    def test_coding_priority_retains_external_and_unfinished_prerequisites(self) -> None:
        value = build_from_text(
            """#### [ ] Story 48.2 - Coding
- [ ] **Task 48.2.4 - Connect**
  - [ ] **Sub-task 48.2.4.1:** Obtain activation. BLOCKED_EXTERNAL(platform=Linux; action=admit exact trust).
  - [ ] **Sub-task 48.2.4.2:** Connect. Depends on Sub-task 48.2.4.1. **Execution:** local.
  - [ ] **Sub-task 48.2.4.7:** Prepare model plan. **Execution:** local.
"""
        )
        self.assertEqual(value["next_action_row_id"], "48.2.4.7")
        self.assertEqual(self.row(value, "48.2.4.1")["classification"], "external")
        self.assertEqual(self.row(value, "48.2.4.2")["prerequisite_row_ids"], ["48.2.4.1"])
        self.assertTrue(all(row["substitution_set"] == [] for row in value["rows"]))

    def test_coding_prerequisite_assessment_precedes_ready_downstream_work(self) -> None:
        value = build_from_text(
            """#### [ ] Story 13.1 - Prerequisite
- [ ] **Task 13.1.9 - Unknown model contract**
#### [ ] Story 48.2 - Coding
- [ ] **Task 48.2.4 - Connect**
  - [ ] **Sub-task 48.2.4.1:** Connect. Depends on Task 13.1.9. **Execution:** local.
  - [ ] **Sub-task 48.2.4.7:** Prepare. **Execution:** local.
"""
        )
        self.assertEqual(value["next_action_kind"], "assess-unknown")
        self.assertEqual(value["next_action_row_id"], "13.1.9")
        self.assertFalse(value["ready_for_unattended_execution"])

    def test_coding_priority_does_not_promote_a_similar_unapproved_task(self) -> None:
        ranks = _coding_dependency_ranks({"48.2.40.1": [], "48.2.4.1": []})
        self.assertEqual(ranks, {"48.2.4.1": -30})

    def test_coding_prerequisite_cycle_terminates_without_becoming_executable(self) -> None:
        value = build_from_text(
            """#### [ ] Story 48.2 - Coding
- [ ] **Task 48.2.4 - Connect**
  - [ ] **Sub-task 48.2.4.1:** A. Depends on Sub-task 48.2.4.2. **Execution:** local.
  - [ ] **Sub-task 48.2.4.2:** B. Depends on Sub-task 48.2.4.1. **Execution:** local.
"""
        )
        self.assertIsNone(value["next_executable_row_id"])
        self.assertFalse(value["ready_for_unattended_execution"])

    def test_daily_use_does_not_precede_mvp_and_completed_work_is_not_reopened(self) -> None:
        value = build_from_text(
            """#### [ ] Story 48.2 - Coding
- [ ] **Task 48.2.4 - Connect**
  - [x] **Sub-task 48.2.4.1:** Completed assessment.
  - [ ] **Sub-task 48.2.4.2:** Connect. Depends on Sub-task 48.2.4.1. **Execution:** local.
#### [ ] Story 50.2 - Daily
- [ ] **Task 50.2.4 - Reliability**
  - [ ] **Sub-task 50.2.4.1:** Recover. **Execution:** local.
"""
        )
        self.assertEqual(value["next_action_row_id"], "48.2.4.2")
        self.assertNotIn("48.2.4.1", {row["row_id"] for row in value["rows"]})
        self.assertEqual(self.row(value, "48.2.4.2")["prerequisite_row_ids"], [])

    def test_coding_unresolved_prerequisite_cannot_be_selected(self) -> None:
        value = build_from_text(
            """#### [ ] Story 48.2 - Coding
- [ ] **Task 48.2.4 - Connect**
  - [ ] **Sub-task 48.2.4.1:** A. Depends on Task 99.9.9. **Execution:** local.
"""
        )
        self.assertIsNone(value["next_executable_row_id"])
        self.assertEqual(self.row(value, "48.2.4.1")["unresolved_reference_ids"], ["99.9.9"])

    def test_coding_parent_does_not_inherit_its_own_story_closure_gate(self) -> None:
        value = build_from_text(
            """#### [ ] Story 48.2 - Coding
- [ ] **Task 48.2.6 - Verify**
  - [ ] **Sub-task 48.2.6.1:** Verify. **Execution:** local.
##### Story Acceptance Criteria
- [ ] **Story AC 48.2.AC1:** Complete coding.
**Gate decision:** Story 48.2 must close before release.
"""
        )
        self.assertEqual(self.row(value, "48.2.6")["prerequisite_row_ids"], ["48.2.6.1"])
        self.assertEqual(value["next_action_row_id"], "48.2.6.1")

    def test_daily_leaf_does_not_depend_on_parent_from_following_commentary(self) -> None:
        value = build_from_text(
            """#### [ ] Story 50.2 - Daily
- [ ] **Task 50.2.4 - Reliability**
  - [ ] **Sub-task 50.2.4.7:** Review. BLOCKED_EXTERNAL(platform=independent review).
  - [ ] **Sub-task 50.2.4.8:** Record. Depends on Sub-task 50.2.4.7. **Execution:** local.

**Daily-use milestone:** Task 50.2.4 does not close Story 50.2.
"""
        )
        self.assertEqual(self.row(value, "50.2.4.8")["prerequisite_row_ids"], ["50.2.4.7"])
        self.assertIsNone(value["next_executable_row_id"])


if __name__ == "__main__":
    unittest.main()
