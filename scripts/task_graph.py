#!/usr/bin/env python3
"""Validate Decisions 0042-0044 task identities, dependencies, and roadmap coverage."""

from __future__ import annotations

import re
import sys
from collections import Counter
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DECISION_0042_STORIES = (
    "1.2",
    "2.3",
    "5.2",
    "11.2",
    "13.4",
    "16.2",
    "16.3",
    "21.3",
    "22.3",
    "22.4",
    "23.5",
    "23.6",
    "50.3",
    "58.2",
    "60.2",
    "62.2",
    "81.2",
)
DECISION_0043_0044_STORIES = (
    "1.3",
    "2.4",
    "5.3",
    "11.3",
    "13.5",
    "13.6",
    "16.4",
    "21.4",
    "22.5",
    "23.7",
    "23.8",
    "49.2",
    "50.4",
    "95.3",
    "95.4",
    "121.2",
    "123.2",
    "124.2",
    "125.3",
    "126.2",
)
DECISION_STORIES = DECISION_0042_STORIES + DECISION_0043_0044_STORIES

STORY = re.compile(r"^#### \[[ xX]\] Story (\d+\.\d+)\b", re.MULTILINE)
TASK = re.compile(r"^- \[[ xX]\] \*\*Task (\d+\.\d+\.\d+)\b", re.MULTILINE)
SUBTASK = re.compile(
    r"^  - \[[ xX]\] \*\*Sub-task (\d+\.\d+\.\d+\.\d+)\b", re.MULTILINE
)
DEPENDENCY = re.compile(r"\b(\d+\.\d+)\b")


def _duplicates(values: list[str]) -> list[str]:
    return sorted(value for value, count in Counter(values).items() if count > 1)


def _story_blocks(text: str) -> dict[str, str]:
    matches = list(STORY.finditer(text))
    return {
        match.group(1): text[
            match.start() : matches[index + 1].start() if index + 1 < len(matches) else len(text)
        ]
        for index, match in enumerate(matches)
    }


def _cycle(graph: dict[str, set[str]]) -> list[str]:
    visiting: set[str] = set()
    visited: set[str] = set()
    path: list[str] = []

    def visit(node: str) -> list[str]:
        if node in visiting:
            start = path.index(node)
            return path[start:] + [node]
        if node in visited:
            return []
        visiting.add(node)
        path.append(node)
        for dependency in sorted(graph.get(node, set())):
            found = visit(dependency)
            if found:
                return found
        path.pop()
        visiting.remove(node)
        visited.add(node)
        return []

    for node in sorted(graph):
        found = visit(node)
        if found:
            return found
    return []


def validate_text(tasks: str, plan: str, architecture: str) -> list[str]:
    failures: list[str] = []
    story_ids = STORY.findall(tasks)
    task_ids = TASK.findall(tasks)
    subtask_ids = SUBTASK.findall(tasks)

    for label, values in (("story", story_ids), ("task", task_ids), ("sub-task", subtask_ids)):
        duplicates = _duplicates(values)
        if duplicates:
            failures.append(f"duplicate {label} ids: {', '.join(duplicates)}")

    story_set = set(story_ids)
    for label, expected_stories in (
        ("Decision 0042", DECISION_0042_STORIES),
        ("Decisions 0043-0044", DECISION_0043_0044_STORIES),
    ):
        missing = sorted(set(expected_stories) - story_set)
        if missing:
            failures.append(f"missing {label} stories: {', '.join(missing)}")

    for task_id in task_ids:
        if ".".join(task_id.split(".")[:2]) not in story_set:
            failures.append(f"task {task_id} has no owning story")
    for subtask_id in subtask_ids:
        if ".".join(subtask_id.split(".")[:3]) not in set(task_ids):
            failures.append(f"sub-task {subtask_id} has no owning task")

    blocks = _story_blocks(tasks)
    graph: dict[str, set[str]] = {}
    for story_id in DECISION_STORIES:
        block = blocks.get(story_id, "")
        dependency_text = block.split("**Dependencies:**", 1)
        if len(dependency_text) != 2:
            failures.append(f"Story {story_id} has no explicit dependency record")
            continue
        declaration = dependency_text[1].split("\n\n", 1)[0]
        dependencies = set(DEPENDENCY.findall(declaration))
        unknown = sorted(dependencies - story_set)
        if unknown:
            failures.append(f"Story {story_id} has unknown dependencies: {', '.join(unknown)}")
        graph[story_id] = dependencies.intersection(DECISION_STORIES)

        if story_id not in plan:
            failures.append(f"Story {story_id} is missing from IMPLEMENTATION-PLAN.md")
        if story_id not in architecture:
            failures.append(f"Story {story_id} is missing from the foundational architecture")

    cycle = _cycle(graph)
    if cycle:
        failures.append("Decision 0042 story dependency cycle: " + " -> ".join(cycle))

    foundational_headings = re.findall(
        r"^## \[[ xX]\] Foundational Runtime Epic F\d+ - ", tasks, re.MULTILINE
    )
    if len(foundational_headings) != 4:
        failures.append(
            f"expected 4 foundational runtime epic headings, found {len(foundational_headings)}"
        )
    for marker in (
        "FRE-INGEST",
        "FRE-WORKFLOW",
        "FRE-ENGINEERING-RUNTIME",
        "FRE-MODEL-GATEWAY",
        "M-FOUNDATIONAL-RUNTIME-CORE",
        "ER-M0",
        "ER-M9",
    ):
        if marker not in tasks + plan + architecture:
            failures.append(f"missing foundational roadmap marker: {marker}")

    return failures


def main() -> int:
    failures = validate_text(
        (ROOT / "TASKS.md").read_text(encoding="utf-8"),
        (ROOT / "IMPLEMENTATION-PLAN.md").read_text(encoding="utf-8"),
        "\n".join(
            (ROOT / path).read_text(encoding="utf-8")
            for path in (
                "docs/architecture/foundational-artifact-and-workflow-runtime.md",
                "ENGINEERING-RUNTIME.md",
                "MODEL-GATEWAY.md",
                "ENGINEERING-CAPABILITY-REGISTRY.md",
            )
        ),
    )
    if failures:
        for failure in failures:
            print(f"task graph error: {failure}", file=sys.stderr)
        return 1
    print(
        "task graph validation passed: "
        f"{len(DECISION_0042_STORIES)} Decision 0042 stories, "
        f"{len(DECISION_0043_0044_STORIES)} Decisions 0043-0044 stories, "
        "4 foundational runtime epics"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
