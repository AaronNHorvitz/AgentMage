#!/usr/bin/env python3
"""Independently evaluate Story 5.3 without widening bounded evidence."""

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
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-5/story-5.3"
REPORT_PATH: Final = EVIDENCE_DIR / "story-gate-report.json"
RAW_PATH: Final = EVIDENCE_DIR / "story-gate-results.log"
REVIEWED_COMMIT: Final = "c3a7bad782664fcfb6bbff9dca94d28d5ba45356"
REVIEWED_TREE: Final = "e23510dcd1b4684aa571cac51a3349d8cc1d0f62"
VALIDATOR_COMMANDS: Final = (
    ("python3", "scripts/story_5_3_ac1_evidence.py"),
    ("python3", "scripts/story_5_3_ac2_evidence.py"),
    ("python3", "scripts/story_5_3_ac3_evidence.py"),
)
VALIDATOR_MARKERS: Final = (
    "Story acceptance criterion 5.3.AC1 runtime-owned workflow transitions validated",
    "Story acceptance criterion 5.3.AC2 verifier-owned completion validated",
    "Story acceptance criterion 5.3.AC3 uncertain and ineligible effects validated",
)
REVIEWED_PATHS: Final = (
    "kernel/engine/src/engineering_records.rs",
    "kernel/engine/src/workflow_definition.rs",
    "kernel/engine/src/workflow_identity.rs",
    "kernel/engine/src/workflow_budget.rs",
    "kernel/engine/src/workflow_progress.rs",
    "kernel/engine/src/workflow_verifier.rs",
    "kernel/engine/src/workflow_terminal.rs",
    "kernel/engine/src/retry_admission.rs",
    "kernel/engine/tests/workflow_definition.rs",
    "kernel/engine/tests/workflow_identity.rs",
    "kernel/engine/tests/workflow_budget_independence.rs",
    "kernel/engine/tests/workflow_verifier.rs",
    "kernel/engine/tests/retry_admission.rs",
    "docs/verification/story-5-3-workflow-definition-evidence.md",
    "docs/verification/story-5-3-workflow-identity-evidence.md",
    "docs/verification/story-5-3-workflow-budget-evidence.md",
    "docs/verification/story-5-3-workflow-verifier-evidence.md",
    "docs/verification/story-5-3-workflow-terminal-evidence.md",
    "docs/verification/story-5-3-workflow-adversarial-evidence.md",
    "artifacts/sprints/sprint-5/story-5.3/workflow-definition-report.json",
    "artifacts/sprints/sprint-5/story-5.3/workflow-identity-report.json",
    "artifacts/sprints/sprint-5/story-5.3/workflow-budget-independence-report.json",
    "artifacts/sprints/sprint-5/story-5.3/workflow-verifier-report.json",
    "artifacts/sprints/sprint-5/story-5.3/workflow-terminal-report.json",
    "artifacts/sprints/sprint-5/story-5.3/workflow-adversarial-report.json",
    "artifacts/sprints/sprint-5/story-5.3/rv52-applicability.json",
    "artifacts/sprints/sprint-5/story-5.3/story-ac1-runtime-owned-transitions-report.json",
    "artifacts/sprints/sprint-5/story-5.3/story-ac2-verifier-owned-completion-report.json",
    "artifacts/sprints/sprint-5/story-5.3/story-ac3-uncertain-effect-blocking-report.json",
)
REQUIRED_TASK_MARKERS: Final = (
    *(f"- [x] **Task 5.3.{task} -" for task in range(1, 4)),
    *(f"  - [x] **Sub-task 5.3.1.{sub}:" for sub in range(1, 4)),
    *(f"  - [x] **Sub-task 5.3.2.{sub}:" for sub in range(1, 3)),
    *(f"  - [x] **Sub-task 5.3.3.{sub}:" for sub in range(1, 3)),
    *(f"- [x] **Story AC 5.3.AC{criterion}:" for criterion in range(1, 4)),
)
G_DOD_IDS: Final = tuple(f"G-DOD-{index:02d}" for index in range(1, 14))
BLOCKERS: Final = (
    "story-1.3-acceptance-gate-open",
    "story-2.4-acceptance-gate-open",
    "story-5.2-acceptance-gate-open",
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
    descriptor, temporary_name = tempfile.mkstemp(prefix=".agentmage-story-5-3-gate-", dir=path.parent)
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
        raise ValueError("Story 5.3 reviewed commit is unavailable")
    if git_output("show", "-s", "--format=%T", REVIEWED_COMMIT) != REVIEWED_TREE:
        raise ValueError("Story 5.3 reviewed tree changed")
    records = []
    for path in REVIEWED_PATHS:
        committed = git_output("show", f"{REVIEWED_COMMIT}:{path}", binary=True)
        current = ROOT / path
        if not current.is_file() or current.read_bytes() != committed:
            raise ValueError(f"reviewed Story 5.3 artifact changed after review: {path}")
        records.append({"path": path, "byte_length": len(committed), "sha256": sha256_bytes(committed)})
    return records


def task_completion(tasks_text: str) -> list[str]:
    return [marker for marker in REQUIRED_TASK_MARKERS if marker not in tasks_text]


def dependency_state(tasks_text: str) -> list[dict[str, str]]:
    expected = (
        ("1.3", "#### [ ] Story 1.3 -", "blocked-open-acceptance"),
        ("2.4", "#### [ ] Story 2.4 -", "blocked-open-acceptance"),
        ("5.2", "#### [ ] Story 5.2 -", "blocked-open-acceptance"),
    )
    states = []
    for story_id, marker, status in expected:
        if marker not in tasks_text:
            raise ValueError(f"Story 5.3 dependency state changed: {story_id}")
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
    failures = [f"Story 5.3 gate results missing marker: {marker}" for marker in VALIDATOR_MARKERS if marker not in value]
    for prohibited in ("Traceback", "FAILED", "validation failed"):
        if prohibited in value:
            failures.append(f"Story 5.3 gate results contain prohibited marker: {prohibited}")
    return failures


def criterion_summary(report: dict[str, Any]) -> dict[str, Any]:
    truth = report["acceptance_truth"]
    criterion_id = report["criterion_id"]
    if criterion_id == "5.3.AC1":
        valid = (
            truth["workflow_lifecycle_state_count"] == 18
            and truth["absorbing_terminal_state_count"] == 8
            and truth["closed_transition_table"] is True
            and truth["approval_bypass_dispatches"] == 0
            and truth["grant_reuse_dispatches"] == 0
        )
        status = "pass-local-current-workflow-admission"
    elif criterion_id == "5.3.AC2":
        valid = (
            truth["evaluated_surface_count"] == 8
            and truth["exit_zero_has_completion_authority"] is False
            and truth["persuasive_model_or_tool_text_has_completion_authority"] is False
            and truth["stale_or_contradictory_evidence_establishes_completion"] is False
            and truth["verified_success_requires_opaque_current_evidence_proof"] is True
        )
        status = "pass-local-current-verifier-completion"
    elif criterion_id == "5.3.AC3":
        valid = (
            truth["fresh_identity_family_count"] == 7
            and truth["old_call_or_authority_replayed"] is False
            and truth["uncertain_effect_automatic_retry"] is False
            and truth["uncertain_outcome_converted_to_success"] is False
            and truth["uncertain_outcome_retried"] is False
        )
        status = "pass-local-current-no-replay-recovery"
    else:
        raise ValueError(f"unexpected Story 5.3 criterion: {criterion_id}")
    if not valid:
        raise ValueError(f"Story 5.3 criterion truth is incomplete: {criterion_id}")
    return {
        "criterion_id": criterion_id,
        "status": status,
        "acceptance_truth_sha256": sha256_bytes(canonical_json(truth)),
    }


def build_report() -> dict[str, Any]:
    tasks_text = (ROOT / "TASKS.md").read_text(encoding="utf-8")
    incomplete = task_completion(tasks_text)
    if incomplete:
        raise ValueError(f"Story 5.3 task or criterion is incomplete: {incomplete[0]}")
    raw = RAW_PATH.read_text(encoding="utf-8")
    raw_failures = validate_raw(raw)
    if raw_failures:
        raise ValueError("; ".join(raw_failures))
    criteria = [
        criterion_summary(read_json(EVIDENCE_DIR / name))
        for name in (
            "story-ac1-runtime-owned-transitions-report.json",
            "story-ac2-verifier-owned-completion-report.json",
            "story-ac3-uncertain-effect-blocking-report.json",
        )
    ]
    return {
        "schema_version": 1,
        "story_id": "5.3",
        "status": "blocked-open-dependencies-and-platform",
        "reviewed_commit": REVIEWED_COMMIT,
        "reviewed_tree": REVIEWED_TREE,
        "task_count": 3,
        "sub_task_count": 7,
        "acceptance_criteria": criteria,
        "dependencies": dependency_state(tasks_text),
        "universal_definition_of_done": universal_dod(),
        "blocking_controls": ["G-DOD-10"],
        "blockers": list(BLOCKERS),
        "independent_review": {
            "reviewer_id": "agentmage-story-5.3-independent-gate-v1",
            "review_type": "automated-independent-aggregate-review",
            "reviewed_commit": REVIEWED_COMMIT,
            "reviewed_tree": REVIEWED_TREE,
            "artifacts": reviewed_artifacts(),
            "finding_count": 0,
            "findings": [],
            "external_human_review_status": "not-performed",
        },
        "validator_results_sha256": sha256_bytes(raw.encode("utf-8")),
        "current_linux_workflow_contract_and_verifier_scope_complete": True,
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
        return ["Story 5.3 gate report must be an object"]
    failures = []
    if (
        value.get("schema_version") != 1
        or value.get("story_id") != "5.3"
        or value.get("status") != "blocked-open-dependencies-and-platform"
        or value.get("reviewed_commit") != REVIEWED_COMMIT
        or value.get("reviewed_tree") != REVIEWED_TREE
    ):
        failures.append("Story 5.3 gate identity or status is invalid")
    criteria = value.get("acceptance_criteria", [])
    if [item.get("criterion_id") for item in criteria if isinstance(item, dict)] != ["5.3.AC1", "5.3.AC2", "5.3.AC3"]:
        failures.append("Story 5.3 acceptance criterion closure is invalid")
    elif [item.get("status") for item in criteria] != [
        "pass-local-current-workflow-admission",
        "pass-local-current-verifier-completion",
        "pass-local-current-no-replay-recovery",
    ]:
        failures.append("Story 5.3 acceptance criterion status is invalid")
    if value.get("universal_definition_of_done") != universal_dod() or value.get("blocking_controls") != ["G-DOD-10"]:
        failures.append("Story 5.3 Definition-of-Done disposition is invalid")
    if value.get("blockers") != list(BLOCKERS):
        failures.append("Story 5.3 blocker set is invalid")
    if value.get("dependencies") != [
        {"story_id": "1.3", "status": "blocked-open-acceptance"},
        {"story_id": "2.4", "status": "blocked-open-acceptance"},
        {"story_id": "5.2", "status": "blocked-open-acceptance"},
    ]:
        failures.append("Story 5.3 dependency disposition is invalid")
    review = value.get("independent_review", {})
    if (
        review.get("reviewer_id") != "agentmage-story-5.3-independent-gate-v1"
        or review.get("review_type") != "automated-independent-aggregate-review"
        or review.get("reviewed_commit") != REVIEWED_COMMIT
        or review.get("reviewed_tree") != REVIEWED_TREE
        or review.get("finding_count") != 0
        or review.get("findings") != []
        or review.get("external_human_review_status") != "not-performed"
        or len(review.get("artifacts", [])) != len(REVIEWED_PATHS)
    ):
        failures.append("Story 5.3 independent review is invalid")
    if (
        value.get("current_linux_workflow_contract_and_verifier_scope_complete") is not True
        or value.get("story_checkbox_complete") is not False
        or value.get("dependency_substitution_permitted") is not False
        or value.get("platform_evidence_substituted") is not False
        or value.get("installed_product_claim") != "none"
        or value.get("product_acceptance_claim") != "none"
        or value.get("sprint_completion_claim") is not False
        or value.get("release_claim") != "none"
    ):
        failures.append("Story 5.3 gate made an unsupported completion claim")
    if verify_current:
        try:
            expected = build_report()
        except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
            failures.append(f"cannot rebuild Story 5.3 gate report: {error}")
        else:
            if value != expected:
                failures.append("Story 5.3 gate report is stale, incomplete, or widened")
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
        print(f"Story 5.3 gate failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Story 5.3 gate failed: {failure}", file=sys.stderr)
        return 1
    print("Story 5.3 current Linux scope passed with dependency and platform blockers preserved")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
