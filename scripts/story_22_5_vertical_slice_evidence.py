#!/usr/bin/env python3
"""Generate and validate source-bound local Story 22.5 evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-22/story-22.5"
RAW_PATH: Final = EVIDENCE_DIR / "vertical-slice-results.log"
GOLDEN_PATH: Final = EVIDENCE_DIR / "vertical-slice-golden.json"
REPORT_PATH: Final = EVIDENCE_DIR / "vertical-slice-report.json"
GOLDEN_PREFIX: Final = "STORY_22_5_EVIDENCE="
GOLDEN_COMMAND: Final = (
    "cargo", "test", "-p", "agentmage-host",
    "story_22_5_prepared_source_flows_through_model_tool_verifier_and_terminal_lineage",
    "--locked", "--", "--nocapture",
)
COMMANDS: Final = (
    ("cargo", "test", "-p", "agentmage-host", "story_22_5_", "--locked"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "story_22_5_durable_prepared_source_checkpoint_resumes_without_replaying_effect", "--locked"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "runtime_loop::tests::story_23_4_", "--locked"),
    ("cargo", "test", "-p", "agentmage-host", "story_50_2_read_only_and_coding_packets_are_equal_across_all_three_callers", "--locked"),
    ("python3", "scripts/story_22_4_attempt_recovery_evidence.py"),
    ("cargo", "clippy", "-p", "agentmage-kernel-engine", "-p", "agentmage-host", "--all-targets", "--all-features", "--locked", "--", "-D", "warnings"),
)
MARKERS: Final = (
    "story_22_5_prepared_source_flows_through_model_tool_verifier_and_terminal_lineage ... ok",
    "story_22_5_stale_required_source_and_tokenizer_drift_stop_before_model ... ok",
    "story_22_5_durable_prepared_source_checkpoint_resumes_without_replaying_effect ... ok",
    "story_23_4_malformed_model_result_fails_closed_with_terminal_evidence ... ok",
    "story_23_4_cancellation_wins_over_a_pending_allow_response ... ok",
    "story_23_4_deny_closes_without_launching_the_tool ... ok",
    "story_50_2_read_only_and_coding_packets_are_equal_across_all_three_callers ... ok",
    "Story 22.4 local durable attempt recovery evidence validated",
    "Finished `dev` profile",
)
SOURCES: Final = (
    "kernel/engine/src/source_runtime_context.rs",
    "kernel/engine/src/source_preparation.rs",
    "kernel/engine/src/runtime_loop.rs",
    "kernel/engine/src/runtime_loop_tests.rs",
    "shells/host/src/source_artifact_runtime.rs",
    "shells/host/src/runtime_read_tests.rs",
    "shells/host/src/coding_client.rs",
    "shells/host/src/runtime_parity_tests.rs",
    "docs/architecture/engineering-runtime-vertical-slice.md",
    "scripts/story_22_5_vertical_slice_evidence.py",
    "tests/test_story_22_5_vertical_slice_evidence.py",
)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def artifact(relative: str) -> dict[str, Any]:
    path = ROOT / relative
    return {"path": relative, "byte_length": path.stat().st_size, "sha256": sha256(path)}


def validate_golden(value: Any) -> list[str]:
    if not isinstance(value, dict) or value.get("record_type") != "agentmage-story-22-5-vertical-slice-golden":
        return ["golden record type is invalid"]
    failures: list[str] = []
    source = value.get("source_manifest", {})
    if source.get("lifecycle") != "current" or source.get("extraction_state") != "complete":
        failures.append("prepared source is not current and complete")
    manifests = value.get("context_manifests", [])
    if len(manifests) != 2:
        failures.append("the two-turn context history is incomplete")
    for manifest in manifests:
        records = manifest.get("records", [])
        if not any(record.get("source_id") == source.get("source_id") and record.get("section_id") and record.get("disposition") == "included" for record in records):
            failures.append("a turn omitted the required prepared source")
        if manifest.get("used_tokens", 0) <= 0 or manifest.get("used_tokens", 0) > manifest.get("allocated_tokens", 0):
            failures.append("a context manifest has invalid token accounting")
    events = value.get("events", [])
    if not events or [event.get("sequence") for event in events] != list(range(len(events))):
        failures.append("runtime event sequence is incomplete")
    for prior, current in zip(events, events[1:]):
        if current.get("previous_event_sha256") != prior.get("event_sha256"):
            failures.append("runtime event digest continuity is broken")
            break
    kinds = {event.get("kind", {}).get("event") for event in events}
    required_kinds = {"run_started", "turn_started", "model_requested", "model_completed", "tool_requested", "permission_decided", "tool_started", "tool_completed", "turn_completed", "run_terminal"}
    if not required_kinds.issubset(kinds):
        failures.append("source-to-terminal runtime event family is incomplete")
    outcome = value.get("outcome", {})
    if outcome.get("state") != "SUCCESS" or outcome.get("turn_count") != 2 or outcome.get("model_call_count") != 2 or outcome.get("tool_call_count") != 1:
        failures.append("verified terminal outcome counters are invalid")
    if len(outcome.get("receipt_ids", [])) != 1 or not outcome.get("answer_evidence"):
        failures.append("terminal receipt or answer evidence is missing")
    source_evidence = [item for item in outcome.get("evidence", []) if item.get("source_id") == source.get("source_id")]
    if len(source_evidence) != 1 or source.get("manifest_sha256") not in source_evidence[0].get("fragment", ""):
        failures.append("source freshness citation is missing")
    for field, expected in (
        ("production_artifact_dispatch", True),
        ("model_direct_tool_authority", False),
        ("model_completion_authority", False),
        ("duplicate_effect_count", 0),
        ("replay_count", 0),
    ):
        if value.get(field) != expected:
            failures.append(f"{field} is not {expected}")
    encoded = json.dumps(value, sort_keys=True)
    for prohibited in ("/home/", "/var/home/", "BEGIN PRIVATE KEY", "tool_arguments"):
        if prohibited in encoded:
            failures.append(f"golden contains prohibited content: {prohibited}")
    return failures


def expected_report() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "agentmage-story-22-5-vertical-slice-evidence",
        "story_id": "22.5",
        "generated_on": "2026-08-31",
        "status": "PASS_LOCAL_DETERMINISTIC_VERTICAL_SLICE",
        "protocols": ["RV-50", "RV-51", "RV-52", "RV-53", "RV-54", "RV-55", "RV-56"],
        "commands": [" ".join(GOLDEN_COMMAND)] + [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "artifacts": [artifact(path) for path in SOURCES] + [artifact(RAW_PATH.relative_to(ROOT).as_posix()), artifact(GOLDEN_PATH.relative_to(ROOT).as_posix())],
        "product_truth": {
            "prepared_source_turns": 2,
            "model_calls": 2,
            "production_artifact_tool_calls": 1,
            "terminal_receipts": 1,
            "verified_success": True,
            "three_client_event_outcome_parity": True,
            "durable_resume_worker_executions": 1,
            "stale_source_model_calls": 0,
            "replayed_effects": 0,
            "duplicate_effects": 0,
            "qualified_production_model": False,
            "installed_package_campaign_complete": False,
            "native_cross_platform_campaign_complete": False,
            "independent_review_complete": False,
            "release_claim": "none",
        },
        "remaining_external_work": [
            "repeat the slice with the first independently qualified production model profile",
            "run installed-package native Chat, CLI, and headless campaigns on supported platforms",
            "obtain independent human review and release qualification",
        ],
    }


def validate() -> list[str]:
    failures = [f"missing source: {path}" for path in SOURCES if not (ROOT / path).is_file()]
    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        golden = json.loads(GOLDEN_PATH.read_text(encoding="utf-8"))
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return failures + [f"cannot read retained Story 22.5 evidence: {error}"]
    failures.extend(f"raw evidence missing marker: {marker}" for marker in MARKERS if marker not in raw)
    failures.extend(f"raw evidence contains prohibited marker: {marker}" for marker in ("test result: FAILED", "error: could not compile", "not ok") if marker in raw)
    failures.extend(validate_golden(golden))
    if report != expected_report():
        failures.append("Story 22.5 report is stale or widened")
    return failures


def capture() -> int:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    environment = os.environ.copy()
    environment["AGENTMAGE_STORY_22_5_EVIDENCE"] = "1"
    generated = subprocess.run(GOLDEN_COMMAND, cwd=ROOT, env=environment, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=False)
    records = [f"$ {' '.join(GOLDEN_COMMAND)}\n{generated.stdout}"]
    if generated.returncode != 0:
        RAW_PATH.write_text("\n".join(records), encoding="utf-8")
        return generated.returncode
    golden_line = next((line for line in generated.stdout.splitlines() if line.startswith(GOLDEN_PREFIX)), None)
    if golden_line is None:
        RAW_PATH.write_text("\n".join(records), encoding="utf-8")
        return 1
    try:
        golden = json.loads(golden_line.removeprefix(GOLDEN_PREFIX))
    except json.JSONDecodeError:
        RAW_PATH.write_text("\n".join(records), encoding="utf-8")
        return 1
    GOLDEN_PATH.write_text(json.dumps(golden, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    for command in COMMANDS:
        result = subprocess.run(command, cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=False)
        records.append(f"$ {' '.join(command)}\n{result.stdout}")
        if result.returncode != 0:
            RAW_PATH.write_text("\n".join(records), encoding="utf-8")
            return result.returncode
    RAW_PATH.write_text("\n".join(records), encoding="utf-8")
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
    print("Story 22.5 local Engineering Runtime vertical-slice evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
