#!/usr/bin/env python3
"""Evaluate Sprint 5 acceptance without substituting for macOS evidence."""

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
from scripts.story_5_1_gate import G_DOD_IDS, check_report as check_story_gate
from scripts.story_5_1_security_evidence import check_map as check_security


REPORT_PATH = ROOT / "artifacts/sprints/sprint-5/sprint-gate-report.json"
REVIEWED_COMMIT = "d25d50671a0879cb2b73c8488b4cdc6cee4f31a9"
REVIEWED_TREE = "847bcea3e5c65079cc1a1f37b122bfe0c47fcb7f"
REVIEWED_PATHS = (
    "artifacts/sprints/sprint-5/story-5.1/story-gate-report.json",
    "scripts/story_5_1_gate.py",
    "tests/test_story_5_1_gate.py",
    "artifacts/sprints/sprint-5/story-5.1/grant-policy-reference-report.json",
    "artifacts/sprints/sprint-5/story-5.1/grant-state-report.json",
    "artifacts/sprints/sprint-5/story-5.1/grant-review-fixture-report.json",
    "artifacts/sprints/sprint-5/story-5.1/adversarial-grant-corpus-report.json",
    "artifacts/sprints/sprint-5/story-5.1/grant-race-replay-report.json",
    "artifacts/sprints/sprint-5/story-5.1/authority-escalation-report.json",
    "artifacts/sprints/sprint-5/story-5.1/grant-stale-dispatch-report.json",
    "artifacts/sprints/sprint-5/story-5.1/grant-boundary-review.json",
    "artifacts/sprints/sprint-5/story-5.1/security-evidence-map.json",
    "fixtures/grants/adversarial/v1/manifest.json",
)
REQUIRED_STORY_MARKERS = (
    "- [x] **Story AC 5.1.AC1:**",
    "- [x] **Story AC 5.1.AC2:**",
)
REQUIRED_SPRINT_MARKERS = tuple(
    f"- [x] **Sprint AC 5.AC{index}:**" for index in range(1, 6)
)


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-sprint-5-gate-", dir=path.parent
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
        raise ValueError(f"git sprint review operation failed: {' '.join(arguments)}")
    return result.stdout if binary else result.stdout.decode("utf-8").strip()


def reviewed_artifacts(root: Path = ROOT) -> list[dict[str, str]]:
    commit = git_output("rev-parse", REVIEWED_COMMIT, root=root)
    tree = git_output("show", "-s", "--format=%T", REVIEWED_COMMIT, root=root)
    if commit != REVIEWED_COMMIT or tree != REVIEWED_TREE:
        raise ValueError("Sprint 5 review identity is unavailable or changed")
    records = []
    for path in REVIEWED_PATHS:
        committed = git_output("show", f"{REVIEWED_COMMIT}:{path}", root=root, binary=True)
        current = root / path
        if not current.is_file() or current.read_bytes() != committed:
            raise ValueError(f"reviewed Sprint 5 artifact changed after review: {path}")
        records.append({"path": path, "sha256": sha256_bytes(committed)})
    return records


def checklist_failures(tasks_text: str) -> list[str]:
    failures = [
        marker
        for marker in (*REQUIRED_STORY_MARKERS, *REQUIRED_SPRINT_MARKERS)
        if marker not in tasks_text
    ]
    if "### [ ] Sprint 5 - Capability Grants and Policy Engine" not in tasks_text:
        failures.append("Sprint 5 checkbox must remain open while macOS is blocked")
    if "#### [ ] Story 5.1 - Capability Grants and Policy Engine" not in tasks_text:
        failures.append("Story 5.1 checkbox must remain open while macOS is blocked")
    return failures


def validate_inputs(root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    try:
        check_boundary_review(root)
    except (OSError, RuntimeError, ValueError, subprocess.SubprocessError) as error:
        failures.append(f"boundary-review: {error}")
    failures.extend(f"security-map: {failure}" for failure in check_security(root))
    failures.extend(f"story-5.1-gate: {failure}" for failure in check_story_gate(root))
    try:
        tasks_text = (root / "TASKS.md").read_text(encoding="utf-8")
    except OSError as error:
        failures.append(f"cannot read TASKS.md: {error}")
    else:
        failures.extend(checklist_failures(tasks_text))
    return failures


def story_gate_summary(value: dict[str, Any]) -> dict[str, Any]:
    dod = value["universal_definition_of_done"]
    return {
        "story_id": "5.1",
        "status": value["status"],
        "acceptance_criteria_passed": len(value["acceptance_criteria"]),
        "acceptance_criteria_failed": 0,
        "shared_linux_story_work_complete": value[
            "shared_linux_story_work_complete"
        ],
        "story_checkbox_complete": value["story_checkbox_complete"],
        "only_blocker": value["only_blocker"],
        "dod_control_ids": [item["control_id"] for item in dod],
        "dod_blocking_controls": [
            item["control_id"] for item in dod if item["status"] == "blocked-macos"
        ],
    }


def build_report(root: Path = ROOT) -> dict[str, Any]:
    failures = validate_inputs(root)
    if failures:
        raise ValueError("; ".join(failures))
    policy = read_json(
        root / "artifacts/sprints/sprint-5/story-5.1/grant-policy-reference-report.json"
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
    story = read_json(
        root / "artifacts/sprints/sprint-5/story-5.1/story-gate-report.json"
    )
    return {
        "schema_version": 1,
        "sprint_id": 5,
        "status": "blocked-macos",
        "reviewed_commit": REVIEWED_COMMIT,
        "reviewed_tree": REVIEWED_TREE,
        "acceptance_criteria": [
            {
                "criterion_id": "5.AC1",
                "status": "pass-shared-linux-in-memory",
                "acceptance_test_id": "AT-AUTH-001",
                "mutation_seed_count": adversarial["coverage"]["case_count"],
                "admitted_mutation_count": adversarial["coverage"][
                    "admitted_attempt_count"
                ],
                "authority_escalation_case_count": escalation["coverage"][
                    "receipt_count"
                ],
                "admitted_escalation_count": escalation["coverage"][
                    "admitted_authority_count"
                ],
                "production_executor_claim": "none",
            },
            {
                "criterion_id": "5.AC2",
                "status": "pass-shared-linux-in-memory",
                "accepted_scenario_count": race["coverage"][
                    "attempt_scenarios_with_one_consumption"
                ],
                "maximum_worker_start_count": race["coverage"][
                    "maximum_worker_start_count"
                ],
                "maximum_effect_count": race["coverage"]["maximum_effect_count"],
                "replay_success_count": race["coverage"]["replay_success_count"],
                "durable_transaction_claim": "none",
            },
            {
                "criterion_id": "5.AC3",
                "status": "pass-shared-linux-in-memory",
                "mutation_class_count": adversarial["coverage"][
                    "mutation_class_count"
                ],
                "mutation_seed_count": adversarial["coverage"]["case_count"],
                "admitted_mutation_count": adversarial["coverage"][
                    "admitted_attempt_count"
                ],
                "post_approval_mutation_count": stale["coverage"]["mutation_count"],
                "post_approval_worker_start_count": stale["coverage"][
                    "worker_start_count"
                ],
            },
            {
                "criterion_id": "5.AC4",
                "status": "pass-shared-linux-contract-boundary",
                "source_count": escalation["coverage"]["source_count"],
                "escalation_kind_count": escalation["coverage"][
                    "escalation_kind_count"
                ],
                "attempt_count": escalation["coverage"]["receipt_count"],
                "admitted_authority_count": escalation["coverage"][
                    "admitted_authority_count"
                ],
            },
            {
                "criterion_id": "5.AC5",
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
            },
        ],
        "story_gates": [story_gate_summary(story)],
        "universal_definition_of_done": {
            "control_ids": list(G_DOD_IDS),
            "story_count": 1,
            "all_non_platform_controls_pass_or_not_applicable": True,
            "blocking_controls": ["G-DOD-10"],
            "macos_evidence_substitution": "prohibited",
        },
        "summary": {
            "acceptance_criteria_passed": 5,
            "acceptance_criteria_failed": 0,
            "story_gate_count": 1,
            "shared_linux_sprint_work_complete": True,
            "sprint_checkbox_complete": False,
            "blocking_story_count": 1,
            "blocking_story_ids": ["5.1"],
            "blocking_control_count": 1,
            "blocking_controls": ["G-DOD-10"],
        },
        "independent_review": {
            "reviewer_id": "agentmage-sprint-5-independent-gate-v1",
            "review_type": "automated-independent-aggregate-review",
            "reviewed_commit": REVIEWED_COMMIT,
            "reviewed_tree": REVIEWED_TREE,
            "reviewed_artifacts": reviewed_artifacts(root),
            "finding_count": 0,
            "findings": [],
            "disposition": "pass-shared-linux-sprint-blocked-macos",
            "external_human_review_status": "not-performed",
        },
        "macos": {
            "status": "blocked-macos",
            "execution_performed": False,
            "evidence_substitution": "prohibited",
            "support_claim": "none",
        },
        "product_requirement_completion_claim": "none",
        "product_acceptance_claim": "none",
        "release_claim": "none",
    }


def validate_report(
    value: Any, root: Path = ROOT, *, verify_current: bool = True
) -> list[str]:
    if not isinstance(value, dict):
        return ["Sprint 5 gate report must be an object"]
    failures: list[str] = []
    if (
        value.get("schema_version") != 1
        or value.get("sprint_id") != 5
        or value.get("status") != "blocked-macos"
        or value.get("reviewed_commit") != REVIEWED_COMMIT
        or value.get("reviewed_tree") != REVIEWED_TREE
    ):
        failures.append("Sprint 5 gate identity or review boundary is invalid")
    criteria = value.get("acceptance_criteria", [])
    if [item.get("criterion_id") for item in criteria if isinstance(item, dict)] != [
        "5.AC1",
        "5.AC2",
        "5.AC3",
        "5.AC4",
        "5.AC5",
    ]:
        failures.append("Sprint 5 acceptance criterion closure is invalid")
    elif (
        criteria[0].get("status") != "pass-shared-linux-in-memory"
        or criteria[0].get("acceptance_test_id") != "AT-AUTH-001"
        or criteria[0].get("mutation_seed_count") != 560
        or criteria[0].get("admitted_mutation_count") != 0
        or criteria[0].get("authority_escalation_case_count") != 28
        or criteria[0].get("admitted_escalation_count") != 0
        or criteria[0].get("production_executor_claim") != "none"
        or criteria[1].get("accepted_scenario_count") != 4
        or criteria[1].get("maximum_worker_start_count") != 1
        or criteria[1].get("maximum_effect_count") != 1
        or criteria[1].get("replay_success_count") != 0
        or criteria[1].get("durable_transaction_claim") != "none"
        or criteria[2].get("mutation_class_count") != 14
        or criteria[2].get("mutation_seed_count") != 560
        or criteria[2].get("admitted_mutation_count") != 0
        or criteria[2].get("post_approval_mutation_count") != 4
        or criteria[2].get("post_approval_worker_start_count") != 0
        or criteria[3].get("source_count") != 7
        or criteria[3].get("escalation_kind_count") != 4
        or criteria[3].get("attempt_count") != 28
        or criteria[3].get("admitted_authority_count") != 0
        or criteria[4].get("grant_operation_count") != 15
        or criteria[4].get("strict_explicit_denial_count") != 12
        or criteria[4].get("strict_denied_by_absence_count") != 2
        or criteria[4].get("approval_forbidden_authority_field_count") != 0
    ):
        failures.append("Sprint 5 acceptance evidence is invalid")
    stories = value.get("story_gates", [])
    if len(stories) != 1 or any(
        item.get("story_id") != "5.1"
        or item.get("status") != "blocked-macos"
        or item.get("acceptance_criteria_passed") != 2
        or item.get("acceptance_criteria_failed") != 0
        or item.get("shared_linux_story_work_complete") is not True
        or item.get("story_checkbox_complete") is not False
        or item.get("only_blocker") != "macos-execution-evidence-unavailable"
        or item.get("dod_control_ids") != list(G_DOD_IDS)
        or item.get("dod_blocking_controls") != ["G-DOD-10"]
        for item in stories
    ):
        failures.append("Sprint 5 story-gate aggregation is invalid")
    if value.get("universal_definition_of_done") != {
        "control_ids": list(G_DOD_IDS),
        "story_count": 1,
        "all_non_platform_controls_pass_or_not_applicable": True,
        "blocking_controls": ["G-DOD-10"],
        "macos_evidence_substitution": "prohibited",
    }:
        failures.append("Sprint 5 Definition-of-Done aggregation is invalid")
    if value.get("summary") != {
        "acceptance_criteria_passed": 5,
        "acceptance_criteria_failed": 0,
        "story_gate_count": 1,
        "shared_linux_sprint_work_complete": True,
        "sprint_checkbox_complete": False,
        "blocking_story_count": 1,
        "blocking_story_ids": ["5.1"],
        "blocking_control_count": 1,
        "blocking_controls": ["G-DOD-10"],
    }:
        failures.append("Sprint 5 gate summary is invalid")
    review = value.get("independent_review", {})
    if (
        review.get("reviewer_id") != "agentmage-sprint-5-independent-gate-v1"
        or review.get("review_type") != "automated-independent-aggregate-review"
        or review.get("reviewed_commit") != REVIEWED_COMMIT
        or review.get("reviewed_tree") != REVIEWED_TREE
        or len(review.get("reviewed_artifacts", [])) != len(REVIEWED_PATHS)
        or review.get("finding_count") != 0
        or review.get("findings") != []
        or review.get("disposition")
        != "pass-shared-linux-sprint-blocked-macos"
        or review.get("external_human_review_status") != "not-performed"
    ):
        failures.append("Sprint 5 independent review is invalid")
    if value.get("macos") != {
        "status": "blocked-macos",
        "execution_performed": False,
        "evidence_substitution": "prohibited",
        "support_claim": "none",
    }:
        failures.append("Sprint 5 macOS blocker is invalid")
    if (
        value.get("product_requirement_completion_claim") != "none"
        or value.get("product_acceptance_claim") != "none"
        or value.get("release_claim") != "none"
    ):
        failures.append("Sprint 5 gate made an unsupported claim")
    if verify_current:
        try:
            expected = build_report(root)
        except (OSError, ValueError, KeyError, TypeError) as error:
            failures.append(f"cannot rebuild Sprint 5 gate report: {error}")
        else:
            if value != expected:
                failures.append("Sprint 5 gate report is stale or non-deterministic")
    return failures


def check_report(root: Path = ROOT) -> list[str]:
    try:
        value = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read Sprint 5 gate report: {error}"]
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
        print(f"Sprint 5 gate failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Sprint 5 gate failed: {failure}", file=sys.stderr)
        return 1
    print("Sprint 5 shared/Linux work passes; gate remains blocked on macOS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
