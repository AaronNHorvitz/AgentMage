#!/usr/bin/env python3
"""Generate retained local Story 21.3 event-projection evidence."""

from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE_DIR = ROOT / "artifacts/sprints/sprint-21/story-21.3"
LOG = EVIDENCE_DIR / "event-projection-results.log"
REPORT = EVIDENCE_DIR / "event-projection-report.json"

COMMANDS = (
    ("cargo", "test", "-p", "agentmage-kernel-engine", "story_21_3", "--locked"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "story_21_2_lagging_subscriber_is_removed_without_blocking_runtime", "--locked"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "story_21_2_dedicated_writer_keeps_progress_and_clients_off_slow_store", "--locked"),
    ("node", "--test", "tests/test_planning_schemas.mjs"),
    ("cargo", "clippy", "-p", "agentmage-kernel-engine", "-p", "agentmage-host", "--all-targets", "--locked", "--", "-D", "warnings"),
)

SOURCES = (
    "kernel/contracts/src/runtime_event.rs",
    "kernel/engine/src/runtime_event.rs",
    "kernel/engine/src/runtime_journal.rs",
    "shells/host/src/cli.rs",
    "schemas/runtime/runtime-event.schema.json",
    "tests/test_planning_schemas.mjs",
    "docs/architecture/artifact-workflow-event-projection.md",
    "scripts/story_21_3_event_projection_evidence.py",
    "tests/test_story_21_3_event_projection_evidence.py",
)

MARKERS = (
    "story_21_3_complete_artifact_workflow_replays_to_one_exact_projection ... ok",
    "story_21_3_missing_duplicate_reordered_cross_bound_and_tampered_events_fail_closed ... ok",
    "story_21_3_blocked_extraction_is_terminal_for_indexing_and_remains_visible ... ok",
    "story_21_3_large_and_sensitive_payloads_remain_reference_only ... ok",
    "story_21_3_legacy_clients_get_explicit_unsupported_kind_without_breaking_old_journals ... ok",
    "story_21_3_materialized_projection_drift_never_overrides_canonical_replay ... ok",
    "story_21_3_rejected_duplicate_identity_events_do_not_mutate_replay_state ... ok",
    "story_21_3_new_correctness_events_commit_reopen_and_replay_exactly ... ok",
    "story_21_3_every_new_correctness_event_survives_before_and_after_process_stop ... ok",
    "story_21_2_lagging_subscriber_is_removed_without_blocking_runtime ... ok",
    "story_21_2_dedicated_writer_keeps_progress_and_clients_off_slow_store ... ok",
    "artifact and workflow projection event families remain closed",
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
        "record_type": "agentmage-story-21-3-event-projection-evidence",
        "story_id": "21.3",
        "generated_on": "2026-08-31",
        "status": "PASS_LOCAL_EVENT_PROJECTION",
        "commands": [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "artifacts": [artifact(path) for path in SOURCES]
        + [artifact(LOG.relative_to(ROOT).as_posix())],
        "new_correctness_event_families": 13,
        "process_stop_cases": 30,
        "product_truth": {
            "closed_schema_complete": True,
            "legal_ordering_complete": True,
            "deterministic_projection_complete": True,
            "encrypted_restart_replay_complete": True,
            "legacy_journal_compatibility_complete": True,
            "unsupported_client_result_explicit": True,
            "large_or_sensitive_inline_payload_count": 0,
            "duplicate_or_invented_event_count": 0,
            "shared_pressure_and_slow_client_gates_pass": True,
            "external_platform_campaign_executed": False,
            "independent_review_executed": False,
            "release_claim": "none",
        },
        "reviewer_protocol_mapping": {
            "RV-08": "local secret/reference-only and closed-code slice",
            "RV-17": "local thirty-case process-stop and exact replay slice",
            "RV-18": "local journal pressure, slow-client, ordering, and privacy slice",
        },
        "limitations": [
            "physical storage fault injection and installed-runtime pressure remain broader gates",
            "supported-platform campaigns and independent review remain external",
        ],
    }
    REPORT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return report


if __name__ == "__main__":
    run()
