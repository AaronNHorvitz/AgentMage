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

try:
    from scripts.context_safety_registration import load, registered_dependencies
except ModuleNotFoundError:
    from context_safety_registration import load, registered_dependencies


ROOT = Path(__file__).resolve().parents[1]
TASKS = ROOT / "TASKS.md"
OUTPUT = ROOT / "docs" / "verification" / "remaining-plan-blocker-audit.json"
CONTEXT_SAFETY_REGISTRATION = ROOT / "requirements" / "context-safety-registration.json"
CODING_PRIORITY_DECISION = (
    ROOT / "docs" / "decisions" / "0061-standalone-coding-harness-critical-path.md"
)
AMENDMENT_AUTHORITIES = (
    "IMPLEMENTATION-AMENDMENT.md",
    "CAPABILITY-ROADMAP.md",
    "docs/decisions/0081-rust-capability-roadmap-and-staged-delivery.md",
    "docs/decisions/0088-amendment-coverage-in-work-selection.md",
)
AMR_ID = re.compile(r"AMR-(?:0[1-9]|[1-9][0-9])(?:\.[1-9][0-9]*)*")
AMR_REFERENCE = re.compile(r"AMR-[A-Za-z0-9_.-]+")
EXPECTED_AMR_IDS = frozenset({
    "AMR-01", "AMR-02", "AMR-03", "AMR-04", "AMR-05", "AMR-06", "AMR-07",
    "AMR-01.1", "AMR-01.2", "AMR-02.1", "AMR-02.2", "AMR-02.3",
    "AMR-02.3.1", "AMR-02.3.2", "AMR-02.4", "AMR-03.1", "AMR-03.1.1",
    "AMR-03.1.2", "AMR-03.1.3", "AMR-03.2", "AMR-04.1", "AMR-04.2",
    "AMR-04.2.1", "AMR-04.2.2", "AMR-04.2.3", "AMR-04.3", "AMR-04.4", "AMR-04.4.1",
    "AMR-04.5", "AMR-04.6", "AMR-04.6.1", "AMR-04.6.2", "AMR-04.6.3", "AMR-04.6.4",
    "AMR-04.7",
    "AMR-04.7.1", "AMR-04.7.2", "AMR-04.8",
    "AMR-04.8.1", "AMR-04.9", "AMR-04.10",
    "AMR-05.1", "AMR-05.2", "AMR-05.3", "AMR-05.4", "AMR-05.5",
    "AMR-05.6", "AMR-05.7", "AMR-05.8", "AMR-05.9", "AMR-05.10",
    "AMR-05.9.1", "AMR-05.9.2", "AMR-05.9.3", "AMR-05.9.4", "AMR-05.9.5",
})
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
CODING_TASK_ORDER = {"48.2.4": -30, "48.2.5": -20, "48.2.6": -10, "50.2.4": -5}
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


def _row_records(text: str, table_boundaries: tuple[int, ...] = ()) -> list[dict[str, Any]]:
    lines = text.splitlines()
    matches: list[tuple[int, re.Match[str]]] = []
    for index, line in enumerate(lines):
        if match := ROW.match(line):
            matches.append((index, match))

    records: list[dict[str, Any]] = []
    for position, (start, match) in enumerate(matches):
        end = matches[position + 1][0] if position + 1 < len(matches) else len(lines)
        end = min((boundary for boundary in table_boundaries if start < boundary < end), default=end)
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


def _amr_records(text: str) -> list[dict[str, Any]]:
    """Inventory closed table rows without interpreting prose as gate approval."""
    records: list[dict[str, Any]] = []
    seen: set[str] = set()
    for line_number, line in enumerate(text.splitlines(), start=1):
        if not line.lstrip().startswith("|"):
            continue
        cells = [cell.strip() for cell in re.split(r"(?<!\\)\|", line.strip())]
        if len(cells) < 3 or not cells[2].startswith("AMR-"):
            continue
        if cells[0] or cells[-1] or len(cells) not in (6, 7):
            raise ValueError(f"malformed AMR table row at line {line_number}")
        cells = cells[1:-1]
        state, row_id = cells[:2]
        package = "." not in row_id
        if (
            state not in ("[ ]", "[x]", "[X]")
            or AMR_ID.fullmatch(row_id) is None
            or len(cells) != (5 if package else 4)
            or any(not cell for cell in cells)
        ):
            raise ValueError(f"invalid AMR identity, state or columns at line {line_number}")
        if row_id in seen:
            raise ValueError(f"duplicate AMR identity: {row_id}")
        seen.add(row_id)
        dependency_text = cells[-2]
        references = sorted(set(AMR_REFERENCE.findall(dependency_text)))
        if any(AMR_ID.fullmatch(reference) is None for reference in references):
            raise ValueError(f"malformed AMR dependency at line {line_number}")
        records.append({
            "row_id": row_id,
            "line": line_number,
            "end_line": line_number,
            "checked": state.lower() == "[x]",
            "kind": "amendment-package" if package else "amendment-component",
            "text": cells[-1],
            "source_text": line,
            "source_sha256": hashlib.sha256(line.encode()).hexdigest(),
            "story_id": None,
            "dependency_text": dependency_text,
            "dependency_reference_ids": references,
        })
    return records


def _selection_key(row: dict[str, Any]) -> tuple[int, tuple[int, ...], int]:
    identity = row["row_id"]
    amendment_order = (
        tuple(int(part) for part in identity.removeprefix("AMR-").split("."))
        if identity.startswith("AMR-") else ()
    )
    return row["critical_path_rank"], amendment_order, row["line"]


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
    if record["row_id"].startswith("AMR-"):
        return -4, record["line"]
    story = record.get("story_id")
    if record["row_id"].startswith(("13.1.4", "13.1.5", "13.1.6")):
        return 10, record["line"]
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


def _coding_task_id(row_id: str) -> str | None:
    return next(
        (task_id for task_id in CODING_TASK_ORDER
         if row_id == task_id or row_id.startswith(task_id + ".")),
        None,
    )


def _coding_dependency_ranks(graph: dict[str, list[str]]) -> dict[str, int]:
    """Prioritize only the accepted coding additions and their exact prerequisites."""
    ranks: dict[str, int] = {}
    for row_id in graph:
        task_id = _coding_task_id(row_id)
        if task_id is not None:
            ranks[row_id] = CODING_TASK_ORDER[task_id]
    pending = list(ranks)
    while pending:
        row_id = pending.pop()
        for dependency in graph.get(row_id, []):
            if ranks.get(dependency, 0) > ranks[row_id]:
                ranks[dependency] = ranks[row_id]
                pending.append(dependency)
    return ranks


def build_from_text(
    text: str, registered_graph: dict[str, list[str]] | None = None
) -> dict[str, Any]:
    amendment_records = _amr_records(text)
    legacy_records = _row_records(text, tuple(row["line"] - 1 for row in amendment_records))
    records = sorted(legacy_records + amendment_records, key=lambda row: row["line"])
    known = {record["row_id"] for record in records}
    open_records = [record for record in records if not record["checked"]]
    open_ids = {record["row_id"] for record in open_records}

    graph: dict[str, list[str]] = {}
    unresolved_by_id: dict[str, list[str]] = {}
    for record in open_records:
        if record["kind"].startswith("amendment-"):
            # These may name a source contract rather than a whole-package gate.
            # Keep the prose and references below; do not invent resolved edges.
            graph[record["row_id"]] = []
            unresolved_by_id[record["row_id"]] = sorted(
                set(record["dependency_reference_ids"]) - known
            )
            continue
        dependency_record = record
        if _coding_task_id(record["row_id"]) is not None:
            # Decision 0061 gives these new rows explicit, inline prerequisites.
            # Trailing story/release commentary is not an entry dependency.
            dependency_record = {**record, "dependency_text": record["text"]}
        referenced, unresolved = _references(dependency_record, known)
        structural = _structural_dependencies(record, records)
        dependencies = set(referenced) | set(structural)
        if registered_graph is not None and record["row_id"] in registered_graph:
            dependencies = set(registered_graph[record["row_id"]])
        graph[record["row_id"]] = sorted(dependencies & open_ids)
        unresolved_by_id[record["row_id"]] = unresolved

    coding_ranks = _coding_dependency_ranks(graph)
    rows: list[dict[str, Any]] = []
    for record in open_records:
        source = record["source_text"]
        classification_source = record.get("dependency_text", source)
        fields = {
            key: value.strip().strip("`").rstrip(".")
            for key, value in FIELD.findall(source)
        }
        if record["kind"].startswith("amendment-"):
            # Descriptive examples do not appoint an owner or execution venue.
            fields = {}
        unresolved = unresolved_by_id[record["row_id"]]
        dependencies = graph[record["row_id"]]
        structural = record["kind"] in {
            "foundational-epic", "epic", "sprint", "story", "task",
            "story-acceptance", "sprint-acceptance", "universal-control",
        }
        if record["kind"].startswith("amendment-"):
            classification = "unknown"
            action = "assess the exact source, native, model or owner prerequisites in the retained dependency text"
            venue = "unassessed"
        elif structural and dependencies:
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
                "critical_path_rank": coding_ranks.get(
                    record["row_id"], _critical_rank(record)[0]
                ),
            }
        )
        if record["kind"].startswith("amendment-"):
            rows[-1]["amendment_assessment"] = {
                "dependency_text": record["dependency_text"],
                "referenced_row_ids": record["dependency_reference_ids"],
                "completion_gate_edges_resolved": False,
                "execution_authorized": False,
            }

    executable = sorted(
        (
            row
            for row in rows
            if row["classification"] == "local"
            and not row["prerequisite_row_ids"]
            and not row["unresolved_reference_ids"]
        ),
        key=_selection_key,
    )
    counts = Counter(row["classification"] for row in rows)
    assessments = sorted(
        (row for row in rows if row["classification"] == "unknown"),
        key=_selection_key,
    )
    first_local = executable[0] if executable else None
    first_unknown = assessments[0] if assessments else None
    if first_unknown is not None and (
        first_local is None
        or _selection_key(first_unknown) < _selection_key(first_local)
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
        "schema_version": 3,
        "decision_id": "ADR-0052",
        "dependency_path_policy": (
            "one deterministic transitive witness per direct prerequisite; "
            "cycle-terminated; maximum 64 nodes"
        ),
        "tasks_sha256": hashlib.sha256(text.encode()).hexdigest(),
        "coverage": {
            "legacy_checkbox_rows": len(legacy_records),
            "legacy_open_rows": sum(not row["checked"] for row in legacy_records),
            "amendment_table_rows": len(amendment_records),
            "amendment_open_rows": sum(not row["checked"] for row in amendment_records),
            "amendment_row_ids": sorted(row["row_id"] for row in amendment_records),
            "amendment_classification_policy": "assessment required; reference recognition is not gate resolution",
        },
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
    registration = load(CONTEXT_SAFETY_REGISTRATION)
    value = build_from_text(
        TASKS.read_text(encoding="utf-8"), registered_dependencies(registration)
    )
    if set(value["coverage"]["amendment_row_ids"]) != EXPECTED_AMR_IDS:
        raise ValueError("accepted AMR row inventory is missing or changed; reconcile its authority before selection")
    value["amendment_authorities"] = [
        {"path": path, "sha256": hashlib.sha256((ROOT / path).read_bytes()).hexdigest()}
        for path in AMENDMENT_AUTHORITIES
    ]
    value["priority_amendment"] = {
        "decision_id": "ADR-0061",
        "path": CODING_PRIORITY_DECISION.relative_to(ROOT).as_posix(),
        "sha256": hashlib.sha256(CODING_PRIORITY_DECISION.read_bytes()).hexdigest(),
        "task_ranks": CODING_TASK_ORDER,
        "scope": "coding additions and exact prerequisites; no blocker substitution",
    }
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
        f"unattended_ready={str(value['ready_for_unattended_execution']).lower()}; "
        f"AMR_assessment={value['coverage']['amendment_open_rows']}/"
        f"{value['coverage']['amendment_table_rows']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
