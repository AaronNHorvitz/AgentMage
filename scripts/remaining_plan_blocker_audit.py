#!/usr/bin/env python3
"""Build a row-scoped dependency and blocker register for open TASKS rows."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from collections import Counter
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
TASKS = ROOT / "TASKS.md"
OUTPUT = ROOT / "docs" / "verification" / "remaining-plan-blocker-audit.json"
ROW = re.compile(
    r"^(?P<indent>\s*)(?:(?P<heading>#{2,4})\s+|(?:-\s+))"
    r"\[(?P<state>[ xX])\]\s+(?P<body>.*)$"
)
IDENTITY = re.compile(
    r"(?:Sub-task|Task|Story AC|Sprint AC|Story|Sprint|Epic)\s+"
    r"(?P<id>(?:F\d+|\d+)(?:\.\d+)*(?:\.AC\d+)?)\b"
)
REFERENCE = re.compile(
    r"\b(?:Sub-task|Task|Story AC|Sprint AC|Story|Sprint)s?\s+"
    r"(\d+\.\d+(?:\.\d+){0,2}(?:\.AC\d+)?)"
)
REFERENCE_LIST = re.compile(
    r"\b(?:Sub-tasks|Tasks|Stories|Sprints)\s+"
    r"((?:\d+\.\d+(?:\.\d+){0,2}(?:\.AC\d+)?(?:,\s*|\s+and\s+|\s+through\s+)?)+)"
)
GDOD = re.compile(r"\b(G-DOD-\d+)\b")
EXTERNAL = re.compile(r"BLOCKED_EXTERNAL", re.IGNORECASE)
DEPENDENCY_WORDING = re.compile(
    r"\b(?:blocked on|depends on|dependency[_ -]blocked|awaiting Decision)\b",
    re.IGNORECASE,
)
LOCAL_EXECUTION = re.compile(r"\*\*Execution:\*\*\s*local\b", re.IGNORECASE)
FIELD = re.compile(
    r"\b(platform|artifact|action|credential|payment|owner|venue)=([^,;)]+)"
)

CRITICAL_STORY_ORDER = {"76.2": 30, "76.3": 31, "77.2": 32, "25.3": 50}
FOUNDATIONAL_PREVIEW_STORIES = {
    "1.2", "2.3", "11.2", "13.3", "13.4", "16.2", "21.3", "22.3", "23.5", "50.3",
    "58.2", "60.2", "62.2", "81.2", "1.3", "2.4", "5.3", "11.3", "16.4",
    "21.4", "22.5", "23.7", "23.8", "50.4", "95.3", "95.4", "121.2",
    "125.3", "126.2", "13.5", "13.6", "49.2", "123.2", "124.2",
}


def _kind(body: str) -> str:
    plain = body.replace("**", "")
    for prefix, kind in (
        ("Foundational Runtime Epic ", "foundational-epic"),
        ("Story AC ", "story-acceptance"),
        ("Sprint AC ", "sprint-acceptance"),
        ("Sub-task ", "subtask"),
        ("Task ", "task"),
        ("Story ", "story"),
        ("Sprint ", "sprint"),
        ("Epic ", "epic"),
    ):
        if plain.startswith(prefix):
            return kind
    if GDOD.search(plain):
        return "universal-control"
    return "row"


def _identity(body: str, line: int) -> str:
    if match := IDENTITY.search(body.replace("**", "")):
        return match.group("id")
    if match := GDOD.search(body):
        return match.group(1)
    return f"line-{line}"


def _story_id(row_id: str) -> str | None:
    match = re.match(r"^(\d+\.\d+)", row_id)
    return match.group(1) if match else None


def _owner(row_id: str, kind: str) -> str:
    story = _story_id(row_id)
    if story:
        return f"story-{story}"
    if kind == "sprint":
        return f"sprint-{row_id}"
    return row_id.lower()


def _row_records(text: str) -> list[dict[str, Any]]:
    lines = text.splitlines()
    matches: list[tuple[int, re.Match[str]]] = []
    for index, line in enumerate(lines):
        if match := ROW.match(line):
            matches.append((index, match))

    records: list[dict[str, Any]] = []
    for position, (start, match) in enumerate(matches):
        end = matches[position + 1][0] if position + 1 < len(matches) else len(lines)
        paragraph = "\n".join(lines[start:end]).rstrip()
        body = match.group("body")
        kind = _kind(body)
        row_id = _identity(body, start + 1)
        records.append(
            {
                "row_id": row_id,
                "line": start + 1,
                "end_line": end,
                "checked": match.group("state").lower() == "x",
                "kind": kind,
                "text": body,
                "source_text": paragraph,
                "source_sha256": hashlib.sha256(paragraph.encode()).hexdigest(),
                "story_id": _story_id(row_id),
            }
        )
    structural_boundaries = {
        "foundational-epic": {"foundational-epic", "epic"},
        "epic": {"epic"},
        "sprint": {"sprint", "epic"},
        "story": {"story", "sprint", "epic"},
        "task": {"task", "story", "sprint", "epic"},
    }
    status_markers = (
        "**Dependencies:**",
        "**Blocked:**",
        "**Dependency state:**",
        "**Current status:**",
        "**Story gate evidence:**",
        "**Gate decision:**",
    )
    for record in records:
        boundary_kinds = structural_boundaries.get(record["kind"])
        if boundary_kinds is None:
            record["dependency_text"] = record["source_text"]
            continue
        end = len(lines)
        for candidate in records:
            if candidate["line"] > record["line"] and candidate["kind"] in boundary_kinds:
                end = candidate["line"] - 1
                break
        block = "\n".join(lines[record["line"] - 1 : end])
        paragraphs = re.split(r"\n\s*\n", block)
        status_text = "\n\n".join(
            paragraph
            for paragraph in paragraphs
            if paragraph.lstrip().startswith(status_markers)
        )
        record["dependency_text"] = record["source_text"] + "\n\n" + status_text
    return records


def _structural_dependencies(
    record: dict[str, Any], records: list[dict[str, Any]]
) -> list[str]:
    kind = record["kind"]
    boundaries = {
        "epic": ({"epic"}, {"sprint"}),
        "sprint": ({"sprint", "epic"}, {"story", "sprint-acceptance"}),
        "story": ({"story", "sprint", "epic"}, {"task", "story-acceptance"}),
        "task": ({"task", "story", "sprint", "epic"}, {"subtask"}),
    }
    if kind == "foundational-epic":
        distributed = record["source_text"].split("**Distributed ownership:**", 1)
        if len(distributed) == 2:
            return re.findall(r"\b\d+\.\d+\b", distributed[1])
        return []
    if kind in boundaries:
        boundary_kinds, direct_kinds = boundaries[kind]
        end = 10**9
        for candidate in records:
            if candidate["line"] > record["line"] and candidate["kind"] in boundary_kinds:
                end = candidate["line"]
                break
        return [
            candidate["row_id"]
            for candidate in records
            if record["line"] < candidate["line"] < end
            and candidate["kind"] in direct_kinds
            and not candidate["checked"]
        ]
    if kind == "story-acceptance" and record.get("story_id"):
        story = record["story_id"]
        return [
            candidate["row_id"]
            for candidate in records
            if candidate.get("story_id") == story
            and candidate["kind"] == "task"
            and not candidate["checked"]
        ]
    if kind == "sprint-acceptance":
        sprint = record["row_id"].split(".", 1)[0]
        return [
            candidate["row_id"]
            for candidate in records
            if candidate["kind"] == "story"
            and candidate["row_id"].split(".", 1)[0] == sprint
            and not candidate["checked"]
        ]
    if kind == "universal-control":
        return [
            candidate["row_id"]
            for candidate in records
            if candidate["kind"] == "epic" and not candidate["checked"]
        ]
    return []


def _references(record: dict[str, Any], known: set[str]) -> tuple[list[str], list[str]]:
    source = record.get("dependency_text", record["source_text"])
    found = set(REFERENCE.findall(source))
    for values in REFERENCE_LIST.findall(source):
        found.update(re.findall(r"\d+\.\d+(?:\.\d+){0,2}(?:\.AC\d+)?", values))
    found.discard(record["row_id"])
    return (
        sorted(item for item in found if item in known),
        sorted(item for item in found if item not in known),
    )


def _paths(
    start: str, graph: dict[str, list[str]], *, max_depth: int = 64
) -> list[list[str]]:
    """Return one bounded dependency witness for every direct prerequisite.

    Every row retains its complete direct prerequisite set separately. A single
    deterministic witness per direct edge is sufficient to demonstrate its
    transitive chain without enumerating the exponentially many paths in the
    complete planning graph. Repeated nodes terminate a witness and expose the
    cycle; the depth bound fails closed on unexpectedly deep chains.
    """
    paths: list[list[str]] = []
    for direct_dependency in graph.get(start, []):
        path = [start, direct_dependency]
        seen = {start, direct_dependency}
        current = direct_dependency
        while len(path) < max_depth:
            dependencies = sorted(graph.get(current, []))
            if not dependencies:
                break
            unseen = [dependency for dependency in dependencies if dependency not in seen]
            if not unseen:
                path.append(dependencies[0])
                break
            current = unseen[0]
            path.append(current)
            seen.add(current)
        paths.append(path)
    return paths


def _critical_rank(record: dict[str, Any]) -> tuple[int, int]:
    story = record.get("story_id")
    if story in CRITICAL_STORY_ORDER:
        return CRITICAL_STORY_ORDER[story], record["line"]
    if story:
        sprint = int(story.split(".", 1)[0])
        if sprint == 0:
            return 0, record["line"]
        if story in FOUNDATIONAL_PREVIEW_STORIES:
            return 10, record["line"]
        if 4 <= sprint <= 25:
            return 20, record["line"]
        if 103 <= sprint <= 126:
            return 40, record["line"]
    if record["row_id"] in {"F1", "F3", "F4"}:
        return 10, record["line"]
    return 60, record["line"]


def build_from_text(text: str) -> dict[str, Any]:
    records = _row_records(text)
    known = {record["row_id"] for record in records}
    open_records = [record for record in records if not record["checked"]]
    open_ids = {record["row_id"] for record in open_records}

    graph: dict[str, list[str]] = {}
    unresolved_by_id: dict[str, list[str]] = {}
    for record in open_records:
        referenced, unresolved = _references(record, known)
        structural = _structural_dependencies(record, records)
        graph[record["row_id"]] = sorted((set(referenced) | set(structural)) & open_ids)
        unresolved_by_id[record["row_id"]] = unresolved

    rows: list[dict[str, Any]] = []
    for record in open_records:
        source = record["source_text"]
        classification_source = record.get("dependency_text", source)
        fields = {
            key: value.strip().strip("`").rstrip(".")
            for key, value in FIELD.findall(source)
        }
        unresolved = unresolved_by_id[record["row_id"]]
        dependencies = graph[record["row_id"]]
        structural = record["kind"] in {
            "foundational-epic", "epic", "sprint", "story", "task",
            "story-acceptance", "sprint-acceptance", "universal-control",
        }
        if structural and dependencies:
            classification = "dependency"
            action = "satisfy the exact prerequisite rows before evaluating this row"
            venue = "dependency-defined"
        elif EXTERNAL.search(classification_source):
            classification = "external"
            action = fields.get("action", "perform the exact external action recorded by this row")
            venue = fields.get("platform", "external venue recorded by this row")
        elif LOCAL_EXECUTION.search(classification_source):
            classification = "local"
            action = "implement and verify this exact row"
            venue = "repository-local"
        elif DEPENDENCY_WORDING.search(classification_source) or dependencies:
            classification = "dependency"
            action = "satisfy the exact prerequisite rows before evaluating this row"
            venue = "dependency-defined"
        else:
            classification = "unknown"
            action = "assess and record exact prerequisites before execution or blockage"
            venue = "unassessed"
        if unresolved and classification not in {"external", "local"}:
            classification = "unknown"
            action = "resolve referenced identities and record exact prerequisites"

        rows.append(
            {
                "row_id": record["row_id"],
                "line": record["line"],
                "kind": record["kind"],
                "text": record["text"],
                "classification": classification,
                "prerequisite_row_ids": dependencies,
                "unresolved_reference_ids": unresolved,
                "dependency_paths": _paths(record["row_id"], graph),
                "evidence": {
                    "path": "TASKS.md",
                    "start_line": record["line"],
                    "end_line": record["end_line"],
                    "sha256": record["source_sha256"],
                },
                "owner": fields.get(
                    "owner", _owner(record["row_id"], record["kind"])
                ),
                "resolution_action": action,
                "execution_venue": fields.get("venue", venue),
                "external_fields": fields,
                "substitution_set": [],
                "critical_path_rank": _critical_rank(record)[0],
            }
        )

    executable = sorted(
        (
            row
            for row in rows
            if row["classification"] == "local"
            and not row["prerequisite_row_ids"]
            and not row["unresolved_reference_ids"]
        ),
        key=lambda row: (row["critical_path_rank"], row["line"]),
    )
    counts = Counter(row["classification"] for row in rows)
    assessments = sorted(
        (row for row in rows if row["classification"] == "unknown"),
        key=lambda row: (row["critical_path_rank"], row["line"]),
    )
    first_local = executable[0] if executable else None
    first_unknown = assessments[0] if assessments else None
    if first_unknown is not None and (
        first_local is None
        or (first_unknown["critical_path_rank"], first_unknown["line"])
        < (first_local["critical_path_rank"], first_local["line"])
    ):
        next_action_kind = "assess-unknown"
        next_action_row_id = first_unknown["row_id"]
        ready_for_unattended_execution = False
    elif first_local is not None:
        next_action_kind = "execute-local"
        next_action_row_id = first_local["row_id"]
        ready_for_unattended_execution = True
    else:
        next_action_kind = None
        next_action_row_id = None
        ready_for_unattended_execution = False
    return {
        "schema_version": 2,
        "decision_id": "ADR-0051",
        "dependency_path_policy": (
            "one deterministic transitive witness per direct prerequisite; "
            "cycle-terminated; maximum 64 nodes"
        ),
        "tasks_sha256": hashlib.sha256(text.encode()).hexdigest(),
        "unchecked_row_count": len(rows),
        "classification_counts": {
            key: counts.get(key, 0)
            for key in ("local", "dependency", "external", "unknown")
        },
        "unresolved_reference_count": sum(bool(row["unresolved_reference_ids"]) for row in rows),
        "nonempty_substitution_count": sum(bool(row["substitution_set"]) for row in rows),
        "next_executable_row_id": first_local["row_id"] if first_local else None,
        "next_executable_line": first_local["line"] if first_local else None,
        "next_action_kind": next_action_kind,
        "next_action_row_id": next_action_row_id,
        "ready_for_unattended_execution": ready_for_unattended_execution,
        "rows": rows,
    }


def build() -> bytes:
    value = build_from_text(TASKS.read_text(encoding="utf-8"))
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def check() -> dict[str, Any]:
    expected = build()
    if not OUTPUT.is_file() or OUTPUT.read_bytes() != expected:
        raise RuntimeError("remaining plan blocker register stale")
    value = json.loads(expected)
    if value["nonempty_substitution_count"]:
        raise RuntimeError("remaining row register contains a prohibited substitution")
    if sum(value["classification_counts"].values()) != value["unchecked_row_count"]:
        raise RuntimeError("remaining row classification coverage incomplete")
    if value["next_action_row_id"] is None:
        raise RuntimeError("no dependency-ready local row or unknown assessment is registered")
    return value


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    if args.write:
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_bytes(build())
    value = check()
    counts = value["classification_counts"]
    print(
        "validated row-level register: "
        f"{counts['local']} local, {counts['dependency']} dependency, "
        f"{counts['external']} external, {counts['unknown']} unknown; "
        f"next={value['next_action_kind']}:{value['next_action_row_id']}; "
        f"unattended_ready={str(value['ready_for_unattended_execution']).lower()}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
