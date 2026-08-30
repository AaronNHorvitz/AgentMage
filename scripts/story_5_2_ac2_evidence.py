#!/usr/bin/env python3
"""Build and validate Story 5.2 AC2 no-replay recovery evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-5/story-5.2"
RAW_PATH: Final = EVIDENCE_DIR / "story-ac2-no-replay-recovery-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "story-ac2-no-replay-recovery-report.json"
COMMANDS: Final = (
    (
        "cargo",
        "test",
        "-p",
        "agentmage-kernel-engine",
        "--test",
        "retry_admission",
        "story_5_2_prior_effect_recovery_never_replays_authority_or_unsafe_effect",
        "--",
        "--exact",
    ),
    (
        "cargo",
        "test",
        "-p",
        "agentmage-kernel-engine",
        "--test",
        "retry_admission",
        "racing_eligible_attempts_execute_once_and_uncertainty_is_sticky",
        "--",
        "--exact",
    ),
    ("python3", "scripts/retry_repair_policy_evidence.py"),
    ("python3", "scripts/story_5_2_policy_evidence.py"),
)
MARKERS: Final = (
    "test story_5_2_prior_effect_recovery_never_replays_authority_or_unsafe_effect ... ok",
    "test racing_eligible_attempts_execute_once_and_uncertainty_is_sticky ... ok",
    "Retry, repair, mutation, and race evidence validated through Sub-task 5.2.4.2",
    "Story 5.2 policy evidence validated through Sub-task 5.2.4.3",
)
RETAINED_PATHS: Final = (
    "kernel/contracts/src/engineering_records.rs",
    "kernel/engine/src/retry_admission.rs",
    "kernel/engine/tests/retry_admission.rs",
    "artifacts/sprints/sprint-5/story-5.2/retry-repair-policy-report.json",
    "artifacts/sprints/sprint-5/story-5.2/retry-repair-policy-results.log",
    "artifacts/sprints/sprint-5/story-5.2/policy-verification-report.json",
)
TRUTH: Final = {
    "current_retry_admission_scope_complete": True,
    "prior_identity_family_count": 7,
    "prior_identity_families": [
        "operation_attempt",
        "call",
        "tool_call",
        "grant",
        "approval",
        "receipt",
        "idempotency_key_sha256",
    ],
    "replayed_prior_identity_admitted": False,
    "old_call_replayed": False,
    "old_authority_object_reused": False,
    "automatic_retry_denied_effect_class_count": 4,
    "automatic_retry_denied_effect_classes": [
        "non_idempotent",
        "destructive",
        "external",
        "unknown",
    ],
    "uncertain_recovery_admitted": False,
    "non_admitting_reconciliation_disposition_count": 2,
    "non_admitting_reconciliation_dispositions": [
        "effect_uncertain",
        "desired_state_already_present",
    ],
    "concurrent_eligible_attempts": 16,
    "execution_admissions": 1,
    "synthetic_effect_callback_invocations": 1,
    "uncertain_outcome_retained": True,
    "uncertain_outcome_retried": False,
    "dispatches_after_policy_mutation": 0,
    "native_tool_effect_executed": False,
    "model_inference_executed": False,
    "synthetic_data_only": True,
    "native_platform_complete": False,
    "independent_review_complete": False,
    "story_completion_claim": False,
    "sprint_completion_claim": False,
    "release_claim": "none",
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def artifact(path: str) -> dict[str, Any]:
    value = ROOT / path
    return {"path": path, "byte_length": value.stat().st_size, "sha256": sha256(value)}


def load(path: str) -> dict[str, Any]:
    return json.loads((ROOT / path).read_text(encoding="utf-8"))


def expected_report() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "agentmage-story-acceptance-evidence",
        "story_id": "5.2",
        "criterion_id": "5.2.AC2",
        "generated_on": "2026-08-30",
        "status": "pass-local-current-retry-admission-scope",
        "commands": [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "retained_evidence": [artifact(path) for path in RETAINED_PATHS],
        "artifacts": [
            artifact("docs/verification/story-5-2-ac2-no-replay-recovery.md"),
            artifact("scripts/story_5_2_ac2_evidence.py"),
            artifact("tests/test_story_5_2_ac2_evidence.py"),
            artifact(RAW_PATH.relative_to(ROOT).as_posix()),
        ],
        "acceptance_truth": dict(TRUTH),
        "limitations": [
            "the criterion closes the current pure retry-admission and synchronized in-runtime gate scope",
            "the single effect callback is synthetic and content free; no native tool or provider executes",
            "cross-process crash durability, installed-product, and cross-platform evidence remain later gates",
            "independent review, Story, Sprint, packaging, and release completion are not claimed",
        ],
    }


def validate_upstream() -> list[str]:
    failures: list[str] = []
    retry = load("artifacts/sprints/sprint-5/story-5.2/retry-repair-policy-report.json")
    policy = load("artifacts/sprints/sprint-5/story-5.2/policy-verification-report.json")
    fresh = retry.get("fresh_attempt_contract", {})
    race = retry.get("race_contract", {})
    mutation = retry.get("mutation_contract", {})
    expected_ledger = [
        "call",
        "tool_call",
        "grant",
        "approval",
        "receipt",
        "idempotency_key_sha256",
        "operation_attempt",
    ]
    if fresh.get("complete_prior_use_ledger") != expected_ledger:
        failures.append("complete prior-use ledger evidence is missing or widened")
    if fresh.get("automatic_retry_denied_effect_classes") != [
        "non_idempotent",
        "destructive",
        "external",
        "uncertain",
        "unknown",
    ]:
        failures.append("unsafe or uncertain effect denial evidence is incomplete")
    if fresh.get("unsafe_or_stale_reconciliation_admitted") is not False:
        failures.append("unsafe reconciliation was admitted")
    expected_race = {
        "concurrent_eligible_attempts": 16,
        "execution_admissions": 1,
        "effect_callback_invocations": 1,
        "predecessor_step_attempt_claim_atomic": True,
        "execution_permit_cloneable": False,
        "uncertain_outcome_retained": True,
        "uncertain_outcome_converted_to_success": False,
        "uncertain_outcome_retried": False,
    }
    if race != expected_race:
        failures.append("race and sticky-uncertainty evidence is incomplete")
    if mutation.get("total_mutations") != 79 or mutation.get("dispatches_after_mutation") != 0:
        failures.append("pre-dispatch mutation evidence is incomplete")
    effect_rows = {row["class"]: row for row in policy.get("decision_table", {}).get("effect_classes", [])}
    for effect_class in ("non_idempotent", "destructive", "external", "unknown"):
        if effect_rows.get(effect_class, {}).get("automatic_retry") is not False:
            failures.append(f"consolidated policy does not deny automatic {effect_class} recovery")
    truth = retry.get("product_truth", {})
    if truth.get("tool_effect_executed") is not False or truth.get("model_inference_executed") is not False:
        failures.append("upstream evidence executed a native effect or model")
    return failures


def validate_raw(value: str) -> list[str]:
    failures = [f"raw results missing marker: {marker}" for marker in MARKERS if marker not in value]
    for prohibited in ("Traceback", "FAILED", "validation failed"):
        if prohibited in value:
            failures.append(f"raw results contain prohibited marker: {prohibited}")
    return failures


def validate_report(value: Any) -> list[str]:
    return [] if value == expected_report() else ["Story 5.2 AC2 report is stale, incomplete, reordered, or widened"]


def capture() -> tuple[str, int]:
    chunks = []
    for command in COMMANDS:
        result = subprocess.run(
            command,
            cwd=ROOT,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            check=False,
        )
        chunks.append(f"$ {' '.join(command)}\n{result.stdout.rstrip()}\n")
        if result.returncode != 0:
            return "".join(chunks), result.returncode
    return "".join(chunks), 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            raw, returncode = capture()
            if returncode != 0:
                sys.stderr.write(raw)
                return 1
            failures = validate_upstream() + validate_raw(raw)
            if failures:
                raise ValueError("; ".join(failures))
            EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
            RAW_PATH.write_text(raw, encoding="utf-8")
            REPORT_PATH.write_text(
                json.dumps(expected_report(), indent=2, sort_keys=True) + "\n",
                encoding="utf-8",
            )
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        failures = validate_upstream() + validate_raw(raw) + validate_report(report)
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as error:
        print(f"Story 5.2 AC2 evidence failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Story 5.2 AC2 evidence failed: {failure}", file=sys.stderr)
        return 1
    print("Story acceptance criterion 5.2.AC2 no-replay recovery validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
