#!/usr/bin/env python3
"""Generate retained local Story 16.4 terminal-observation evidence."""

from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE_DIR = ROOT / "artifacts/sprints/sprint-16/story-16.4"
LOG = EVIDENCE_DIR / "tool-observation-results.log"
REPORT = EVIDENCE_DIR / "tool-observation-report.json"

COMMANDS = (
    ("cargo", "test", "-p", "agentmage-kernel-engine", "tool_observation::tests", "--locked"),
    (
        "cargo", "test", "-p", "agentmage-kernel-engine",
        "engineering_records::tests::tool_failure_may_report_incomplete_cleanup_but_success_may_not",
        "--locked",
    ),
    (
        "cargo", "clippy", "-p", "agentmage-kernel-engine", "--all-targets",
        "--locked", "--", "-D", "warnings",
    ),
)

SOURCES = (
    "kernel/engine/src/tool_observation.rs",
    "kernel/engine/src/tool_composition.rs",
    "kernel/engine/src/engineering_records.rs",
    "kernel/engine/src/lib.rs",
    "docs/architecture/closed-tool-observation-contract.md",
    "scripts/story_16_4_tool_observation_evidence.py",
    "tests/test_story_16_4_tool_observation_evidence.py",
)

MARKERS = (
    "story_16_4_all_ten_terminal_states_emit_one_closed_observation ... ok",
    "story_16_4_complete_outputs_are_separate_content_addressed_and_excerpts_are_bounded_redacted ... ok",
    "story_16_4_duplicate_mismatch_forgery_and_missing_artifacts_fail_closed ... ok",
    "story_16_4_output_and_excerpt_boundaries_fail_before_publication ... ok",
    "story_16_4_every_measured_resource_ceiling_fails_closed_before_publication ... ok",
    "story_16_4_crash_race_pipe_and_uncertainty_never_infer_success ... ok",
    "story_16_4_tampered_observation_cannot_claim_more_than_measured ... ok",
    "story_16_4_restart_restore_rejects_duplicate_corruption_and_replay ... ok",
    "story_16_4_live_local_process_output_closes_through_the_same_assembler ... ok",
    "tool_failure_may_report_incomplete_cleanup_but_success_may_not ... ok",
)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def artifact(relative: str) -> dict[str, object]:
    path = ROOT / relative
    return {"path": relative, "byte_length": path.stat().st_size, "sha256": sha256(path)}


def run() -> dict[str, object]:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    records: list[str] = []
    for command in COMMANDS:
        completed = subprocess.run(
            command, cwd=ROOT, check=False, stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT, text=True,
        )
        records.append(f"$ {' '.join(command)}\n{completed.stdout}")
        if completed.returncode != 0:
            LOG.write_text("\n".join(records), encoding="utf-8")
            raise SystemExit(completed.returncode)
    LOG.write_text("\n".join(records), encoding="utf-8")
    output = LOG.read_text(encoding="utf-8")
    missing = [marker for marker in MARKERS if marker not in output]
    if missing:
        raise SystemExit(f"missing retained markers: {missing}")
    report: dict[str, object] = {
        "schema_version": 1,
        "record_type": "agentmage-story-16-4-tool-observation-evidence",
        "story_id": "16.4",
        "generated_on": "2026-08-31",
        "status": "PASS_LOCAL_CLOSED_OBSERVATION",
        "commands": [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "artifacts": [artifact(path) for path in SOURCES]
        + [artifact(LOG.relative_to(ROOT).as_posix())],
        "terminal_disposition_count": 10,
        "output_artifact_families": ["stdout", "stderr", "binary", "structured"],
        "product_truth": {
            "exactly_once_terminal_contract_complete": True,
            "atomic_content_addressed_output_contract_complete": True,
            "bounded_redacted_excerpt_contract_complete": True,
            "resource_ceiling_matrix_complete": True,
            "hostile_terminal_matrix_complete": True,
            "restart_restore_matrix_complete": True,
            "live_local_process_executed": True,
            "false_completion_count": 0,
            "installed_worker_campaign_executed": False,
            "release_claim": "none",
        },
        "limitations": [
            "the live fixture is a bounded local process feeding the same observation assembler",
            "installed native worker, macOS XPC, and independent release evidence remain separate gates",
        ],
    }
    REPORT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return report


if __name__ == "__main__":
    run()
