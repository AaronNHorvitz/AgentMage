#!/usr/bin/env python3
"""Build and validate retained evidence for Sub-task 5.2.2.1."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts" / "sprints" / "sprint-5" / "story-5.2"
RAW_PATH: Final = EVIDENCE_DIR / "retry-repair-policy-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "retry-repair-policy-report.json"
COMMANDS: Final = (
    (
        "cargo",
        "test",
        "-p",
        "agentmage-kernel-engine",
        "tool_call_repair::tests",
        "--locked",
    ),
    (
        "cargo",
        "test",
        "-p",
        "agentmage-kernel-engine",
        "--test",
        "retry_admission",
        "--locked",
    ),
    ("npm", "run", "engineering-runtime:schemas:check"),
)
MARKERS: Final = (
    "normalization_is_ordered_bounded_and_does_not_invent_values ... ok",
    "model_repair_is_single_profile_bound_and_never_an_effect_attempt ... ok",
    "test result: ok. 2 passed; 0 failed",
    "fresh_attempt_requires_current_preflight_remaining_budget_and_single_use_grant ... ok",
    "conditional_retry_requires_current_safe_effect_reconciliation ... ok",
    "per_attempt_approval_must_be_current_exact_and_different_from_prior ... ok",
    "unsafe_and_uncertain_effects_never_receive_automatic_new_attempts ... ok",
    "complete_prior_use_ledger_denies_every_replayed_identity_and_any_receipt ... ok",
    "required_idempotency_key_must_be_fresh_and_digest_bound ... ok",
    "every_effect_failure_and_budget_mutation_denies_before_dispatch ... ok",
    "every_approval_field_mutation_denies_before_dispatch ... ok",
    "every_preflight_and_reconciliation_field_mutation_denies_before_dispatch ... ok",
    "every_attempt_identity_field_mutation_denies_before_dispatch ... ok",
    "test result: ok. 10 passed; 0 failed",
    "call envelope carries only the digest of validated arguments and explains every rejection",
    "pass 52",
    "fail 0",
)
TRUTH: Final = {
    "synthetic_data_only": True,
    "tool_effect_executed": False,
    "authority_minted": False,
    "model_inference_executed": False,
    "network_calls": 0,
    "native_platform_claim": "none",
    "product_support_claim": "none",
    "release_claim": "none",
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def artifact(path: str) -> dict[str, Any]:
    absolute = ROOT / path
    return {"path": path, "byte_length": absolute.stat().st_size, "sha256": sha256(absolute)}


def command_text(command: tuple[str, ...]) -> str:
    return " ".join(command)


def expected_report() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "agentmage-retry-repair-policy-evidence",
        "story_id": "5.2",
        "task_id": "5.2.4.1",
        "coverage_task_ids": ["5.2.2.1", "5.2.2.2", "5.2.2.3", "5.2.4.1"],
        "generated_on": "2026-08-30",
        "status": "pass-local-contract-evidence",
        "normalization_contract": {
            "complete_contiguous_fragments_required": True,
            "maximum_fragments": 1024,
            "maximum_assembled_bytes": 1048576,
            "rules_in_order": [
                "reassemble_ordered_fragments",
                "strip_leading_utf8_bom_when_present",
                "trim_outer_json_whitespace_when_present",
            ],
            "value_invention_permitted": False,
            "exact_schema_validation_precedes_model_repair": True,
        },
        "model_repair_contract": {
            "maximum_repairs": 1,
            "exact_model_profile_bound": True,
            "exact_tool_schema_bound": True,
            "prior_schema_rejection_required": True,
            "prior_effect_attempts_permitted": 0,
            "repair_admission_contains_grant_or_executor_authority": False,
            "call_envelope_repair_count_values": [0, 1],
        },
        "denied_conditions": [
            "empty_or_incomplete_fragments",
            "fragment_or_byte_limit_exceeded",
            "empty_normalized_payload",
            "repair_disabled",
            "exact_schema_not_rejected",
            "model_profile_mismatch",
            "tool_schema_mismatch",
            "repair_already_used",
            "effect_attempt_already_exists",
        ],
        "fresh_attempt_contract": {
            "current_complete_preflight_required": True,
            "conditional_effect_reconciliation_required": True,
            "unsafe_or_stale_reconciliation_admitted": False,
            "remaining_attempt_budget_required": True,
            "fresh_call_and_attempt_identities_required": True,
            "successor_grant_revision": 1,
            "successor_grant_use_limit": 1,
            "successor_grant_use_count": 0,
            "successor_grant_status": "issued",
            "fresh_per_attempt_approval_required_when_configured": True,
            "grant_consumed_or_effect_dispatched_by_compiler": False,
            "automatic_retry_denied_effect_classes": [
                "non_idempotent",
                "destructive",
                "external",
                "uncertain",
                "unknown",
            ],
            "complete_prior_use_ledger": [
                "call",
                "tool_call",
                "grant",
                "approval",
                "receipt",
                "idempotency_key_sha256",
                "operation_attempt",
            ],
            "any_successor_receipt_before_execution_admitted": False,
            "required_idempotency_key_must_be_fresh": True,
        },
        "mutation_contract": {
            "effect_and_retry_field_mutations": 9,
            "failure_and_uncertainty_field_mutations": 14,
            "budget_field_mutations": 9,
            "approval_field_mutations": 21,
            "preflight_field_mutations": 5,
            "reconciliation_field_mutations": 6,
            "attempt_identity_field_mutations": 15,
            "total_mutations": 79,
            "canonical_policy_digest_verified": True,
            "canonical_decision_digest_verified": True,
            "canonical_admission_digest_verified": True,
            "canonical_approval_digest_verified": True,
            "dispatches_after_mutation": 0,
        },
        "commands": [command_text(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "artifacts": [
            artifact("kernel/engine/src/tool_call_repair.rs"),
            artifact("kernel/engine/src/retry_admission.rs"),
            artifact("kernel/engine/tests/retry_admission.rs"),
            artifact("scripts/engineering_runtime_schemas.mjs"),
            artifact("schemas/engineering-runtime/call-envelope.schema.json"),
            artifact("tests/test_engineering_runtime_schemas.mjs"),
            artifact("scripts/retry_repair_policy_evidence.py"),
            artifact("tests/test_retry_repair_policy_evidence.py"),
            artifact(RAW_PATH.relative_to(ROOT).as_posix()),
        ],
        "product_truth": dict(TRUTH),
    }


def render(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n"


def validate_raw(text: str) -> list[str]:
    failures = [f"raw results missing marker: {marker}" for marker in MARKERS if marker not in text]
    for prohibited in ("FAILED (", "test result: FAILED", "not ok ", "Traceback (most recent call last)"):
        if prohibited in text:
            failures.append(f"raw results contain failure marker: {prohibited}")
    return failures


def validate_report(value: Any) -> list[str]:
    if value != expected_report():
        return ["retry-repair policy report is stale, incomplete, reordered, or widened"]
    if value.get("product_truth") != TRUTH:
        return ["retry-repair policy product truth was widened"]
    return []


def capture() -> tuple[str, int]:
    chunks: list[str] = []
    for command in COMMANDS:
        chunks.append(f"$ {command_text(command)}\n")
        result = subprocess.run(
            command,
            cwd=ROOT,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            check=False,
        )
        chunks.append(result.stdout)
        if not result.stdout.endswith("\n"):
            chunks.append("\n")
        if result.returncode != 0:
            return "".join(chunks), result.returncode
    return "".join(chunks), 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    if args.write:
        raw, returncode = capture()
        if returncode != 0:
            sys.stderr.write(raw)
            print("retry-repair policy evidence build failed", file=sys.stderr)
            return 1
        failures = validate_raw(raw)
        if failures:
            for failure in failures:
                print(f"retry-repair policy evidence build failed: {failure}", file=sys.stderr)
            return 1
        EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
        RAW_PATH.write_text(raw, encoding="utf-8")
        REPORT_PATH.write_text(render(expected_report()), encoding="utf-8")

    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(f"retry-repair policy evidence validation failed: {error}", file=sys.stderr)
        return 1
    failures = validate_raw(raw) + validate_report(report)
    if failures:
        for failure in failures:
            print(f"retry-repair policy evidence validation failed: {failure}", file=sys.stderr)
        return 1
    print("Retry, repair, and mutation evidence validated through Sub-task 5.2.4.1")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
