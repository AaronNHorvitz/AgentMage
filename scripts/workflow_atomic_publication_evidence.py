#!/usr/bin/env python3
"""Build and validate Sub-task 11.2.2.3 atomic workflow-publication evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-11/story-11.2"
RAW_PATH: Final = EVIDENCE_DIR / "workflow-atomic-publication-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "workflow-atomic-publication-report.json"
SOURCE_PATH: Final = ROOT / "kernel/engine/src/operational_store.rs"
COMMANDS: Final = (
    (
        "cargo",
        "test",
        "-p",
        "agentmage-kernel-engine",
        "operational_store::tests::workflow_state_event_checkpoint_cursor_and_artifact_reference_commit_atomically",
        "--locked",
    ),
    (
        "cargo",
        "test",
        "-p",
        "agentmage-kernel-engine",
        "operational_store::tests::workflow_projection_failure_retains_no_event_checkpoint_cursor_or_state",
        "--locked",
    ),
    (
        "cargo",
        "test",
        "-p",
        "agentmage-kernel-engine",
        "operational_store::tests::correctness_event_checkpoint_and_binding_survive_one_atomic_reopen",
        "--locked",
    ),
    (
        "cargo",
        "clippy",
        "-p",
        "agentmage-kernel-engine",
        "--all-targets",
        "--all-features",
        "--locked",
        "--",
        "-D",
        "warnings",
    ),
)
MARKERS: Final = (
    "workflow_state_event_checkpoint_cursor_and_artifact_reference_commit_atomically ... ok",
    "workflow_projection_failure_retains_no_event_checkpoint_cursor_or_state ... ok",
    "correctness_event_checkpoint_and_binding_survive_one_atomic_reopen ... ok",
)
SOURCE_MARKERS: Final = (
    "pub struct WorkflowStateMaterialization",
    "checkpoint_runtime_session_with_workflow_state",
    "persist_authority_with_workflow_checkpoint",
    "runtime_resume_binding_after_events: true",
    "append_transaction_events(&transaction, continuity.runtime_events)",
    "persist_workflow_state_materializations(&transaction, continuity.workflow_state)",
    "persist_runtime_resume_binding(&transaction, checkpoint, binding)",
    ".commit()",
)
TRUTH: Final = {
    "synthetic_data_only": True,
    "workflow_metadata_materializations_complete": True,
    "attempt_and_idempotency_invariants_complete": True,
    "atomic_event_projection_checkpoint_commit_complete": True,
    "full_workflow_crash_campaign_complete": False,
    "story_completion_claim": False,
    "sprint_completion_claim": False,
    "release_claim": "none",
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def artifact(path: str) -> dict[str, Any]:
    absolute = ROOT / path
    return {"path": path, "byte_length": absolute.stat().st_size, "sha256": sha256(absolute)}


def expected_report() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "agentmage-workflow-atomic-publication-evidence",
        "story_id": "11.2",
        "task_id": "11.2.2.3",
        "generated_on": "2026-08-30",
        "status": "pass-local-atomic-publication",
        "canonical_store": "operational-store",
        "transaction_members": [
            "correctness_event",
            "workflow_state_materialization",
            "session_checkpoint",
            "new_event_cursor",
            "referenced_runtime_artifacts",
        ],
        "failure_contract": {
            "single_immediate_sqlcipher_transaction": True,
            "generation_compare_and_swap": True,
            "new_event_cursor_exact": True,
            "artifact_reference_exact": True,
            "injected_projection_failure_retains_none": True,
            "writer_poisoned_after_ambiguous_storage_failure": True,
        },
        "commands": [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "artifacts": [
            artifact("kernel/engine/src/operational_store.rs"),
            artifact("docs/verification/story-11-2-workflow-atomic-publication-evidence.md"),
            artifact("scripts/workflow_atomic_publication_evidence.py"),
            artifact("tests/test_workflow_atomic_publication_evidence.py"),
            artifact(RAW_PATH.relative_to(ROOT).as_posix()),
        ],
        "product_truth": dict(TRUTH),
    }


def render(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n"


def validate_source(value: str) -> list[str]:
    failures = [f"source missing atomic publication marker: {marker}" for marker in SOURCE_MARKERS if marker not in value]
    transaction_start = value.find("fn persist_snapshot(")
    transaction_end = value.find("fn validate_workflow_state_batch(")
    transaction = value[transaction_start:transaction_end]
    for marker in (
        "append_transaction_events",
        "persist_workflow_state_materializations",
        "persist_runtime_resume_binding",
        ".commit()",
    ):
        if marker not in transaction:
            failures.append(f"canonical snapshot transaction missing member: {marker}")
    if "runtime_resume_binding_after_events: true" not in value:
        failures.append("new event cursor is not explicitly published after event append")
    return failures


def validate_raw(value: str) -> list[str]:
    failures = [f"raw results missing marker: {marker}" for marker in MARKERS if marker not in value]
    for prohibited in ("test result: FAILED", "error: could not compile", "warning:"):
        if prohibited in value:
            failures.append(f"raw results contain prohibited marker: {prohibited}")
    return failures


def validate_report(value: Any) -> list[str]:
    if value != expected_report():
        return ["workflow atomic-publication report is stale, incomplete, reordered, or widened"]
    if value.get("product_truth") != TRUTH:
        return ["workflow atomic-publication product truth was widened"]
    return []


def capture() -> tuple[str, int]:
    chunks: list[str] = []
    for command in COMMANDS:
        result = subprocess.run(
            command,
            cwd=ROOT,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            check=False,
        )
        chunks.append(f"$ {' '.join(command)}\n{result.stdout.rstrip(chr(10))}\n")
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
            return 1
        failures = validate_source(SOURCE_PATH.read_text(encoding="utf-8")) + validate_raw(raw)
        if failures:
            for failure in failures:
                print(f"workflow atomic-publication evidence build failed: {failure}", file=sys.stderr)
            return 1
        EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
        RAW_PATH.write_text(raw, encoding="utf-8")
        REPORT_PATH.write_text(render(expected_report()), encoding="utf-8")

    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        source = SOURCE_PATH.read_text(encoding="utf-8")
    except (OSError, json.JSONDecodeError) as error:
        print(f"workflow atomic-publication evidence validation failed: {error}", file=sys.stderr)
        return 1
    failures = validate_source(source) + validate_raw(raw) + validate_report(report)
    if failures:
        for failure in failures:
            print(f"workflow atomic-publication evidence validation failed: {failure}", file=sys.stderr)
        return 1
    print("Sub-task 11.2.2.3 atomic workflow publication validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
