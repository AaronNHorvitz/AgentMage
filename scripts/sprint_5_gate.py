#!/usr/bin/env python3
"""Evaluate the complete current Sprint 5 scope without widening evidence."""

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

from scripts.story_5_1_gate import check_report as check_story_5_1_gate
from scripts.story_5_2_gate import validate_report as validate_story_5_2_gate
from scripts.story_5_3_gate import validate_report as validate_story_5_3_gate


REPORT_PATH = ROOT / "artifacts/sprints/sprint-5/sprint-gate-report.json"
REVIEWED_COMMIT = "1d5a9bb0274e048303d028fd88993085d9c386e2"
REVIEWED_TREE = "0751b6cac4c443d4624a4d138639d7ed93cedcab"
REVIEWED_PATHS = (
    "artifacts/sprints/sprint-5/story-5.1/story-gate-report.json",
    "scripts/story_5_1_gate.py",
    "tests/test_story_5_1_gate.py",
    "artifacts/sprints/sprint-5/story-5.2/story-gate-report.json",
    "scripts/story_5_2_gate.py",
    "tests/test_story_5_2_gate.py",
    "artifacts/sprints/sprint-5/story-5.3/story-gate-report.json",
    "scripts/story_5_3_gate.py",
    "tests/test_story_5_3_gate.py",
    "artifacts/sprints/sprint-5/story-5.2/story-ac1-deterministic-disposition-report.json",
    "artifacts/sprints/sprint-5/story-5.2/story-ac2-no-replay-recovery-report.json",
)
G_DOD_IDS = tuple(f"G-DOD-{index:02d}" for index in range(1, 14))
REQUIRED_STORY_MARKERS = tuple(
    f"- [x] **Story AC {story}.AC{criterion}:**"
    for story, criterion_count in (("5.1", 2), ("5.2", 3), ("5.3", 3))
    for criterion in range(1, criterion_count + 1)
)
REQUIRED_SPRINT_MARKERS = tuple(
    f"- [x] **Sprint AC 5.AC{index}:**" for index in range(1, 7)
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
        ["git", *arguments], cwd=root, stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False, timeout=15,
    )
    if result.returncode != 0:
        raise ValueError(f"git sprint review operation failed: {' '.join(arguments)}")
    return result.stdout if binary else result.stdout.decode("utf-8").strip()


def reviewed_artifacts(root: Path = ROOT) -> list[dict[str, Any]]:
    if git_output("rev-parse", REVIEWED_COMMIT, root=root) != REVIEWED_COMMIT:
        raise ValueError("Sprint 5 reviewed commit is unavailable")
    if git_output("show", "-s", "--format=%T", REVIEWED_COMMIT, root=root) != REVIEWED_TREE:
        raise ValueError("Sprint 5 reviewed tree changed")
    records = []
    for path in REVIEWED_PATHS:
        committed = git_output("show", f"{REVIEWED_COMMIT}:{path}", root=root, binary=True)
        current = root / path
        if not current.is_file() or current.read_bytes() != committed:
            raise ValueError(f"reviewed Sprint 5 artifact changed after review: {path}")
        records.append(
            {"path": path, "byte_length": len(committed), "sha256": sha256_bytes(committed)}
        )
    return records


def checklist_failures(tasks_text: str) -> list[str]:
    failures = [
        marker for marker in (*REQUIRED_STORY_MARKERS, *REQUIRED_SPRINT_MARKERS)
        if marker not in tasks_text
    ]
    required_open = (
        "### [ ] Sprint 5 - Capability Grants and Policy Engine",
        "#### [ ] Story 5.1 - Capability Grants and Policy Engine",
        "#### [ ] Story 5.2 - Side-Effect, Idempotency, and Retry Policy",
        "#### [ ] Story 5.3 - Verified Workflow Definition and Completion Authority",
    )
    failures.extend(marker for marker in required_open if marker not in tasks_text)
    return failures


def validate_inputs(root: Path = ROOT) -> list[str]:
    failures = [f"story-5.1-gate: {item}" for item in check_story_5_1_gate(root)]
    story_5_2 = read_json(root / "artifacts/sprints/sprint-5/story-5.2/story-gate-report.json")
    story_5_3 = read_json(root / "artifacts/sprints/sprint-5/story-5.3/story-gate-report.json")
    failures.extend(f"story-5.2-gate: {item}" for item in validate_story_5_2_gate(story_5_2))
    failures.extend(f"story-5.3-gate: {item}" for item in validate_story_5_3_gate(story_5_3))
    failures.extend(checklist_failures((root / "TASKS.md").read_text(encoding="utf-8")))
    return failures


def story_gate_summary(story_id: str, value: dict[str, Any]) -> dict[str, Any]:
    if story_id == "5.1":
        blockers = [value["only_blocker"]]
        blocking_controls = [
            item["control_id"] for item in value["universal_definition_of_done"]
            if item["status"].startswith("blocked-")
        ]
        current_scope_complete = value["shared_linux_story_work_complete"]
    else:
        blockers = value["blockers"]
        blocking_controls = value["blocking_controls"]
        scope_key = (
            "current_linux_contract_policy_and_supervision_scope_complete"
            if story_id == "5.2"
            else "current_linux_workflow_contract_and_verifier_scope_complete"
        )
        current_scope_complete = value[scope_key]
    return {
        "story_id": story_id,
        "status": value["status"],
        "acceptance_criteria_passed": len(value["acceptance_criteria"]),
        "acceptance_criteria_failed": 0,
        "current_linux_scope_complete": current_scope_complete,
        "story_checkbox_complete": value["story_checkbox_complete"],
        "blockers": blockers,
        "blocking_controls": blocking_controls,
    }


def build_report(root: Path = ROOT) -> dict[str, Any]:
    failures = validate_inputs(root)
    if failures:
        raise ValueError("; ".join(failures))
    story_5_1 = read_json(root / "artifacts/sprints/sprint-5/story-5.1/story-gate-report.json")
    story_5_2 = read_json(root / "artifacts/sprints/sprint-5/story-5.2/story-gate-report.json")
    story_5_3 = read_json(root / "artifacts/sprints/sprint-5/story-5.3/story-gate-report.json")
    policy = read_json(root / "artifacts/sprints/sprint-5/story-5.1/grant-policy-reference-report.json")
    adversarial = read_json(root / "artifacts/sprints/sprint-5/story-5.1/adversarial-grant-corpus-report.json")
    race = read_json(root / "artifacts/sprints/sprint-5/story-5.1/grant-race-replay-report.json")
    escalation = read_json(root / "artifacts/sprints/sprint-5/story-5.1/authority-escalation-report.json")
    stale = read_json(root / "artifacts/sprints/sprint-5/story-5.1/grant-stale-dispatch-report.json")
    fixtures = read_json(root / "artifacts/sprints/sprint-5/story-5.1/grant-review-fixture-report.json")
    effect_truth = read_json(
        root / "artifacts/sprints/sprint-5/story-5.2/story-ac1-deterministic-disposition-report.json"
    )["acceptance_truth"]
    retry_truth = read_json(
        root / "artifacts/sprints/sprint-5/story-5.2/story-ac2-no-replay-recovery-report.json"
    )["acceptance_truth"]
    stories = [
        story_gate_summary("5.1", story_5_1),
        story_gate_summary("5.2", story_5_2),
        story_gate_summary("5.3", story_5_3),
    ]
    return {
        "schema_version": 2,
        "sprint_id": 5,
        "status": "blocked-open-dependencies-and-platform",
        "reviewed_commit": REVIEWED_COMMIT,
        "reviewed_tree": REVIEWED_TREE,
        "acceptance_criteria": [
            {
                "criterion_id": "5.AC1", "status": "pass-current-linux-in-memory",
                "acceptance_test_id": "AT-AUTH-001",
                "mutation_seed_count": adversarial["coverage"]["case_count"],
                "admitted_mutation_count": adversarial["coverage"]["admitted_attempt_count"],
                "authority_escalation_case_count": escalation["coverage"]["receipt_count"],
                "admitted_escalation_count": escalation["coverage"]["admitted_authority_count"],
                "production_executor_claim": "none",
            },
            {
                "criterion_id": "5.AC2", "status": "pass-current-linux-in-memory",
                "accepted_scenario_count": race["coverage"]["attempt_scenarios_with_one_consumption"],
                "maximum_worker_start_count": race["coverage"]["maximum_worker_start_count"],
                "maximum_effect_count": race["coverage"]["maximum_effect_count"],
                "replay_success_count": race["coverage"]["replay_success_count"],
                "durable_transaction_claim": "none",
            },
            {
                "criterion_id": "5.AC3", "status": "pass-current-linux-in-memory",
                "mutation_class_count": adversarial["coverage"]["mutation_class_count"],
                "mutation_seed_count": adversarial["coverage"]["case_count"],
                "admitted_mutation_count": adversarial["coverage"]["admitted_attempt_count"],
                "post_approval_mutation_count": stale["coverage"]["mutation_count"],
                "post_approval_worker_start_count": stale["coverage"]["worker_start_count"],
            },
            {
                "criterion_id": "5.AC4", "status": "pass-current-linux-contract-boundary",
                "source_count": escalation["coverage"]["source_count"],
                "escalation_kind_count": escalation["coverage"]["escalation_kind_count"],
                "attempt_count": escalation["coverage"]["receipt_count"],
                "admitted_authority_count": escalation["coverage"]["admitted_authority_count"],
            },
            {
                "criterion_id": "5.AC5", "status": "pass-current-linux-contract-and-policy",
                "grant_operation_count": policy["coverage"]["grant_operation_count"],
                "strict_explicit_denial_count": policy["coverage"]["strict_explicit_denial_count"],
                "strict_denied_by_absence_count": len(policy["coverage"]["strict_denied_by_absence"]),
                "approval_forbidden_authority_field_count": fixtures["coverage"]["approval_forbidden_authority_field_count"],
            },
            {
                "criterion_id": "5.AC6", "status": "pass-current-linux-contract-and-retry-policy",
                "registered_operation_count": effect_truth["registered_operation_count"],
                "effect_class_count": effect_truth["effect_class_count"],
                "failure_class_count": effect_truth["failure_class_count"],
                "operation_failure_pair_count": effect_truth["operation_failure_pair_count"],
                "one_effect_class_per_registered_operation": effect_truth["one_effect_class_per_registered_operation"],
                "prior_identity_family_count": retry_truth["prior_identity_family_count"],
                "old_call_replayed": retry_truth["old_call_replayed"],
                "old_authority_object_reused": retry_truth["old_authority_object_reused"],
                "uncertain_outcome_retried": retry_truth["uncertain_outcome_retried"],
            },
        ],
        "story_gates": stories,
        "universal_definition_of_done": {
            "control_ids": list(G_DOD_IDS), "story_count": 3,
            "all_non_platform_controls_pass_or_not_applicable": True,
            "blocking_controls": ["G-DOD-10"],
            "platform_evidence_substitution": "prohibited",
        },
        "summary": {
            "acceptance_criteria_passed": 6, "acceptance_criteria_failed": 0,
            "story_gate_count": 3, "current_linux_sprint_scope_complete": True,
            "sprint_checkbox_complete": False, "blocking_story_count": 3,
            "blocking_story_ids": ["5.1", "5.2", "5.3"],
            "open_dependency_ids": ["1.3", "2.3", "2.4", "5.1", "5.2"],
            "blocking_control_count": 1, "blocking_controls": ["G-DOD-10"],
        },
        "independent_review": {
            "reviewer_id": "agentmage-sprint-5-independent-gate-v2",
            "review_type": "automated-independent-aggregate-review",
            "reviewed_commit": REVIEWED_COMMIT, "reviewed_tree": REVIEWED_TREE,
            "reviewed_artifacts": reviewed_artifacts(root), "finding_count": 0,
            "findings": [],
            "disposition": "pass-current-linux-sprint-blocked-open-dependencies-and-platform",
            "external_human_review_status": "not-performed",
        },
        "platform": {
            "status": "blocked-supported-platform-installed-product-matrix",
            "execution_performed": False, "evidence_substitution": "prohibited",
            "support_claim": "none",
        },
        "installed_product_claim": "none",
        "product_requirement_completion_claim": "none",
        "product_acceptance_claim": "none",
        "release_claim": "none",
    }


def validate_report(value: Any, root: Path = ROOT, *, verify_current: bool = True) -> list[str]:
    if not isinstance(value, dict):
        return ["Sprint 5 gate report must be an object"]
    failures: list[str] = []
    if (
        value.get("schema_version") != 2 or value.get("sprint_id") != 5
        or value.get("status") != "blocked-open-dependencies-and-platform"
        or value.get("reviewed_commit") != REVIEWED_COMMIT
        or value.get("reviewed_tree") != REVIEWED_TREE
    ):
        failures.append("Sprint 5 gate identity or review boundary is invalid")
    criteria = value.get("acceptance_criteria", [])
    if [item.get("criterion_id") for item in criteria if isinstance(item, dict)] != [f"5.AC{i}" for i in range(1, 7)]:
        failures.append("Sprint 5 acceptance criterion closure is invalid")
    elif (
        criteria[0].get("admitted_mutation_count") != 0
        or criteria[0].get("admitted_escalation_count") != 0
        or criteria[1].get("maximum_worker_start_count") != 1
        or criteria[1].get("maximum_effect_count") != 1
        or criteria[1].get("replay_success_count") != 0
        or criteria[2].get("admitted_mutation_count") != 0
        or criteria[2].get("post_approval_worker_start_count") != 0
        or criteria[3].get("admitted_authority_count") != 0
        or criteria[4].get("grant_operation_count") != 22
        or criteria[4].get("strict_explicit_denial_count") != 21
        or criteria[4].get("strict_denied_by_absence_count") != 0
        or criteria[4].get("approval_forbidden_authority_field_count") != 0
        or criteria[5].get("registered_operation_count") != 22
        or criteria[5].get("effect_class_count") != 7
        or criteria[5].get("failure_class_count") != 14
        or criteria[5].get("operation_failure_pair_count") != 308
        or criteria[5].get("one_effect_class_per_registered_operation") is not True
        or criteria[5].get("prior_identity_family_count") != 7
        or criteria[5].get("old_call_replayed") is not False
        or criteria[5].get("old_authority_object_reused") is not False
        or criteria[5].get("uncertain_outcome_retried") is not False
    ):
        failures.append("Sprint 5 acceptance evidence is invalid")
    stories = value.get("story_gates", [])
    if [item.get("story_id") for item in stories if isinstance(item, dict)] != ["5.1", "5.2", "5.3"] or any(
        item.get("current_linux_scope_complete") is not True
        or item.get("story_checkbox_complete") is not False
        or item.get("blocking_controls") != ["G-DOD-10"]
        or not item.get("blockers") for item in stories
    ):
        failures.append("Sprint 5 story-gate aggregation is invalid")
    expected_dod = {
        "control_ids": list(G_DOD_IDS), "story_count": 3,
        "all_non_platform_controls_pass_or_not_applicable": True,
        "blocking_controls": ["G-DOD-10"],
        "platform_evidence_substitution": "prohibited",
    }
    if value.get("universal_definition_of_done") != expected_dod:
        failures.append("Sprint 5 Definition-of-Done aggregation is invalid")
    expected_summary = {
        "acceptance_criteria_passed": 6, "acceptance_criteria_failed": 0,
        "story_gate_count": 3, "current_linux_sprint_scope_complete": True,
        "sprint_checkbox_complete": False, "blocking_story_count": 3,
        "blocking_story_ids": ["5.1", "5.2", "5.3"],
        "open_dependency_ids": ["1.3", "2.3", "2.4", "5.1", "5.2"],
        "blocking_control_count": 1, "blocking_controls": ["G-DOD-10"],
    }
    if value.get("summary") != expected_summary:
        failures.append("Sprint 5 gate summary is invalid")
    review = value.get("independent_review", {})
    if (
        review.get("reviewer_id") != "agentmage-sprint-5-independent-gate-v2"
        or review.get("reviewed_commit") != REVIEWED_COMMIT
        or review.get("reviewed_tree") != REVIEWED_TREE
        or len(review.get("reviewed_artifacts", [])) != len(REVIEWED_PATHS)
        or review.get("finding_count") != 0 or review.get("findings") != []
        or review.get("external_human_review_status") != "not-performed"
    ):
        failures.append("Sprint 5 independent review is invalid")
    if value.get("platform") != {
        "status": "blocked-supported-platform-installed-product-matrix",
        "execution_performed": False, "evidence_substitution": "prohibited",
        "support_claim": "none",
    }:
        failures.append("Sprint 5 platform blocker is invalid")
    if (
        value.get("installed_product_claim") != "none"
        or value.get("product_requirement_completion_claim") != "none"
        or value.get("product_acceptance_claim") != "none"
        or value.get("release_claim") != "none"
    ):
        failures.append("Sprint 5 gate made an unsupported claim")
    if verify_current:
        try:
            expected = build_report(root)
        except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
            failures.append(f"cannot rebuild Sprint 5 gate report: {error}")
        else:
            if value != expected:
                failures.append("Sprint 5 gate report is stale, incomplete, or widened")
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
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
        print(f"Sprint 5 gate failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Sprint 5 gate failed: {failure}", file=sys.stderr)
        return 1
    print("Sprint 5 current Linux scope passed with dependency and platform blockers preserved")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
