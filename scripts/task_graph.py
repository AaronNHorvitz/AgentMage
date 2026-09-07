#!/usr/bin/env python3
"""Validate governed story identities, dependencies, and release applicability."""

from __future__ import annotations

import re
import json
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
DECISION_0051_STORIES = ("25.3", "76.2", "76.3", "77.2")
DECISION_STORIES = (
    DECISION_0042_STORIES + DECISION_0043_0044_STORIES + DECISION_0051_STORIES
)
RELEASE_APPLICABILITY_PATH = ROOT / "architecture" / "release-applicability.json"
EXPECTED_STORY_APPLICABILITY = {
    "25.3": ["v1.0-preview-windows"],
    "76.2": ["v1.0-preview-windows", "v1.0-full-ga"],
    "76.3": ["v1.0-preview-windows", "v1.0-full-ga"],
    "77.2": ["v1.0-preview-windows", "v1.0-full-ga"],
}
EXPECTED_NETWORK_PHASES = {
    "normal-question-answering": "denied",
    "model-acquisition": "separate-explicit-consent",
    "signed-update-retrieval": "separate-explicit-consent",
    "diagnostic-transmission": "explicit-per-report-consent-after-local-redaction",
}

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


def validate_applicability(value: object, story_set: set[str]) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict):
        return ["release applicability must be an object"]
    if value.get("schema_version") != 1 or value.get("decision_id") != "ADR-0051":
        failures.append("release applicability must bind schema 1 and ADR-0051")

    milestones = value.get("milestones")
    expected_milestones = {
        "v1.0-preview-windows",
        "v1.0-full-ga",
        "retained-platforms",
    }
    if not isinstance(milestones, dict) or set(milestones) != expected_milestones:
        failures.append("release applicability milestone set is incomplete")
        milestones = {}
    expected_gates = {
        "v1.0-preview-windows": "25.3",
        "v1.0-full-ga": "166.1",
        "retained-platforms": "25.1",
    }
    for milestone_id, gate in expected_gates.items():
        record = milestones.get(milestone_id, {})
        if not isinstance(record, dict) or record.get("release_gate_story_id") != gate:
            failures.append(f"{milestone_id} release gate must be Story {gate}")
            continue
        required = record.get("required_story_ids")
        if not isinstance(required, list) or any(item not in story_set for item in required):
            failures.append(f"{milestone_id} has an unknown required story")

    if value.get("story_applicability") != EXPECTED_STORY_APPLICABILITY:
        failures.append("Decision 0051 story applicability differs from the accepted matrix")
    if value.get("source_document_write_policy") != "denied":
        failures.append("preview source-document writes must remain denied")
    if value.get("authorized_encrypted_application_state") is not True:
        failures.append("authorized encrypted application state must remain explicit")
    if value.get("network_phases") != EXPECTED_NETWORK_PHASES:
        failures.append("preview network phases or consent boundaries differ")
    return failures


def validate_text(
    tasks: str,
    plan: str,
    architecture: str,
    applicability: object | None = None,
) -> list[str]:
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
        ("Decisions 0043-0045", DECISION_0043_0044_STORIES),
        ("Decision 0051", DECISION_0051_STORIES),
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
        failures.append("governed story dependency cycle: " + " -> ".join(cycle))

    if applicability is not None:
        failures.extend(validate_applicability(applicability, story_set))

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
    applicability = json.loads(RELEASE_APPLICABILITY_PATH.read_text(encoding="utf-8"))
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
                "architecture/release-applicability.json",
            )
        ),
        applicability,
    )
    if failures:
        for failure in failures:
            print(f"task graph error: {failure}", file=sys.stderr)
        return 1
    print(
        "task graph validation passed: "
        f"{len(DECISION_0042_STORIES)} Decision 0042 stories, "
        f"{len(DECISION_0043_0044_STORIES)} Decisions 0043-0045 stories, "
        f"{len(DECISION_0051_STORIES)} Decision 0051 stories, "
        "3 release applicability classes, 4 foundational runtime epics"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
