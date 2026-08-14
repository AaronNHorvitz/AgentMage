#!/usr/bin/env python3
"""Independently evaluate Story 5.1 while preserving the macOS blocker."""

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

from scripts.grant_boundary_review import check_report as check_boundary_review
from scripts.story_5_1_security_evidence import check_map as check_security


REPORT_PATH = ROOT / "artifacts/sprints/sprint-5/story-5.1/story-gate-report.json"
REVIEWED_COMMIT = "812b5f6b12f5e5de97723751e6bdfc4fc094e112"
REVIEWED_TREE = "1ce707d86d1e3fb9085411226443fbce3b64c15f"
REVIEWED_PATHS = (
    "docs/architecture/grant-policy-reference.md",
    "docs/architecture/grant-state-transitions.md",
    "fixtures/grants/v1/manifest.json",
    "fixtures/grants/adversarial/v1/corpus.json",
    "fixtures/grants/adversarial/v1/manifest.json",
    "kernel/contracts/src/approval.rs",
    "kernel/contracts/src/grant.rs",
    "kernel/contracts/src/operation.rs",
    "kernel/engine/src/approval.rs",
    "kernel/engine/src/authority.rs",
    "kernel/engine/src/grants.rs",
    "kernel/engine/src/policy.rs",
    "kernel/engine/tests/adversarial_grant_corpus.rs",
    "kernel/engine/tests/authority_escalation_matrix.rs",
    "kernel/engine/tests/grant_race_replay.rs",
    "kernel/engine/tests/grant_stale_dispatch.rs",
    "artifacts/sprints/sprint-5/story-5.1/grant-policy-reference-report.json",
    "artifacts/sprints/sprint-5/story-5.1/grant-state-report.json",
    "artifacts/sprints/sprint-5/story-5.1/grant-review-fixture-report.json",
    "artifacts/sprints/sprint-5/story-5.1/adversarial-grant-corpus-report.json",
    "artifacts/sprints/sprint-5/story-5.1/grant-race-replay-report.json",
    "artifacts/sprints/sprint-5/story-5.1/authority-escalation-report.json",
    "artifacts/sprints/sprint-5/story-5.1/grant-stale-dispatch-report.json",
    "artifacts/sprints/sprint-5/story-5.1/grant-boundary-review.json",
    "artifacts/sprints/sprint-5/story-5.1/security-evidence-map.json",
)
REQUIRED_TASK_MARKERS = (
    "- [x] **Task 5.1.1 - Implement the bounded story**",
    "- [x] **Task 5.1.2 - Produce reviewable artifacts**",
    "- [x] **Task 5.1.3 - Verify and close the story**",
    "  - [x] **Sub-task 5.1.3.1:**",
    "  - [x] **Sub-task 5.1.3.2:**",
    "  - [x] **Sub-task 5.1.3.3:**",
    "  - [x] **Sub-task 5.1.3.4:**",
    "  - [x] **Sub-task 5.1.3.5 - Product security evidence:**",
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
        prefix=".agentmage-story-5-1-gate-", dir=path.parent
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
        raise ValueError("Story 5.1 review identity is unavailable or changed")
    records = []
    for path in REVIEWED_PATHS:
        committed = git_output("show", f"{REVIEWED_COMMIT}:{path}", root=root, binary=True)
        current = root / path
        if not current.is_file() or current.read_bytes() != committed:
            raise ValueError(f"reviewed Story 5.1 artifact changed after review: {path}")
        records.append({"path": path, "sha256": sha256_bytes(committed)})
    return records


def task_completion(tasks_text: str) -> list[str]:
    return [marker for marker in REQUIRED_TASK_MARKERS if marker not in tasks_text]


def validate_inputs(root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    try:
        check_boundary_review(root)
    except (OSError, RuntimeError, ValueError, subprocess.SubprocessError) as error:
        failures.append(f"boundary-review: {error}")
    failures.extend(f"security-map: {failure}" for failure in check_security(root))
    try:
        tasks_text = (root / "TASKS.md").read_text(encoding="utf-8")
    except OSError as error:
        failures.append(f"cannot read TASKS.md: {error}")
    else:
        failures.extend(
            f"task marker is incomplete: {marker}"
            for marker in task_completion(tasks_text)
        )
    return failures


def universal_dod() -> list[dict[str, Any]]:
    statuses = {
        "G-DOD-01": "pass-story-scope",
        "G-DOD-02": "pass-story-scope",
        "G-DOD-03": "pass-story-scope",
        "G-DOD-04": "pass-story-scope-exact-grant",
        "G-DOD-05": "pass-story-scope",
        "G-DOD-06": "not-applicable-no-production-tool-attempt",
        "G-DOD-07": "pass-no-persisted-user-data",
        "G-DOD-08": "pass-story-scope",
        "G-DOD-09": "pass-story-scope",
        "G-DOD-10": "blocked-macos",
        "G-DOD-11": "pass-story-scope",
        "G-DOD-12": "pass-independent-automated-boundary-review",
        "G-DOD-13": "pass-gate-reporting",
    }
    return [
        {
            "control_id": control_id,
            "status": statuses[control_id],
            "evidence": [
                "artifacts/sprints/sprint-5/story-5.1/grant-boundary-review.json",
                "artifacts/sprints/sprint-5/story-5.1/security-evidence-map.json",
                "artifacts/sprints/sprint-5/story-5.1/story-gate-report.json",
            ],
        }
        for control_id in G_DOD_IDS
    ]


def build_report(root: Path = ROOT) -> dict[str, Any]:
    failures = validate_inputs(root)
    if failures:
        raise ValueError("; ".join(failures))
    reviewed = reviewed_artifacts(root)
    policy = read_json(
        root / "artifacts/sprints/sprint-5/story-5.1/grant-policy-reference-report.json"
    )
    state = read_json(
        root / "artifacts/sprints/sprint-5/story-5.1/grant-state-report.json"
    )
    fixtures = read_json(
        root / "artifacts/sprints/sprint-5/story-5.1/grant-review-fixture-report.json"
    )
    adversarial = read_json(
        root / "artifacts/sprints/sprint-5/story-5.1/adversarial-grant-corpus-report.json"
    )
    race = read_json(
        root / "artifacts/sprints/sprint-5/story-5.1/grant-race-replay-report.json"
    )
    escalation = read_json(
        root / "artifacts/sprints/sprint-5/story-5.1/authority-escalation-report.json"
    )
    stale = read_json(
        root / "artifacts/sprints/sprint-5/story-5.1/grant-stale-dispatch-report.json"
    )
    security = read_json(
        root / "artifacts/sprints/sprint-5/story-5.1/security-evidence-map.json"
    )
    return {
        "schema_version": 1,
        "story_id": "5.1",
        "status": "blocked-macos",
        "reviewed_commit": REVIEWED_COMMIT,
        "reviewed_tree": REVIEWED_TREE,
        "acceptance_criteria": [
            {
                "criterion_id": "5.1.AC1",
                "status": "pass-shared-linux-in-memory",
                "mutation_seed_count": adversarial["coverage"]["case_count"],
                "admitted_mutation_count": adversarial["coverage"][
                    "admitted_attempt_count"
                ],
                "race_scenario_count": race["coverage"]["scenario_count"],
                "replay_success_count": race["coverage"]["replay_success_count"],
                "stale_dispatch_case_count": stale["coverage"]["mutation_count"],
                "stale_worker_start_count": stale["coverage"]["worker_start_count"],
                "implemented_transition_count": state["coverage"][
                    "implemented_transition_count"
                ],
                "production_executor_claim": "none",
            },
            {
                "criterion_id": "5.1.AC2",
                "status": "pass-shared-linux-contract-and-policy",
                "grant_operation_count": policy["coverage"]["grant_operation_count"],
                "strict_explicit_denial_count": policy["coverage"][
                    "strict_explicit_denial_count"
                ],
                "strict_denied_by_absence_count": len(
                    policy["coverage"]["strict_denied_by_absence"]
                ),
                "approval_forbidden_authority_field_count": fixtures["coverage"][
                    "approval_forbidden_authority_field_count"
                ],
                "authority_escalation_case_count": escalation["coverage"][
                    "receipt_count"
                ],
                "admitted_authority_count": escalation["coverage"][
                    "admitted_authority_count"
                ],
            },
        ],
        "security_mapping": {
            "mapped_requirement_count": security["summary"][
                "mapped_requirement_count"
            ],
            "product_requirements_complete": security["summary"][
                "product_requirements_complete"
            ],
            "retained_artifact_count": security["summary"][
                "retained_artifact_count"
            ],
        },
        "universal_definition_of_done": universal_dod(),
        "independent_review": {
            "reviewer_id": "agentmage-story-5.1-independent-gate-v1",
            "review_type": "automated-independent-aggregate-review",
            "reviewed_commit": REVIEWED_COMMIT,
            "reviewed_tree": REVIEWED_TREE,
            "artifacts": reviewed,
            "finding_count": 0,
            "findings": [],
            "external_human_review_status": "not-performed",
        },
        "shared_linux_story_work_complete": True,
        "story_checkbox_complete": False,
        "only_blocker": "macos-execution-evidence-unavailable",
        "macos_execution_status": "blocked-macos",
        "macos_evidence_substituted": False,
        "macos_support_claim": "none",
        "product_requirement_completion_claim": "none",
        "product_acceptance_claim": "none",
        "release_claim": "none",
    }


def validate_report(
    value: Any, root: Path = ROOT, *, verify_current: bool = True
) -> list[str]:
    if not isinstance(value, dict):
        return ["Story 5.1 gate report must be an object"]
    failures: list[str] = []
    if (
        value.get("schema_version") != 1
        or value.get("story_id") != "5.1"
        or value.get("status") != "blocked-macos"
        or value.get("reviewed_commit") != REVIEWED_COMMIT
        or value.get("reviewed_tree") != REVIEWED_TREE
    ):
        failures.append("Story 5.1 gate identity is invalid")
    criteria = value.get("acceptance_criteria", [])
    if [item.get("criterion_id") for item in criteria if isinstance(item, dict)] != [
        "5.1.AC1",
        "5.1.AC2",
    ]:
        failures.append("Story 5.1 acceptance criterion closure is invalid")
    elif (
        criteria[0].get("status") != "pass-shared-linux-in-memory"
        or criteria[0].get("mutation_seed_count") != 560
        or criteria[0].get("admitted_mutation_count") != 0
        or criteria[0].get("race_scenario_count") != 5
        or criteria[0].get("replay_success_count") != 0
        or criteria[0].get("stale_dispatch_case_count") != 4
        or criteria[0].get("stale_worker_start_count") != 0
        or criteria[0].get("implemented_transition_count") != 8
        or criteria[0].get("production_executor_claim") != "none"
        or criteria[1].get("status")
        != "pass-shared-linux-contract-and-policy"
        or criteria[1].get("grant_operation_count") != 22
        or criteria[1].get("strict_explicit_denial_count") != 21
        or criteria[1].get("strict_denied_by_absence_count") != 0
        or criteria[1].get("approval_forbidden_authority_field_count") != 0
        or criteria[1].get("authority_escalation_case_count") != 28
        or criteria[1].get("admitted_authority_count") != 0
    ):
        failures.append("Story 5.1 acceptance evidence is invalid")
    if value.get("security_mapping") != {
        "mapped_requirement_count": 8,
        "product_requirements_complete": 0,
        "retained_artifact_count": 31,
    }:
        failures.append("Story 5.1 security mapping summary is invalid")
    if value.get("universal_definition_of_done") != universal_dod():
        failures.append("Story 5.1 universal definition of done is invalid")
    review = value.get("independent_review", {})
    if (
        review.get("reviewer_id") != "agentmage-story-5.1-independent-gate-v1"
        or review.get("review_type") != "automated-independent-aggregate-review"
        or review.get("reviewed_commit") != REVIEWED_COMMIT
        or review.get("reviewed_tree") != REVIEWED_TREE
        or review.get("finding_count") != 0
        or review.get("findings") != []
        or review.get("external_human_review_status") != "not-performed"
        or len(review.get("artifacts", [])) != len(REVIEWED_PATHS)
    ):
        failures.append("Story 5.1 independent review is invalid")
    if (
        value.get("shared_linux_story_work_complete") is not True
        or value.get("story_checkbox_complete") is not False
        or value.get("only_blocker") != "macos-execution-evidence-unavailable"
        or value.get("macos_execution_status") != "blocked-macos"
        or value.get("macos_evidence_substituted") is not False
        or value.get("macos_support_claim") != "none"
        or value.get("product_requirement_completion_claim") != "none"
        or value.get("product_acceptance_claim") != "none"
        or value.get("release_claim") != "none"
    ):
        failures.append("Story 5.1 gate made an unsupported claim")
    if verify_current:
        try:
            expected = build_report(root)
        except (OSError, ValueError, KeyError, TypeError) as error:
            failures.append(f"cannot rebuild Story 5.1 gate report: {error}")
        else:
            if value != expected:
                failures.append("Story 5.1 gate report is stale or non-deterministic")
    return failures


def check_report(root: Path = ROOT) -> list[str]:
    try:
        value = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read Story 5.1 gate report: {error}"]
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
        print(f"Story 5.1 gate failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Story 5.1 gate failed: {failure}", file=sys.stderr)
        return 1
    print("Story 5.1 shared/Linux work passes; gate remains blocked on macOS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
