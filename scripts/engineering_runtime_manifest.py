#!/usr/bin/env python3
"""Build and validate the Decisions 0043-0044 change manifest."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
OUTPUT: Final = ROOT / "architecture" / "engineering-runtime-change-manifest.json"
REGISTRY: Final = ROOT / "requirements" / "registry.json"
NEW_SCHEMAS: Final = (
    "artifact-envelope", "artifact-ingestion-result", "artifact-transformation",
    "capability-manifest", "context-delivery-receipt", "context-manifest",
    "model-endpoint-profile", "model-route-decision", "terminal-result",
    "tool-observation", "verification-result", "workflow-checkpoint",
    "workflow-definition", "workflow-state",
)
REUSED_SCHEMAS: Final = {
    "action-proposal": "schemas/model/closed-proposal.schema.json",
    "model-event": "schemas/model/stream-fragment.schema.json",
    "model-profile": "schemas/model/exact-profile.schema.json",
    "model-request": "schemas/model/run-request.schema.json",
    "runtime-event": "schemas/runtime/runtime-event.schema.json",
    "tool-call": "schemas/model/tool-call-candidate.schema.json",
}
ADDED_STORY_SUBTASK_COUNTS: Final = {
    "1.3": 7,
    "2.4": 7,
    "5.3": 7,
    "11.3": 7,
    "13.5": 7,
    "13.6": 7,
    "16.4": 7,
    "21.4": 7,
    "22.5": 7,
    "23.7": 7,
    "23.8": 7,
    "49.2": 7,
    "50.4": 8,
    "95.3": 7,
    "95.4": 7,
    "121.2": 6,
    "123.2": 7,
    "124.2": 7,
    "125.3": 7,
    "126.2": 8,
}

# requirement: decision, epic, milestone, sprint, story, schemas, modules, controls, protocol
PLAN: Final = {
    "AM-ERT-001": ("ADR-0043", "FRE-ENGINEERING-RUNTIME", "ER-M1", 1, "1.3", ("workflow-definition", "workflow-state", "runtime-event"), ("kernel-contracts", "kernel-engine"), ("SR-ERT-001", "SR-ERT-002", "SR-ERT-003"), "RV-50"),
    "AM-CTX-001": ("ADR-0043", "FRE-ENGINEERING-RUNTIME", "ER-M1", 2, "2.4", ("artifact-envelope", "artifact-ingestion-result", "artifact-transformation"), ("kernel-contracts", "kernel-engine"), ("SR-CTX-001", "SR-CTX-002"), "RV-51"),
    "AM-CTX-002": ("ADR-0043", "FRE-ENGINEERING-RUNTIME", "ER-M1", 2, "2.4", ("context-manifest", "artifact-transformation"), ("kernel-contracts", "kernel-engine"), ("SR-CTX-003", "SR-CTX-004"), "RV-51"),
    "AM-CTX-003": ("ADR-0043", "FRE-ENGINEERING-RUNTIME", "ER-M1", 2, "2.4", ("context-delivery-receipt", "context-manifest"), ("kernel-contracts", "kernel-engine"), ("SR-CTX-005", "SR-CTX-006"), "RV-51"),
    "AM-WKF-001": ("ADR-0043", "FRE-ENGINEERING-RUNTIME", "ER-M2", 5, "5.3", ("workflow-definition", "workflow-state", "verification-result"), ("kernel-contracts", "kernel-engine"), ("SR-ERT-002", "SR-ERT-003", "SR-ERT-004"), "RV-50"),
    "AM-WKF-002": ("ADR-0043", "FRE-ENGINEERING-RUNTIME", "ER-M2", 11, "11.3", ("workflow-state", "workflow-checkpoint"), ("kernel-contracts", "kernel-engine"), ("SR-ERT-004", "SR-ERT-005"), "RV-52"),
    "AM-SES-002": ("ADR-0043", "FRE-ENGINEERING-RUNTIME", "ER-M2", 11, "11.3", ("workflow-checkpoint", "workflow-state"), ("kernel-engine", "shell-host"), ("SR-ERT-004", "SR-ERT-005"), "RV-52"),
    "AM-TIO-001": ("ADR-0043", "FRE-ENGINEERING-RUNTIME", "ER-M2", 16, "16.4", ("tool-observation", "terminal-result", "tool-call"), ("kernel-contracts", "kernel-engine", "shell-host"), ("SR-ERT-005", "SR-ERT-006"), "RV-52"),
    "AM-VER-001": ("ADR-0043", "FRE-ENGINEERING-RUNTIME", "ER-M2", 5, "5.3", ("verification-result", "terminal-result"), ("kernel-contracts", "kernel-engine"), ("SR-ERT-003", "SR-ERT-006"), "RV-50"),
    "AM-GWY-001": ("ADR-0044", "FRE-MODEL-GATEWAY", "ER-M3", 13, "13.5", ("model-request", "model-event", "action-proposal"), ("kernel-contracts", "kernel-engine"), ("SR-GWY-001", "SR-GWY-002"), "RV-53"),
    "AM-GWY-002": ("ADR-0044", "FRE-MODEL-GATEWAY", "ER-M3", 13, "13.6", ("model-request", "model-event", "model-endpoint-profile"), ("kernel-contracts", "kernel-engine", "platform-linux-native-inference"), ("SR-GWY-003", "SR-GWY-004"), "RV-53"),
    "AM-GWY-003": ("ADR-0044", "FRE-MODEL-GATEWAY", "ER-M3", 13, "13.6", ("model-route-decision", "model-profile", "model-endpoint-profile"), ("kernel-contracts", "kernel-engine"), ("SR-GWY-005", "SR-GWY-006"), "RV-53"),
    "AM-REM-001": ("ADR-0044", "FRE-MODEL-GATEWAY", "ER-M3", 13, "13.5", ("model-endpoint-profile", "model-route-decision"), ("kernel-contracts", "kernel-engine"), ("SR-GWY-005", "SR-GWY-006"), "RV-53"),
    "AM-REM-002": ("ADR-0044", "FRE-MODEL-GATEWAY", "ER-M6", 123, "123.2", ("model-endpoint-profile", "model-route-decision"), ("kernel-engine", "platform-linux", "platform-windows"), ("SR-GWY-007", "SR-GWY-008", "SR-GWY-009"), "RV-54"),
    "AM-REM-003": ("ADR-0044", "FRE-MODEL-GATEWAY", "ER-M7", 123, "123.2", ("model-profile", "model-endpoint-profile", "model-route-decision"), ("kernel-engine", "platform-linux", "platform-windows"), ("SR-GWY-009", "SR-GWY-010"), "RV-54"),
    "AM-OBS-002": ("ADR-0043", "FRE-ENGINEERING-RUNTIME", "ER-M2", 21, "21.4", ("runtime-event", "tool-observation", "terminal-result"), ("kernel-engine", "shell-host", "shell-vscode"), ("SR-ERT-005", "SR-VSC-004"), "RV-52"),
    "AM-DEG-001": ("ADR-0043", "FRE-ENGINEERING-RUNTIME", "ER-M9", 50, "50.4", ("workflow-state", "terminal-result", "capability-manifest"), ("kernel-engine",), ("SR-ERT-006", "SR-CAP-004"), "RV-50"),
    "AM-PERF-001": ("ADR-0043", "FRE-ENGINEERING-RUNTIME", "ER-M2", 21, "21.4", ("runtime-event", "model-event", "tool-observation"), ("kernel-engine", "shell-host"), ("SR-ERT-005", "SR-GWY-004"), "RV-52"),
    "AM-VSC-004": ("ADR-0043", "FRE-ENGINEERING-RUNTIME", "ER-M4", 23, "23.7", ("runtime-event", "workflow-state", "terminal-result"), ("shell-vscode", "shell-host", "kernel-engine"), ("SR-VSC-001", "SR-VSC-002"), "RV-55"),
    "AM-VSC-005": ("ADR-0043", "FRE-ENGINEERING-RUNTIME", "ER-M4", 23, "23.8", ("artifact-envelope", "context-manifest", "runtime-event"), ("shell-vscode", "shell-host"), ("SR-VSC-002", "SR-VSC-003"), "RV-55"),
    "AM-VSC-006": ("ADR-0043", "FRE-ENGINEERING-RUNTIME", "ER-M4", 23, "23.8", ("model-request", "model-event", "action-proposal"), ("shell-vscode", "shell-host"), ("SR-VSC-003", "SR-VSC-004"), "RV-55"),
    "AM-CAP-001": ("ADR-0043", "FRE-ENGINEERING-RUNTIME", "ER-M5", 95, "95.3", ("capability-manifest", "workflow-definition"), ("kernel-contracts", "kernel-engine"), ("SR-CAP-001", "SR-CAP-002"), "RV-56"),
    "AM-CAP-002": ("ADR-0043", "FRE-ENGINEERING-RUNTIME", "ER-M5", 95, "95.3", ("capability-manifest", "workflow-state", "verification-result"), ("kernel-engine",), ("SR-CAP-003", "SR-CAP-004"), "RV-56"),
    "AM-MAG-001": ("ADR-0043", "FRE-ENGINEERING-RUNTIME", "ER-M8", 95, "95.4", ("capability-manifest", "workflow-definition", "workflow-state"), ("kernel-engine", "shell-host"), ("SR-MAG-001", "SR-MAG-002", "SR-MAG-003", "SR-MAG-004"), "RV-57"),
}


def _load(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected object")
    return value


def _schema_path(name: str) -> str:
    return REUSED_SCHEMAS.get(name, f"schemas/engineering-runtime/{name}.schema.json")


def build_manifest(root: Path = ROOT) -> dict[str, Any]:
    registry = _load(root / REGISTRY.relative_to(ROOT))
    indexed = {item["id"]: item for item in registry["requirements"]}
    mappings: list[dict[str, Any]] = []
    for requirement_id in sorted(PLAN):
        decision, epic, milestone, sprint, story, schemas, modules, controls, protocol = PLAN[requirement_id]
        mappings.append({
            "requirement_id": requirement_id,
            "decision_id": decision,
            "acceptance_test_ids": indexed[requirement_id]["acceptance_tests"],
            "security_control_ids": list(controls),
            "review_protocol_ids": [protocol],
            "milestone_id": milestone,
            "foundational_epic_id": epic,
            "sprint_id": sprint,
            "story_id": story,
            "task_ids": [f"{story}.1", f"{story}.2", f"{story}.3"],
            "schema_paths": [_schema_path(name) for name in schemas],
            "module_ids": list(modules),
            "verification_commands": [
                "npm run engineering-runtime:schemas:check",
                "npm run task-graph:check",
                "npm run requirements:current-check",
            ],
            "status": "planned",
        })
    return {
        "schema_version": 1,
        "status": "enforced-planning-contract",
        "decision_ids": ["ADR-0043", "ADR-0044"],
        "current_status_ref": "architecture/status-model.json",
        "execution_authority": "TASKS.md",
        "strict_local_complete_target": True,
        "remote_inference_optional": True,
        "automatic_fallback": False,
        "counts": {
            "product_requirements": len(mappings),
            "acceptance_tests": len({test for item in mappings for test in item["acceptance_test_ids"]}),
            "new_schemas": len(NEW_SCHEMAS),
            "reused_schemas": len(REUSED_SCHEMAS),
            "foundational_epics_added": 2,
            "foundational_epics_total": 4,
            "release_epics_added": 0,
            "sprints_added": 0,
            "requirement_owner_stories": len({item["story_id"] for item in mappings}),
            "planned_stories_added": len(ADDED_STORY_SUBTASK_COUNTS),
            "tasks_added": 3 * len(ADDED_STORY_SUBTASK_COUNTS),
            "sub_tasks_added": sum(ADDED_STORY_SUBTASK_COUNTS.values()),
        },
        "added_story_ids": list(ADDED_STORY_SUBTASK_COUNTS),
        "profile_classes": ["strict_local", "local_network_private", "remote_private", "remote_managed"],
        "new_schema_paths": [_schema_path(name) for name in NEW_SCHEMAS],
        "reused_schema_paths": [REUSED_SCHEMAS[name] for name in sorted(REUSED_SCHEMAS)],
        "requirement_mappings": mappings,
    }


def validate_manifest(manifest: dict[str, Any], root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    if manifest != build_manifest(root):
        failures.append("engineering runtime change manifest is stale or structurally different")
    mappings = manifest.get("requirement_mappings", [])
    if not isinstance(mappings, list):
        return failures + ["requirement_mappings must be an array"]
    requirement_ids = [item.get("requirement_id") for item in mappings if isinstance(item, dict)]
    if requirement_ids != sorted(PLAN):
        failures.append("requirement mapping identity closure is incomplete or reordered")
    test_ids = {test for item in mappings if isinstance(item, dict) for test in item.get("acceptance_test_ids", [])}
    if len(test_ids) != 29:
        failures.append(f"expected 29 acceptance tests, found {len(test_ids)}")
    tasks = (root / "TASKS.md").read_text(encoding="utf-8")
    inventory = (root / "Agent-Scaffolding-Inventory.md").read_text(encoding="utf-8")
    security = (root / "SECURITY-REVIEW.md").read_text(encoding="utf-8")
    plan = (root / "IMPLEMENTATION-PLAN.md").read_text(encoding="utf-8")
    modules = {item["id"] for item in _load(root / "architecture/module-inventory.json")["modules"]}
    if manifest.get("added_story_ids") != list(ADDED_STORY_SUBTASK_COUNTS):
        failures.append("complete added-story identity closure is incomplete or reordered")
    for story_id, expected_subtasks in ADDED_STORY_SUBTASK_COUNTS.items():
        block_match = re.search(
            rf"^#### \[ \] Story {re.escape(story_id)}\b(?P<body>.*?)(?=^#### \[[ x]\] Story |^### \[[ x]\] Sprint |\Z)",
            tasks,
            re.MULTILINE | re.DOTALL,
        )
        if block_match is None:
            failures.append(f"added story {story_id} is missing or promoted")
            continue
        body = block_match.group("body")
        task_ids = re.findall(rf"^\- \[ \] \*\*Task ({re.escape(story_id)}\.\d+)\b", body, re.MULTILINE)
        expected_task_ids = [f"{story_id}.{index}" for index in range(1, 4)]
        if task_ids != expected_task_ids:
            failures.append(f"added story {story_id} does not retain exactly three ordered open tasks")
        subtask_count = len(re.findall(r"^  - \[ \] \*\*Sub-task ", body, re.MULTILINE))
        if subtask_count != expected_subtasks:
            failures.append(
                f"added story {story_id} expected {expected_subtasks} open sub-tasks, found {subtask_count}"
            )
    for item in mappings:
        requirement_id = item["requirement_id"]
        if f"`{requirement_id}`" not in inventory:
            failures.append(f"{requirement_id}: inventory source is missing")
        if not re.search(rf"^#### \[ \] Story {re.escape(item['story_id'])}\b", tasks, re.MULTILINE):
            failures.append(f"{requirement_id}: story {item['story_id']} is missing or promoted")
        for task_id in item["task_ids"]:
            if not re.search(rf"\*\*Task {re.escape(task_id)}\b", tasks):
                failures.append(f"{requirement_id}: task {task_id} is missing")
        if item["milestone_id"] not in plan or item["foundational_epic_id"] not in tasks:
            failures.append(f"{requirement_id}: milestone or foundational epic is missing")
        for control in item["security_control_ids"] + item["review_protocol_ids"]:
            if f"`{control}`" not in security:
                failures.append(f"{requirement_id}: security owner {control} is missing")
        for path in item["schema_paths"]:
            if not (root / path).is_file():
                failures.append(f"{requirement_id}: schema is missing: {path}")
        for module_id in item["module_ids"]:
            if module_id not in modules:
                failures.append(f"{requirement_id}: module is unknown: {module_id}")
    return failures


def _render(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args(argv)
    try:
        expected = build_manifest()
        if args.write:
            OUTPUT.write_text(_render(expected), encoding="utf-8")
        failures = validate_manifest(_load(OUTPUT))
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"engineering runtime manifest validation failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"engineering runtime manifest validation failed: {failure}", file=sys.stderr)
        return 1
    print(
        "engineering runtime manifest validation passed: 24 requirements, 29 tests, "
        "20 stories, 60 tasks, 141 sub-tasks, 14 new schemas, 6 reused schemas"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
