#!/usr/bin/env python3
"""Generate retained local Story 21.4 lineage and inspector evidence."""

from __future__ import annotations

import hashlib
import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE_DIR = ROOT / "artifacts/sprints/sprint-21/story-21.4"
LOG = EVIDENCE_DIR / "lineage-inspector-results.log"
REPORT = EVIDENCE_DIR / "lineage-inspector-report.json"

COMMANDS = (
    ("cargo", "test", "-p", "agentmage-kernel-engine", "story_21_4", "--locked", "--", "--nocapture"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "story_21_2_lagging_subscriber_is_removed_without_blocking_runtime", "--locked"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "story_21_2_subscriber_bounds_and_disconnect_are_non_authoritative", "--locked"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "story_50_2_replay_rejects_cursor_drift_and_undersized_pages", "--locked"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "story_21_2_durable_model_progress_and_cancellation_survive_delayed_sqlcipher", "--locked"),
    ("node", "--test", "tests/test_planning_schemas.mjs"),
    ("cargo", "clippy", "-p", "agentmage-kernel-engine", "-p", "agentmage-host", "--all-targets", "--locked", "--", "-D", "warnings"),
)

SOURCES = (
    "kernel/contracts/src/runtime_event.rs",
    "kernel/engine/src/runtime_event.rs",
    "kernel/engine/src/runtime_inspector.rs",
    "kernel/engine/src/runtime_journal.rs",
    "kernel/engine/src/runtime_loop_tests.rs",
    "schemas/runtime/runtime-event.schema.json",
    "tests/test_planning_schemas.mjs",
    "docs/architecture/ordered-runtime-lineage-inspector.md",
    "scripts/story_21_4_lineage_inspector_evidence.py",
    "tests/test_story_21_4_lineage_inspector_evidence.py",
)

MARKERS = (
    "story_21_4_inspector_reconstructs_complete_content_free_lineage ... ok",
    "story_21_4_inspector_supports_prefix_resume_and_refuses_tampering ... ok",
    "story_21_4_lineage_gap_events_enforce_exact_ordering ... ok",
    "story_21_4_performance_qualification_is_complete_and_fail_closed ... ok",
    "story_21_4_measures_the_bounded_local_inspector_fixture ... ok",
    "AGENTMAGE_STORY_21_4_PERFORMANCE=",
    "story_21_2_lagging_subscriber_is_removed_without_blocking_runtime ... ok",
    "story_21_2_subscriber_bounds_and_disconnect_are_non_authoritative ... ok",
    "story_50_2_replay_rejects_cursor_drift_and_undersized_pages ... ok",
    "story_21_2_durable_model_progress_and_cancellation_survive_delayed_sqlcipher ... ok",
    "artifact and workflow projection event families remain closed",
)

PERFORMANCE_PATTERN = re.compile(
    r"AGENTMAGE_STORY_21_4_PERFORMANCE=total_ms:(\d+),events_per_second:(\d+),"
    r"p50_us:(\d+),p95_us:(\d+),p99_us:(\d+),artifact_bytes:(\d+)"
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
    match = PERFORMANCE_PATTERN.search(output)
    if match is None:
        raise SystemExit("missing measured performance distribution")
    total_ms, throughput, p50_us, p95_us, p99_us, artifact_bytes = map(int, match.groups())
    report: dict[str, object] = {
        "schema_version": 1,
        "record_type": "agentmage-story-21-4-lineage-inspector-evidence",
        "story_id": "21.4",
        "generated_on": "2026-08-31",
        "status": "PASS_LOCAL_LINEAGE_INSPECTOR",
        "commands": [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "artifacts": [artifact(path) for path in SOURCES]
        + [artifact(LOG.relative_to(ROOT).as_posix())],
        "lineage_fact_categories": 18,
        "performance_metrics_qualified": 17,
        "performance_fixture": {
            "fixture_id": "story-21-4-local-inspector-100x",
            "lineage_events_per_iteration": 26,
            "iterations": 100,
            "total_ms": total_ms,
            "events_per_second": throughput,
            "p50_us": p50_us,
            "p95_us": p95_us,
            "p99_us": p99_us,
            "artifact_bytes": artifact_bytes,
            "accelerator_applicable": False,
        },
        "product_truth": {
            "ordered_lineage_complete": True,
            "read_only_inspector_complete": True,
            "journal_reload_equivalence_complete": True,
            "materialized_view_authority_count": 0,
            "raw_secret_or_hidden_reasoning_field_count": 0,
            "all_declared_thresholds_fail_closed": True,
            "shared_backpressure_cursor_cancellation_gates_pass": True,
            "full_RV_56_executed": False,
            "installed_runtime_campaign_executed": False,
            "supported_platform_campaign_executed": False,
            "independent_qualification_executed": False,
            "release_claim": "none",
        },
        "gate_scope": {
            "local_story_slice": "complete",
            "RV-56_owner": "Story 95.3",
            "reason_not_claimed": "authoritative reusable-gate table assigns Engineering Capability Registry to Story 95.3",
        },
        "limitations": [
            "the measured fixture qualifies the local inspector contract, not installed-product latency",
            "supported-platform campaigns and independent qualification remain external",
        ],
    }
    REPORT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return report


if __name__ == "__main__":
    run()
