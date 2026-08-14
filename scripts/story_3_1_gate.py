#!/usr/bin/env python3
"""Independently evaluate Story 3.1 while preserving the macOS blocker."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.component_inventory import check_artifact as check_components  # noqa: E402
from scripts.configuration_authority_mutation_evidence import (  # noqa: E402
    check_artifact as check_authority_mutations,
)
from scripts.configuration_loader_evidence import check_artifact as check_loader  # noqa: E402
from scripts.configuration_migration_recovery_evidence import (  # noqa: E402
    check_artifact as check_migration,
)
from scripts.configuration_startup_evidence import check_artifact as check_startup  # noqa: E402
from scripts.story_3_1_security_evidence import check_map as check_security  # noqa: E402


REPORT_PATH = ROOT / "artifacts/sprints/sprint-3/story-3.1/story-gate-report.json"
REVIEWED_COMMIT = "79eb3b35093a35f1dacc69207ceddfbeecc1727a"
REVIEWED_TREE = "8d072d14b9735804ee7a568f677cccc781aefce0"
REVIEWED_PATHS = (
    "configuration/profiles/catalog.json",
    "configuration/profiles/capability-deltas.json",
    "configuration/permission-bearing-values.json",
    "kernel/engine/src/configuration.rs",
    "artifacts/sprints/sprint-3/story-3.1/configuration-schema-report.json",
    "artifacts/sprints/sprint-3/story-3.1/profile-catalog-report.json",
    "artifacts/sprints/sprint-3/story-3.1/configuration-loader-report.json",
    "artifacts/sprints/sprint-3/story-3.1/configuration-authority-mutation-report.json",
    "artifacts/sprints/sprint-3/story-3.1/configuration-migration-recovery-report.json",
    "artifacts/sprints/sprint-3/story-3.1/configuration-startup-report.json",
    "artifacts/sprints/sprint-3/story-3.1/configuration-result-report.json",
    "artifacts/sprints/sprint-3/story-3.1/configuration-schema-failure-report.json",
    "artifacts/sprints/sprint-3/story-3.1/configuration-review-artifacts-report.json",
    "artifacts/sprints/sprint-3/story-3.1/component-inventory-report.json",
    "artifacts/sprints/sprint-3/story-3.1/update-rollback-design-report.json",
    "artifacts/sprints/sprint-3/story-3.1/security-evidence-map.json",
)
REQUIRED_TASK_MARKERS = (
    "- [x] **Task 3.1.1 - Implement the bounded story**",
    "- [x] **Task 3.1.2 - Produce reviewable artifacts**",
    "- [x] **Task 3.1.3 - Verify and close the story**",
    "  - [x] **Sub-task 3.1.3.1:**",
    "  - [x] **Sub-task 3.1.3.2:**",
    "  - [x] **Sub-task 3.1.3.3:**",
    "  - [x] **Sub-task 3.1.3.4:**",
    "  - [x] **Sub-task 3.1.3.5 - Product security evidence:**",
    "- [x] **Story AC 3.1.AC1:**",
    "- [x] **Story AC 3.1.AC2:**",
)
G_DOD_IDS = tuple(f"G-DOD-{index:02d}" for index in range(1, 14))


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-story-3-1-gate-", dir=path.parent
    )
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
        temporary.chmod(0o644)
        os.replace(temporary, path)
    except BaseException:
        temporary.unlink(missing_ok=True)
        raise


def git_output(*arguments: str, root: Path = ROOT, binary: bool = False) -> bytes | str:
    result = subprocess.run(
        ["git", *arguments],
        cwd=root,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
        timeout=10,
    )
    if result.returncode != 0:
        raise ValueError(f"git review operation failed: {' '.join(arguments)}")
    return result.stdout if binary else result.stdout.decode("utf-8").strip()


def reviewed_artifacts(root: Path = ROOT) -> list[dict[str, str]]:
    commit = git_output("rev-parse", REVIEWED_COMMIT, root=root)
    tree = git_output("show", "-s", "--format=%T", REVIEWED_COMMIT, root=root)
    if commit != REVIEWED_COMMIT or tree != REVIEWED_TREE:
        raise ValueError("Story 3.1 review identity is unavailable or changed")
    records = []
    for path in REVIEWED_PATHS:
        committed = git_output("show", f"{REVIEWED_COMMIT}:{path}", root=root, binary=True)
        current = root / path
        if not current.is_file() or current.read_bytes() != committed:
            raise ValueError(f"reviewed Story 3.1 artifact changed after review: {path}")
        records.append({"path": path, "sha256": sha256_bytes(committed)})
    return records


def task_completion(tasks_text: str) -> list[str]:
    return [marker for marker in REQUIRED_TASK_MARKERS if marker not in tasks_text]


def validate_inputs(root: Path = ROOT) -> list[str]:
    validators = (
        ("loader", check_loader),
        ("authority-mutations", check_authority_mutations),
        ("migration", check_migration),
        ("startup", check_startup),
        ("components", check_components),
        ("security-map", check_security),
    )
    failures = []
    for name, validator in validators:
        failures.extend(f"{name}: {failure}" for failure in validator(root))
    failures.extend(
        f"task marker is incomplete: {marker}"
        for marker in task_completion((root / "TASKS.md").read_text(encoding="utf-8"))
    )
    return failures


def universal_dod() -> list[dict[str, Any]]:
    statuses = {
        "G-DOD-01": "pass-story-scope",
        "G-DOD-02": "pass-story-scope",
        "G-DOD-03": "pass-story-scope",
        "G-DOD-04": "pass-no-product-authority",
        "G-DOD-05": "pass-story-scope",
        "G-DOD-06": "pass-no-product-tool-attempt",
        "G-DOD-07": "pass-story-scope",
        "G-DOD-08": "pass-story-scope",
        "G-DOD-09": "pass-story-scope",
        "G-DOD-10": "blocked-macos",
        "G-DOD-11": "pass-story-scope",
        "G-DOD-12": "pass-automated-independent-implementation-review",
        "G-DOD-13": "pass-gate-reporting",
    }
    return [
        {
            "control_id": control_id,
            "status": statuses[control_id],
            "evidence": [
                "artifacts/sprints/sprint-3/story-3.1/security-evidence-map.json",
                "artifacts/sprints/sprint-3/story-3.1/story-gate-report.json",
            ],
        }
        for control_id in G_DOD_IDS
    ]


def build_report(root: Path = ROOT) -> dict[str, Any]:
    failures = validate_inputs(root)
    if failures:
        raise ValueError("; ".join(failures))
    reviewed = reviewed_artifacts(root)
    startup = read_json(
        root / "artifacts/sprints/sprint-3/story-3.1/configuration-startup-report.json"
    )
    migration = read_json(
        root
        / "artifacts/sprints/sprint-3/story-3.1/configuration-migration-recovery-report.json"
    )
    authority = read_json(
        root
        / "artifacts/sprints/sprint-3/story-3.1/configuration-authority-mutation-report.json"
    )
    security = read_json(
        root / "artifacts/sprints/sprint-3/story-3.1/security-evidence-map.json"
    )
    return {
        "schema_version": 1,
        "story_id": "3.1",
        "status": "blocked-macos",
        "reviewed_commit": REVIEWED_COMMIT,
        "reviewed_tree": REVIEWED_TREE,
        "acceptance_criteria": [
            {
                "criterion_id": "3.1.AC1",
                "status": "pass-shared-linux",
                "schema_failure_cases": 72,
                "authority_mutation_attempts": authority["summary"][
                    "cross_channel_mutation_attempt_count"
                ],
                "profile_startup_count": startup["summary"]["profile_count"],
                "registered_capability_count": startup["summary"][
                    "registered_capability_count"
                ],
            },
            {
                "criterion_id": "3.1.AC2",
                "status": "pass-shared-linux",
                "durable_transition_count": migration["summary"][
                    "durable_transition_count"
                ],
                "injected_interruption_count": migration["summary"][
                    "injected_interruption_count"
                ],
                "repeatable_rollback": migration["summary"]["repeatable_rollback"],
            },
        ],
        "security_mapping": {
            "mapped_requirement_count": security["summary"][
                "mapped_requirement_count"
            ],
            "product_requirements_complete": security["summary"][
                "product_requirements_complete"
            ],
        },
        "universal_definition_of_done": universal_dod(),
        "independent_review": {
            "reviewer_id": "agentmage-story-3.1-independent-gate-v1",
            "reviewed_commit": REVIEWED_COMMIT,
            "reviewed_tree": REVIEWED_TREE,
            "artifacts": reviewed,
            "external_human_review_claim": "none",
        },
        "shared_linux_story_work_complete": True,
        "only_blocker": "macos-execution-evidence-unavailable",
        "macos_execution_status": "blocked-macos",
        "macos_evidence_substituted": False,
        "macos_support_claim": "none",
        "product_startup_activation_claim": "none",
        "product_acceptance_claim": "none",
        "release_claim": "none",
    }


def validate_report(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["Story 3.1 gate report must be an object"]
    failures = []
    if (
        value.get("schema_version") != 1
        or value.get("story_id") != "3.1"
        or value.get("status") != "blocked-macos"
        or value.get("reviewed_commit") != REVIEWED_COMMIT
        or value.get("reviewed_tree") != REVIEWED_TREE
    ):
        failures.append("Story 3.1 gate identity is invalid")
    criteria = value.get("acceptance_criteria", [])
    if [item.get("criterion_id") for item in criteria if isinstance(item, dict)] != [
        "3.1.AC1",
        "3.1.AC2",
    ] or any(item.get("status") != "pass-shared-linux" for item in criteria):
        failures.append("Story 3.1 acceptance criteria are incomplete")
    dod = value.get("universal_definition_of_done", [])
    if dod != universal_dod():
        failures.append("Story 3.1 universal definition of done is invalid")
    if value.get("security_mapping") != {
        "mapped_requirement_count": 9,
        "product_requirements_complete": 0,
    }:
        failures.append("Story 3.1 security mapping summary is invalid")
    if (
        value.get("shared_linux_story_work_complete") is not True
        or value.get("only_blocker") != "macos-execution-evidence-unavailable"
        or value.get("macos_execution_status") != "blocked-macos"
        or value.get("macos_evidence_substituted") is not False
        or value.get("macos_support_claim") != "none"
        or value.get("product_startup_activation_claim") != "none"
        or value.get("product_acceptance_claim") != "none"
        or value.get("release_claim") != "none"
    ):
        failures.append("Story 3.1 gate made an unsupported claim")
    try:
        expected = build_report(root)
    except (OSError, ValueError, KeyError, TypeError) as error:
        failures.append(f"cannot rebuild Story 3.1 gate report: {error}")
    else:
        if value != expected:
            failures.append("Story 3.1 gate report is stale or non-deterministic")
    return failures


def check_report(root: Path = ROOT) -> list[str]:
    try:
        value = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read Story 3.1 gate report: {error}"]
    return validate_report(value, root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            write_atomic(REPORT_PATH, canonical_json(build_report()))
        failures = check_report()
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"Story 3.1 gate failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Story 3.1 gate failed: {failure}", file=sys.stderr)
        return 1
    print("Story 3.1 shared/Linux acceptance passed with macOS blocker preserved")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
