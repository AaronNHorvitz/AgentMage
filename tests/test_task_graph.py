from __future__ import annotations

import unittest

from scripts.task_graph import DECISION_STORIES, validate_text


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


if __name__ == "__main__":
    unittest.main()
