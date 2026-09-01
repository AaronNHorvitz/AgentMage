#!/usr/bin/env python3
"""Generate and validate source-bound local Story 22.4 evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-22/story-22.4"
RAW_PATH: Final = EVIDENCE_DIR / "attempt-recovery-results.log"
GOLDEN_PATH: Final = EVIDENCE_DIR / "attempt-recovery-goldens.json"
REPORT_PATH: Final = EVIDENCE_DIR / "attempt-recovery-report.json"
GENERATOR: Final = ("cargo", "run", "-p", "agentmage-kernel-engine", "--example", "story_22_4_evidence", "--locked")
COMMANDS: Final = (
    ("cargo", "test", "-p", "agentmage-kernel-engine", "durable_attempt_recovery", "--locked"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "attempt_checkpoint_and_single_resume_owner_survive_verified_restart", "--locked"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "story_22_4_recovery_fresh_attempt_composes_only_through_existing_admission", "--locked"),
    ("node", "--test", "tests/test_attempt_recovery_schemas.mjs"),
    ("cargo", "clippy", "-p", "agentmage-kernel-contracts", "-p", "agentmage-kernel-engine", "--all-targets", "--all-features", "--locked", "--", "-D", "warnings"),
)
MARKERS: Final = (
    "story_22_4_one_hundred_seed_no_unwind_boundary_campaign_has_zero_replay ... ok",
    "story_22_4_concurrent_clients_have_one_owner ... ok",
    "story_22_4_checkpoint_binds_every_required_identity_and_rejects_mutation ... ok",
    "attempt_checkpoint_and_single_resume_owner_survive_verified_restart ... ok",
    "story_22_4_recovery_fresh_attempt_composes_only_through_existing_admission ... ok",
    "attempt recovery schemas accept closed content-free records",
    "attempt recovery schemas reject replay, hidden content, missing budgets, and widening",
    "Finished `dev` profile",
)
SOURCES: Final = (
    "kernel/contracts/src/engineering_records.rs",
    "kernel/contracts/src/serialization.rs",
    "kernel/engine/src/durable_attempt_recovery.rs",
    "kernel/engine/src/operational_store.rs",
    "kernel/engine/migrations/operational-store/0018-attempt-recovery.sql",
    "kernel/engine/examples/story_22_4_evidence.rs",
    "schemas/runtime/attempt-recovery-checkpoint.schema.json",
    "schemas/runtime/attempt-recovery-decision.schema.json",
    "tests/test_attempt_recovery_schemas.mjs",
    "docs/architecture/durable-attempt-recovery-and-resume.md",
    "scripts/story_22_4_attempt_recovery_evidence.py",
    "tests/test_story_22_4_attempt_recovery_evidence.py",
)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def artifact(relative: str) -> dict[str, Any]:
    path = ROOT / relative
    return {"path": relative, "byte_length": path.stat().st_size, "sha256": sha256(path)}


def validate_golden(value: Any) -> list[str]:
    if not isinstance(value, dict) or value.get("record_type") != "agentmage-story-22-4-attempt-recovery-goldens":
        return ["golden record type is invalid"]
    failures: list[str] = []
    expected_actions = {"continue", "fresh_attempt", "deterministic_repair", "replan", "await_approval", "await_dependency", "reconcile_effect", "cancel", "terminate_diagnosed"}
    decisions = value.get("decisions", [])
    if {item.get("action") for item in decisions} != expected_actions:
        failures.append("closed recovery action family is incomplete")
    if any(item.get("replay_allowed") for item in decisions):
        failures.append("a recovery decision permits replay")
    if value.get("deterministic_seed_count") != 100 or value.get("interruption_boundary_count") != 14:
        failures.append("100-seed interruption matrix is incomplete")
    traces = value.get("crash_traces", [])
    expected_boundaries = {"source", "context", "proposal", "preflight", "approval", "grant", "worker", "receipt", "artifact", "verification", "retry", "recovery", "checkpoint", "terminal"}
    if len(traces) != 100 or {item.get("seed") for item in traces} != set(range(100)):
        failures.append("crash trace seed coverage is incomplete")
    if {item.get("boundary") for item in traces} != expected_boundaries or {item.get("position") for item in traces} != {"before", "after"}:
        failures.append("before/after interruption boundary coverage is incomplete")
    if any(item.get("replay_count") != 0 for item in traces):
        failures.append("a retained interruption trace reports replay")
    if len(value.get("invalidation_dimensions", [])) != 18:
        failures.append("checkpoint invalidation family is incomplete")
    graph = value.get("checkpoint_graph", {})
    if graph.get("current_node") != "attempt_checkpoint" or len(graph.get("edges", [])) != 2:
        failures.append("checkpoint graph is incomplete")
    diagnosis = value.get("terminal_diagnosis", {})
    if not diagnosis.get("blocked_reason_code") or len(diagnosis.get("diagnosis_sha256", "")) != 64:
        failures.append("terminal diagnosis is incomplete")
    if value.get("cleanup") != {"temporary_campaign_directories_remaining": 0, "orphan_resume_owners": 0, "unbound_checkpoint_count": 0}:
        failures.append("campaign cleanup is incomplete")
    for field, expected in (("concurrent_client_count", 32), ("durable_owner_count", 1), ("duplicate_effect_count", 0), ("replay_count", 0), ("external_values_retained", 0)):
        if value.get(field) != expected:
            failures.append(f"{field} is not {expected}")
    encoded = json.dumps(value, sort_keys=True)
    for prohibited in ("tool_arguments", "model_output", "/home/", "/var/home/", "BEGIN PRIVATE KEY"):
        if prohibited in encoded:
            failures.append(f"golden contains prohibited content: {prohibited}")
    return failures


def expected_report() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "agentmage-story-22-4-attempt-recovery-evidence",
        "story_id": "22.4",
        "generated_on": "2026-08-31",
        "status": "PASS_LOCAL_DURABLE_RECOVERY",
        "protocols": ["RV-12", "RV-16", "RV-17", "RV-18", "RV-25"],
        "commands": [" ".join(GENERATOR)] + [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "artifacts": [artifact(path) for path in SOURCES] + [artifact(RAW_PATH.relative_to(ROOT).as_posix()), artifact(GOLDEN_PATH.relative_to(ROOT).as_posix())],
        "product_truth": {
            "attempt_checkpoint_identity_families": 18,
            "closed_recovery_actions": 9,
            "deterministic_interruption_seeds": 100,
            "interruption_boundaries": 14,
            "concurrent_clients": 32,
            "durable_resume_owners": 1,
            "replayed_effects": 0,
            "duplicate_effects": 0,
            "encrypted_store_restart_complete": True,
            "budget_and_repeat_state_retained": True,
            "consumed_authority_and_uncertainty_retained": True,
            "installed_package_campaign_complete": False,
            "windows_campaign_complete": False,
            "physical_fault_campaign_complete": False,
            "independent_review_complete": False,
            "release_claim": "none",
        },
        "remaining_external_work": [
            "run the attempt-recovery campaign from an installed package on Linux and Windows",
            "retain physical power-loss and storage-fault evidence",
            "obtain independent human security review of the recovery boundary",
        ],
    }


def validate() -> list[str]:
    failures = [f"missing source: {path}" for path in SOURCES if not (ROOT / path).is_file()]
    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        golden = json.loads(GOLDEN_PATH.read_text(encoding="utf-8"))
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return failures + [f"cannot read retained Story 22.4 evidence: {error}"]
    failures.extend(f"raw evidence missing marker: {marker}" for marker in MARKERS if marker not in raw)
    failures.extend(f"raw evidence contains prohibited marker: {marker}" for marker in ("test result: FAILED", "error: could not compile", "not ok") if marker in raw)
    failures += validate_golden(golden)
    if report != expected_report():
        failures.append("Story 22.4 report is stale or widened")
    return failures


def capture() -> int:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    generated = subprocess.run(GENERATOR, cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
    records = [f"$ {' '.join(GENERATOR)}\n{generated.stderr}"]
    if generated.returncode != 0:
        RAW_PATH.write_text("\n".join(records), encoding="utf-8")
        return generated.returncode
    try:
        golden = json.loads(generated.stdout)
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
    print("Story 22.4 local durable attempt recovery evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
