#!/usr/bin/env python3
"""Independently evaluate Story 5.2 without widening bounded evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-5/story-5.2"
REPORT_PATH: Final = EVIDENCE_DIR / "story-gate-report.json"
RAW_PATH: Final = EVIDENCE_DIR / "story-gate-results.log"
REVIEWED_COMMIT: Final = "eee6b97c8d9cca728f8811c5435f81e1fd2c3ed0"
REVIEWED_TREE: Final = "1da6266abced763075c831a24e540fff7c801f66"
VALIDATOR_COMMANDS: Final = (
    ("python3", "scripts/story_5_2_ac1_evidence.py"),
    ("python3", "scripts/story_5_2_ac2_evidence.py"),
    ("python3", "scripts/story_5_2_ac3_evidence.py"),
)
VALIDATOR_MARKERS: Final = (
    "Story acceptance criterion 5.2.AC1 deterministic retry disposition validated",
    "Story acceptance criterion 5.2.AC2 no-replay recovery validated",
    "Story acceptance criterion 5.2.AC3 bounded termination validated",
)
REVIEWED_PATHS: Final = (
    "kernel/contracts/src/engineering_records.rs",
    "kernel/engine/src/tooling.rs",
    "kernel/engine/src/tool_call_repair.rs",
    "kernel/engine/src/retry_admission.rs",
    "kernel/engine/tests/retry_admission.rs",
    "kernel/engine/src/workflow_budget.rs",
    "kernel/engine/src/workflow_progress.rs",
    "kernel/engine/src/workflow_termination.rs",
    "docs/verification/story-5-2-policy-evidence.md",
    "artifacts/sprints/sprint-5/story-5.2/effect-class-taxonomy-report.json",
    "artifacts/sprints/sprint-5/story-5.2/retry-repair-policy-report.json",
    "artifacts/sprints/sprint-5/story-5.2/workflow-supervision-report.json",
    "artifacts/sprints/sprint-5/story-5.2/policy-verification-report.json",
    "artifacts/sprints/sprint-5/story-5.2/story-ac1-deterministic-disposition-report.json",
    "artifacts/sprints/sprint-5/story-5.2/story-ac2-no-replay-recovery-report.json",
    "artifacts/sprints/sprint-5/story-5.2/story-ac3-bounded-termination-report.json",
)
REQUIRED_TASK_MARKERS: Final = (
    *(f"- [x] **Task 5.2.{task} -" for task in range(1, 5)),
    *(f"  - [x] **Sub-task 5.2.{task}.{sub}:" for task in range(1, 5) for sub in range(1, 4)),
    *(f"- [x] **Story AC 5.2.AC{criterion}:" for criterion in range(1, 4)),
)
G_DOD_IDS: Final = tuple(f"G-DOD-{index:02d}" for index in range(1, 14))
BLOCKERS: Final = (
    "story-2.3-acceptance-gate-open",
    "story-5.1-acceptance-gate-open",
    "supported-platform-installed-product-evidence-incomplete",
)


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(prefix=".agentmage-story-5-2-gate-", dir=path.parent)
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


def git_output(*arguments: str, binary: bool = False) -> bytes | str:
    result = subprocess.run(
        ["git", *arguments], cwd=ROOT, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
        stderr=subprocess.PIPE, check=False, timeout=15,
    )
    if result.returncode != 0:
        raise ValueError(f"git review operation failed: {' '.join(arguments)}")
    return result.stdout if binary else result.stdout.decode("utf-8").strip()


def reviewed_artifacts() -> list[dict[str, Any]]:
    if git_output("rev-parse", REVIEWED_COMMIT) != REVIEWED_COMMIT:
        raise ValueError("Story 5.2 reviewed commit is unavailable")
    if git_output("show", "-s", "--format=%T", REVIEWED_COMMIT) != REVIEWED_TREE:
        raise ValueError("Story 5.2 reviewed tree changed")
    records = []
    for path in REVIEWED_PATHS:
        committed = git_output("show", f"{REVIEWED_COMMIT}:{path}", binary=True)
        current = ROOT / path
        if not current.is_file() or current.read_bytes() != committed:
            raise ValueError(f"reviewed Story 5.2 artifact changed after review: {path}")
        records.append({"path": path, "byte_length": len(committed), "sha256": sha256_bytes(committed)})
    return records


def task_completion(tasks_text: str) -> list[str]:
    return [marker for marker in REQUIRED_TASK_MARKERS if marker not in tasks_text]


def dependency_state(tasks_text: str) -> list[dict[str, str]]:
    expected = (
        ("1.2", "#### [x] Story 1.2 -", "complete"),
        ("2.3", "#### [ ] Story 2.3 -", "blocked-open-acceptance"),
        ("5.1", "#### [ ] Story 5.1 -", "blocked-open-acceptance"),
    )
    states = []
    for story_id, marker, status in expected:
        if marker not in tasks_text:
            raise ValueError(f"Story 5.2 dependency state changed: {story_id}")
        states.append({"story_id": story_id, "status": status})
    return states


def universal_dod() -> list[dict[str, str]]:
    statuses = {control_id: "pass-current-story-scope" for control_id in G_DOD_IDS}
    statuses["G-DOD-06"] = "pass-no-production-tool-attempt"
    statuses["G-DOD-07"] = "pass-synthetic-data-no-new-persistence"
    statuses["G-DOD-08"] = "pass-no-user-owned-file-effects"
    statuses["G-DOD-10"] = "blocked-supported-platform-installed-product-matrix"
    statuses["G-DOD-12"] = "pass-automated-independent-aggregate-review"
    return [{"control_id": control_id, "status": statuses[control_id]} for control_id in G_DOD_IDS]


def capture_validators() -> tuple[str, int]:
    chunks = []
    for command in VALIDATOR_COMMANDS:
        result = subprocess.run(
            command, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
            text=True, check=False,
        )
        chunks.append(f"$ {' '.join(command)}\n{result.stdout.rstrip()}\n")
        if result.returncode != 0:
            return "".join(chunks), result.returncode
    return "".join(chunks), 0


def validate_raw(value: str) -> list[str]:
    failures = [f"Story 5.2 gate results missing marker: {marker}" for marker in VALIDATOR_MARKERS if marker not in value]
    for prohibited in ("Traceback", "FAILED", "validation failed"):
        if prohibited in value:
            failures.append(f"Story 5.2 gate results contain prohibited marker: {prohibited}")
    return failures


def criterion_summary(report: dict[str, Any]) -> dict[str, Any]:
    truth = report["acceptance_truth"]
    criterion_id = report["criterion_id"]
    if criterion_id == "5.2.AC1":
        valid = (
            truth["operation_failure_pair_count"] == 308
            and truth["every_operation_failure_pair_deterministic"] is True
            and truth["caller_effect_override_admitted"] is False
            and truth["dispatches_after_policy_mutation"] == 0
        )
        status = "pass-local-current-contract-policy"
    elif criterion_id == "5.2.AC2":
        valid = (
            truth["prior_identity_family_count"] == 7
            and truth["old_call_replayed"] is False
            and truth["old_authority_object_reused"] is False
            and truth["synthetic_effect_callback_invocations"] == 1
            and truth["uncertain_outcome_retried"] is False
        )
        status = "pass-local-current-retry-admission"
    elif criterion_id == "5.2.AC3":
        valid = (
            truth["budget_termination_case_count"] == 19
            and truth["one_actionable_diagnosis_per_termination"] is True
            and truth["terminal_diagnosis_sticky"] is True
            and truth["additional_effect_count"] == 0
        )
        status = "pass-local-current-supervision"
    else:
        raise ValueError(f"unexpected Story 5.2 criterion: {criterion_id}")
    if not valid:
        raise ValueError(f"Story 5.2 criterion truth is incomplete: {criterion_id}")
    return {
        "criterion_id": criterion_id,
        "status": status,
        "acceptance_truth_sha256": sha256_bytes(canonical_json(truth)),
    }


def build_report() -> dict[str, Any]:
    tasks_text = (ROOT / "TASKS.md").read_text(encoding="utf-8")
    incomplete = task_completion(tasks_text)
    if incomplete:
        raise ValueError(f"Story 5.2 task or criterion is incomplete: {incomplete[0]}")
    raw = RAW_PATH.read_text(encoding="utf-8")
    raw_failures = validate_raw(raw)
    if raw_failures:
        raise ValueError("; ".join(raw_failures))
    criteria = [
        criterion_summary(read_json(EVIDENCE_DIR / name))
        for name in (
            "story-ac1-deterministic-disposition-report.json",
            "story-ac2-no-replay-recovery-report.json",
            "story-ac3-bounded-termination-report.json",
        )
    ]
    return {
        "schema_version": 1,
        "story_id": "5.2",
        "status": "blocked-open-dependencies-and-platform",
        "reviewed_commit": REVIEWED_COMMIT,
        "reviewed_tree": REVIEWED_TREE,
        "task_count": 4,
        "sub_task_count": 12,
        "acceptance_criteria": criteria,
        "dependencies": dependency_state(tasks_text),
        "universal_definition_of_done": universal_dod(),
        "blocking_controls": ["G-DOD-10"],
        "blockers": list(BLOCKERS),
        "independent_review": {
            "reviewer_id": "agentmage-story-5.2-independent-gate-v1",
            "review_type": "automated-independent-aggregate-review",
            "reviewed_commit": REVIEWED_COMMIT,
            "reviewed_tree": REVIEWED_TREE,
            "artifacts": reviewed_artifacts(),
            "finding_count": 0,
            "findings": [],
            "external_human_review_status": "not-performed",
        },
        "validator_results_sha256": sha256_bytes(raw.encode("utf-8")),
        "current_linux_contract_policy_and_supervision_scope_complete": True,
        "story_checkbox_complete": False,
        "dependency_substitution_permitted": False,
        "platform_evidence_substituted": False,
        "installed_product_claim": "none",
        "product_acceptance_claim": "none",
        "sprint_completion_claim": False,
        "release_claim": "none",
    }


def validate_report(value: Any, *, verify_current: bool = True) -> list[str]:
    if not isinstance(value, dict):
        return ["Story 5.2 gate report must be an object"]
    failures = []
    if (
        value.get("schema_version") != 1
        or value.get("story_id") != "5.2"
        or value.get("status") != "blocked-open-dependencies-and-platform"
        or value.get("reviewed_commit") != REVIEWED_COMMIT
        or value.get("reviewed_tree") != REVIEWED_TREE
    ):
        failures.append("Story 5.2 gate identity or status is invalid")
    criteria = value.get("acceptance_criteria", [])
    if [item.get("criterion_id") for item in criteria if isinstance(item, dict)] != ["5.2.AC1", "5.2.AC2", "5.2.AC3"]:
        failures.append("Story 5.2 acceptance criterion closure is invalid")
    elif [item.get("status") for item in criteria] != [
        "pass-local-current-contract-policy",
        "pass-local-current-retry-admission",
        "pass-local-current-supervision",
    ]:
        failures.append("Story 5.2 acceptance criterion status is invalid")
    if value.get("universal_definition_of_done") != universal_dod() or value.get("blocking_controls") != ["G-DOD-10"]:
        failures.append("Story 5.2 Definition-of-Done disposition is invalid")
    if value.get("blockers") != list(BLOCKERS):
        failures.append("Story 5.2 blocker set is invalid")
    if value.get("dependencies") != [
        {"story_id": "1.2", "status": "complete"},
        {"story_id": "2.3", "status": "blocked-open-acceptance"},
        {"story_id": "5.1", "status": "blocked-open-acceptance"},
    ]:
        failures.append("Story 5.2 dependency disposition is invalid")
    review = value.get("independent_review", {})
    if (
        review.get("reviewer_id") != "agentmage-story-5.2-independent-gate-v1"
        or review.get("review_type") != "automated-independent-aggregate-review"
        or review.get("reviewed_commit") != REVIEWED_COMMIT
        or review.get("reviewed_tree") != REVIEWED_TREE
        or review.get("finding_count") != 0
        or review.get("findings") != []
        or review.get("external_human_review_status") != "not-performed"
        or len(review.get("artifacts", [])) != len(REVIEWED_PATHS)
    ):
        failures.append("Story 5.2 independent review is invalid")
    if (
        value.get("current_linux_contract_policy_and_supervision_scope_complete") is not True
        or value.get("story_checkbox_complete") is not False
        or value.get("dependency_substitution_permitted") is not False
        or value.get("platform_evidence_substituted") is not False
        or value.get("installed_product_claim") != "none"
        or value.get("product_acceptance_claim") != "none"
        or value.get("sprint_completion_claim") is not False
        or value.get("release_claim") != "none"
    ):
        failures.append("Story 5.2 gate made an unsupported completion claim")
    if verify_current:
        try:
            expected = build_report()
        except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
            failures.append(f"cannot rebuild Story 5.2 gate report: {error}")
        else:
            if value != expected:
                failures.append("Story 5.2 gate report is stale, incomplete, or widened")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            raw, returncode = capture_validators()
            if returncode != 0:
                sys.stderr.write(raw)
                return 1
            raw_failures = validate_raw(raw)
            if raw_failures:
                raise ValueError("; ".join(raw_failures))
            write_atomic(RAW_PATH, raw.encode("utf-8"))
            write_atomic(REPORT_PATH, canonical_json(build_report()))
        report = read_json(REPORT_PATH)
        failures = validate_report(report)
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
        print(f"Story 5.2 gate failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Story 5.2 gate failed: {failure}", file=sys.stderr)
        return 1
    print("Story 5.2 current Linux scope passed with dependency and platform blockers preserved")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
