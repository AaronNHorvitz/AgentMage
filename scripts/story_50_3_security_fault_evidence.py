#!/usr/bin/env python3
"""Generate and validate the local Story 50.3 security/fault campaign."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-50/story-50.3-security-fault"
RAW_PATH: Final = EVIDENCE_DIR / "local-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "report.json"
COMMANDS: Final = (
    ("cargo", "test", "-p", "agentmage-kernel-engine", "two_hundred_injections_never_create_guidance_or_authority_without_user_decision", "--all-features", "--locked"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "story_22_3_declared_encodings_are_exact_and_binary_or_lossy_input_fails", "--all-features", "--locked"),
    ("cargo", "test", "-p", "agentmage-capability-read-only", "stale_restricted_unsupported_redacted_missing_and_range_fail_closed", "--locked"),
    ("cargo", "test", "-p", "agentmage-capability-read-only", "cancellation_timeout_and_crash_are_receipted_for_every_tool", "--locked"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "story_16_3_post_approval_preflight_policy_and_grant_drift_start_no_worker", "--all-features", "--locked"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "story_16_3_crash_boundaries_restore_one_uncertain_terminal_without_replay", "--all-features", "--locked"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "story_22_4_one_hundred_seed_no_unwind_boundary_campaign_has_zero_replay", "--all-features", "--locked", "--", "--nocapture"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "story_23_4_request_and_outcome_tampering_fail_closed", "--all-features", "--locked"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "story_21_3_missing_duplicate_reordered_cross_bound_and_tampered_events_fail_closed", "--all-features", "--locked"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "verified_workflow_supervisor::tests", "--all-features", "--locked"),
    ("cargo", "test", "-p", "agentmage-host", "read_output_secret_classes_are_withheld_before_model_projection", "--all-features", "--locked"),
)
SOURCES: Final = (
    "kernel/engine/src/instruction_provenance.rs",
    "kernel/engine/src/source_preparation.rs",
    "capabilities/read-only/src/artifact.rs",
    "kernel/engine/src/tool_composition.rs",
    "kernel/engine/src/durable_attempt_recovery.rs",
    "kernel/engine/src/runtime_coordinator.rs",
    "kernel/engine/src/runtime_event.rs",
    "kernel/engine/src/verified_workflow_supervisor.rs",
    "shells/host/src/linux_coding_runtime.rs",
    "scripts/story_50_3_security_fault_evidence.py",
    "tests/test_story_50_3_security_fault_evidence.py",
)
BOUNDARIES: Final = (
    "source", "context", "proposal", "preflight", "approval", "grant", "worker",
    "receipt", "artifact", "verification", "retry", "recovery", "checkpoint", "terminal",
)


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def artifact(relative: str) -> dict[str, Any]:
    path = ROOT / relative
    return {"path": relative, "byte_length": path.stat().st_size, "sha256": digest(path)}


def expected_report() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "agentmage-story-50-3-security-fault-evidence",
        "story_id": "50.3",
        "generated_on": "2026-09-01",
        "status": "PASS_LOCAL_TEXT_LOG_SECURITY_FAULT",
        "commands": [" ".join(command) for command in COMMANDS],
        "crash_boundary_matrix": list(BOUNDARIES),
        "crash_seed_count": 100,
        "artifacts": [artifact(path) for path in SOURCES]
        + [artifact(RAW_PATH.relative_to(ROOT).as_posix())],
        "product_truth": {
            "prompt_injection_records_inert": True,
            "active_or_archive_bytes_executed": False,
            "secret_canary_disclosures": 0,
            "hostile_metadata_or_stale_reference_bypass": False,
            "model_output_authority": False,
            "approval_or_grant_bypass": False,
            "duplicate_guarded_effects": 0,
            "unsafe_automatic_retries": 0,
            "false_completions": 0,
            "silent_event_drops": 0,
            "client_disconnect_authority": False,
            "all_local_crash_boundaries_exercised": True,
            "installed_active_document_parser_campaign_complete": False,
            "independent_review_complete": False,
            "windows_validation_complete": False,
            "macos_validation_complete": False,
            "release_claim": "none",
        },
        "limitations": [
            "active document and archive-like inputs are refused as unsupported bytes because structured parser adapters are not admitted",
            "the model boundary is deterministic and cannot substitute for an independently admitted production tuple",
            "installed-client, independent-review, final Windows, and deferred macOS evidence remains separate",
        ],
    }


def validate_report(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["Story 50.3 security/fault report is not an object"]
    failures: list[str] = []
    if value.get("crash_boundary_matrix") != list(BOUNDARIES):
        failures.append("crash boundary matrix is incomplete or reordered")
    if value.get("crash_seed_count") != 100:
        failures.append("crash seed count is not exact")
    truth = value.get("product_truth", {})
    for field in ("prompt_injection_records_inert", "all_local_crash_boundaries_exercised"):
        if truth.get(field) is not True:
            failures.append(f"{field} must remain true")
    required_zero = (
        "secret_canary_disclosures", "duplicate_guarded_effects", "unsafe_automatic_retries",
        "false_completions", "silent_event_drops",
    )
    for field in required_zero:
        if truth.get(field) != 0:
            failures.append(f"{field} must remain zero")
    required_false = (
        "active_or_archive_bytes_executed", "hostile_metadata_or_stale_reference_bypass",
        "model_output_authority", "approval_or_grant_bypass", "client_disconnect_authority",
        "installed_active_document_parser_campaign_complete", "independent_review_complete",
        "windows_validation_complete", "macos_validation_complete",
    )
    for field in required_false:
        if truth.get(field) is not False:
            failures.append(f"{field} must remain false")
    if truth.get("release_claim") != "none":
        failures.append("security/fault evidence cannot claim a release")
    return failures


def validate() -> list[str]:
    failures = [f"missing source: {path}" for path in SOURCES if not (ROOT / path).is_file()]
    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return failures + [f"cannot read retained security/fault evidence: {error}"]
    failures.extend(
        f"command {index} did not retain a passing exit"
        for index in range(len(COMMANDS))
        if f"COMMAND_{index}_EXIT=0" not in raw
    )
    markers = (
        "two_hundred_injections_never_create_guidance_or_authority_without_user_decision ... ok",
        "story_22_3_declared_encodings_are_exact_and_binary_or_lossy_input_fails ... ok",
        "stale_restricted_unsupported_redacted_missing_and_range_fail_closed ... ok",
        "cancellation_timeout_and_crash_are_receipted_for_every_tool ... ok",
        "story_16_3_post_approval_preflight_policy_and_grant_drift_start_no_worker ... ok",
        "story_16_3_crash_boundaries_restore_one_uncertain_terminal_without_replay ... ok",
        "STORY_50_3_CRASH_BOUNDARY_COUNT=14;SEEDS=100;REPLAY_COUNT=0",
        "story_23_4_request_and_outcome_tampering_fail_closed ... ok",
        "story_21_3_missing_duplicate_reordered_cross_bound_and_tampered_events_fail_closed ... ok",
        "uncertainty_cancellation_and_resource_failure_never_false_complete ... ok",
        "read_output_secret_classes_are_withheld_before_model_projection ... ok",
    )
    failures.extend(f"raw evidence lacks marker: {marker}" for marker in markers if marker not in raw)
    for prohibited in ("test result: FAILED", "not ok", "BEGIN PRIVATE KEY"):
        if prohibited in raw:
            failures.append(f"raw evidence contains prohibited marker: {prohibited}")
    failures.extend(validate_report(report))
    if report != expected_report():
        failures.append("Story 50.3 security/fault report is stale or widened")
    return failures


def capture() -> int:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    RAW_PATH.write_text("Story 50.3 security/fault capture in progress\n", encoding="utf-8")
    REPORT_PATH.write_text("{}\n", encoding="utf-8")
    records: list[str] = []
    for index, command in enumerate(COMMANDS):
        result = subprocess.run(
            command, cwd=ROOT, text=True, stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT, check=False,
        )
        records.append(f"$ {' '.join(command)}\n{result.stdout}\nCOMMAND_{index}_EXIT={result.returncode}")
        if result.returncode != 0:
            RAW_PATH.write_text("\n\n".join(records) + "\n", encoding="utf-8")
            return result.returncode
    RAW_PATH.write_text("\n\n".join(records) + "\n", encoding="utf-8")
    REPORT_PATH.write_text(json.dumps(expected_report(), indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    if args.write and capture() != 0:
        return 1
    failures = validate()
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    print("Story 50.3 local security/fault evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
