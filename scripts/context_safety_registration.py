#!/usr/bin/env python3
"""Validate the owner-requested G1/G2 planning registration under accepted owner-delegated governance."""

from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
REGISTRATION = ROOT / "requirements" / "context-safety-registration.json"
EXPECTED_CASES = {
    "CTX-DISPATCH",
    "CTX-FINISH",
    "CTX-FIT",
    "CTX-JOURNEY",
    "CTX-RECORD",
    "CTX-REOPEN",
    "CTX-SERVED",
}
EXPECTED_MIGRATIONS = {
    "continuity-capture-coverage",
    "model-run-result",
    "prepared-model-request",
    "served-capability-observation",
    "summary-lineage",
}
EXPECTED_MILESTONES = {
    "retained-platforms",
    "v1.0-full-ga",
    "v1.0-preview-windows",
}
RESOLVED_FILES = [
    "docs/decisions/0051-business-source-license.md",
    "docs/decisions/0052-decision-0048-restart-readiness-correction.md",
]
PLAN_ID = re.compile(r"(?:Story|Task|Sub-task) (\d+\.\d+(?:\.\d+){0,2})\b")


def load(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected object")
    return value


def _cycle(graph: dict[str, list[str]]) -> list[str]:
    visiting: set[str] = set()
    visited: set[str] = set()
    path: list[str] = []

    def visit(node: str) -> list[str]:
        if node in visiting:
            return path[path.index(node) :] + [node]
        if node in visited:
            return []
        visiting.add(node)
        path.append(node)
        for dependency in graph.get(node, []):
            if dependency in graph:
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


def validate(
    registration: dict[str, Any],
    tasks_text: str,
    prd_text: str,
    registry: dict[str, Any],
) -> list[str]:
    failures: list[str] = []
    if registration.get("schema_version") != 1:
        failures.append("schema_version must equal 1")
    if registration.get("registration_id") != "context-safety-g1-g2-2026-09-08":
        failures.append("registration identity is missing or changed")
    if registration.get("owner_request_date") != "2026-09-08":
        failures.append("owner request date is missing or changed")

    governance = registration.get("governance")
    if not isinstance(governance, dict):
        failures.append("governance record is missing")
        governance = {}
    if governance.get("status") != "accepted" or governance.get("decision_id") != "ADR-0053":
        failures.append("governance must bind accepted owner-delegated Decision 0053")
    if governance.get("owner_direction_date") != "2026-09-15":
        failures.append("governance owner-direction date is missing or changed")
    if governance.get("resolved_decision_files") != RESOLVED_FILES:
        failures.append("resolved decision filenames must preserve BSL 0051 and restart readiness 0052")
    for relative in RESOLVED_FILES:
        if not (ROOT / relative).is_file():
            failures.append(f"resolved decision file is missing: {relative}")
    decision_numbers: dict[str, str] = {}
    for path in sorted((ROOT / "docs" / "decisions").glob("[0-9][0-9][0-9][0-9]-*.md")):
        number = path.name[:4]
        if number in decision_numbers:
            failures.append(f"duplicate accepted decision identity {number}: {decision_numbers[number]}, {path.name}")
        decision_numbers[number] = path.name
        if not path.read_text(encoding="utf-8").startswith(f"# Decision {number}:"):
            failures.append(f"decision title/filename identity mismatch: {path.name}")
    document = "docs/decisions/0053-owner-delegated-linux-desktop-demo.md"
    if governance.get("decision_document") != document or not (ROOT / document).is_file():
        failures.append("accepted owner-delegation document is missing or changed")
    else:
        source = (ROOT / document).read_bytes()
        if hashlib.sha256(source).hexdigest() != governance.get("decision_source_sha256"):
            failures.append("accepted owner-delegation source hash changed")
        if b"| Status | Accepted owner-delegated milestone decision |" not in source:
            failures.append("owner-delegation acceptance marker is missing")

    records = registry.get("requirements", [])
    by_id = {
        item.get("id"): item
        for item in records
        if isinstance(item, dict) and isinstance(item.get("id"), str)
    }
    bindings = registration.get("requirement_bindings")
    if not isinstance(bindings, dict) or set(bindings) != {"v0.1", "v1.0"}:
        failures.append("requirement bindings must contain exactly v0.1 and v1.0")
        bindings = {}
    for release, identifiers in bindings.items():
        if not isinstance(identifiers, list) or identifiers != sorted(set(identifiers)):
            failures.append(f"{release} requirement bindings must be sorted and unique")
            continue
        for identifier in identifiers:
            record = by_id.get(identifier)
            if record is None:
                failures.append(f"unknown requirement binding: {identifier}")
            elif record.get("release") != release:
                failures.append(
                    f"requirement binding release mismatch: {identifier} is {record.get('release')}"
                )

    known_tasks = set(PLAN_ID.findall(tasks_text))
    graph = registration.get("task_dependencies")
    if not isinstance(graph, dict) or not graph:
        failures.append("task dependency graph is missing")
        graph = {}
    for node, dependencies in graph.items():
        if node not in known_tasks:
            failures.append(f"task dependency node is absent from TASKS.md: {node}")
        if not isinstance(dependencies, list) or dependencies != sorted(set(dependencies)):
            failures.append(f"task dependencies must be sorted and unique: {node}")
            continue
        for dependency in dependencies:
            if dependency not in known_tasks:
                failures.append(f"unknown task dependency: {node} -> {dependency}")
    if cycle := _cycle(graph):
        failures.append("context-safety task cycle: " + " -> ".join(cycle))
    if graph.get("13.4.5.1") != ["13.1.4", "13.1.5", "13.3.4.1"]:
        failures.append("prepared-request entry must not depend on its guarded native trial")
    if "13.4.5" not in graph.get("13.3.4.3", []):
        failures.append("native trial must remain downstream of prepared-request enforcement")

    cases = registration.get("acceptance_cases")
    if not isinstance(cases, dict) or set(cases) != EXPECTED_CASES:
        failures.append("acceptance case set must exactly match the PRD CTX matrix")
        cases = {}
    for case_id, case in cases.items():
        if f"`{case_id}`" not in prd_text:
            failures.append(f"acceptance case is absent from PRD.md: {case_id}")
        if not isinstance(case, dict) or case.get("owner") not in known_tasks:
            failures.append(f"acceptance case owner is invalid: {case_id}")
        elif set(case.get("release_applicability", [])) != EXPECTED_MILESTONES:
            failures.append(f"acceptance case applicability is incomplete: {case_id}")

    migrations = registration.get("contract_migrations")
    if not isinstance(migrations, list):
        failures.append("contract migration plan is missing")
        migrations = []
    migration_ids = [item.get("contract") for item in migrations if isinstance(item, dict)]
    if set(migration_ids) != EXPECTED_MIGRATIONS or len(migration_ids) != len(set(migration_ids)):
        failures.append("contract migration set is incomplete or duplicated")
    for migration in migrations:
        if not isinstance(migration, dict):
            failures.append("contract migration entry must be an object")
            continue
        if migration.get("legacy_disposition") not in {
            "disable-complete-continuity",
            "reject",
            "reject-summary-authority",
            "requalify",
        }:
            failures.append(f"open legacy migration disposition: {migration.get('contract')}")
        if not isinstance(migration.get("target_version"), int) or migration["target_version"] < 1:
            failures.append(f"invalid target version: {migration.get('contract')}")
        if migration.get("missing_reason_code") not in registration.get("refusal_codes", []):
            failures.append(f"migration lacks a closed missing-field refusal: {migration.get('contract')}")

    refusal_codes = registration.get("refusal_codes")
    if not isinstance(refusal_codes, list) or refusal_codes != sorted(set(refusal_codes)):
        failures.append("refusal codes must be sorted and unique")
    return failures


def registered_dependencies(value: dict[str, Any] | None = None) -> dict[str, list[str]]:
    registration = value if value is not None else load(REGISTRATION)
    graph = registration.get("task_dependencies")
    if not isinstance(graph, dict):
        raise ValueError("task dependency graph is missing")
    return {str(node): [str(item) for item in dependencies] for node, dependencies in graph.items()}


def main() -> int:
    try:
        registration = load(REGISTRATION)
        failures = validate(
            registration,
            (ROOT / "TASKS.md").read_text(encoding="utf-8"),
            (ROOT / "PRD.md").read_text(encoding="utf-8"),
            load(ROOT / "requirements" / "registry.json"),
        )
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"context-safety registration failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"context-safety registration failed: {failure}", file=sys.stderr)
        return 1
    print(
        "context-safety registration validated: 34 task nodes, 7 CTX cases, "
        "5 migrations; governance accepted under Decision 0053"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
