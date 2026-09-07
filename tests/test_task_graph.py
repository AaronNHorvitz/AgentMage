from __future__ import annotations

import unittest

from scripts.task_graph import (
    DECISION_STORIES,
    EXPECTED_NETWORK_PHASES,
    EXPECTED_STORY_APPLICABILITY,
    validate_applicability,
    validate_text,
)


class TaskGraphTests(unittest.TestCase):
    def test_minimal_valid_graph(self) -> None:
        stories = []
        for index, story in enumerate(DECISION_STORIES):
            dependency = "None" if index == 0 else f"Story {stories[-1][0]}"
            stories.append(
                (
                    story,
                    f"#### [ ] Story {story} - Test\n\n**Dependencies:** {dependency}.\n\n"
                    f"- [ ] **Task {story}.1 - Test**\n"
                    f"  - [ ] **Sub-task {story}.1.1:** Test.\n"
                )
            )
        text = (
            "## [ ] Foundational Runtime Epic F1 - Ingest\n"
            "## [ ] Foundational Runtime Epic F2 - Workflow\n"
            "## [ ] Foundational Runtime Epic F3 - Engineering Runtime\n"
            "## [ ] Foundational Runtime Epic F4 - Model Gateway\n"
            "FRE-INGEST FRE-WORKFLOW FRE-ENGINEERING-RUNTIME "
            "FRE-MODEL-GATEWAY M-FOUNDATIONAL-RUNTIME-CORE ER-M0 ER-M9\n"
            + "".join(value for _, value in stories)
        )
        references = " ".join(story for story, _ in stories)
        self.assertEqual(validate_text(text, references, references), [])

    def test_duplicate_and_cycle_fail(self) -> None:
        text = (
            "## [ ] Foundational Runtime Epic F1 - Ingest\n"
            "## [ ] Foundational Runtime Epic F2 - Workflow\n"
            "FRE-INGEST FRE-WORKFLOW M-FOUNDATIONAL-RUNTIME-CORE\n"
            "#### [ ] Story 1.2 - A\n\n**Dependencies:** Story 2.3.\n\n"
            "- [ ] **Task 1.2.1 - A**\n  - [ ] **Sub-task 1.2.1.1:** A.\n"
            "#### [ ] Story 2.3 - B\n\n**Dependencies:** Story 1.2.\n\n"
            "- [ ] **Task 1.2.1 - Duplicate**\n  - [ ] **Sub-task 1.2.1.1:** Duplicate.\n"
        )
        failures = validate_text(text, "1.2 2.3", "1.2 2.3")
        self.assertTrue(any("duplicate task" in failure for failure in failures))
        self.assertTrue(any("dependency cycle" in failure for failure in failures))

    def test_decision_0051_stories_cannot_be_omitted(self) -> None:
        for story_id in ("25.3", "76.2", "76.3", "77.2"):
            tasks = "".join(
                f"#### [ ] Story {item} - Test\n\n**Dependencies:** None.\n\n"
                for item in DECISION_STORIES
                if item != story_id
            )
            failures = validate_text(tasks, " ".join(DECISION_STORIES), " ".join(DECISION_STORIES))
            self.assertTrue(any(story_id in failure for failure in failures))

    def test_release_applicability_is_closed_and_fail_closed(self) -> None:
        milestones = {
            "v1.0-preview-windows": {
                "release_gate_story_id": "25.3",
                "required_story_ids": ["25.3", "76.2", "76.3", "77.2"],
            },
            "v1.0-full-ga": {
                "release_gate_story_id": "166.1",
                "required_story_ids": ["166.1"],
            },
            "retained-platforms": {
                "release_gate_story_id": "25.1",
                "required_story_ids": ["25.1", "76.1", "77.1"],
            },
        }
        value = {
            "schema_version": 1,
            "decision_id": "ADR-0051",
            "milestones": milestones,
            "story_applicability": EXPECTED_STORY_APPLICABILITY,
            "source_document_write_policy": "denied",
            "authorized_encrypted_application_state": True,
            "network_phases": EXPECTED_NETWORK_PHASES,
        }
        story_set = set(DECISION_STORIES) | {"25.1", "76.1", "77.1", "166.1"}
        self.assertEqual(validate_applicability(value, story_set), [])
        value["milestones"]["v1.0-preview-windows"]["required_story_ids"].append("999.9")
        self.assertTrue(any("unknown required story" in item for item in validate_applicability(value, story_set)))


if __name__ == "__main__":
    unittest.main()
