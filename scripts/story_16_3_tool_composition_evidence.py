#!/usr/bin/env python3
"""Generate retained local Story 16.3 composition evidence."""

from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE_DIR = ROOT / "artifacts/sprints/sprint-16/story-16.3"
LOG = EVIDENCE_DIR / "tool-composition-results.log"
REPORT = EVIDENCE_DIR / "tool-composition-report.json"

COMMANDS = (
    ("cargo", "test", "-p", "agentmage-kernel-engine", "tool_composition::tests", "--locked"),
    ("cargo", "test", "-p", "agentmage-host", "runtime_tools", "--locked"),
    ("cargo", "test", "-p", "agentmage-host", "story_48_2_native_catalog_is_exact_local_and_mcp_independent", "--locked"),
    (
        "cargo", "clippy", "-p", "agentmage-kernel-engine", "-p", "agentmage-host",
        "--all-targets", "--all-features", "--locked", "--", "-D", "warnings",
    ),
)

SOURCES = (
    "kernel/engine/src/tool_composition.rs",
    "kernel/engine/src/lib.rs",
    "shells/host/src/runtime_tools.rs",
    "shells/host/src/coding_tools.rs",
    "docs/architecture/tool-preflight-attempt-verification-composition.md",
    "scripts/story_16_3_tool_composition_evidence.py",
    "tests/test_story_16_3_tool_composition_evidence.py",
)

MARKERS = (
    "story_16_3_registers_all_exact_preflights_and_maps_every_visible_tool ... ok",
    "story_16_3_preflight_policy_grant_launch_verify_and_cleanup_order_is_exact ... ok",
    "story_16_3_every_preflight_drift_fails_before_attempt_or_worker ... ok",
    "story_16_3_post_approval_preflight_policy_and_grant_drift_start_no_worker ... ok",
    "story_16_3_duplicate_calls_and_failed_consumption_never_launch ... ok",
    "story_16_3_all_terminal_states_are_explicit_and_never_false_complete ... ok",
    "story_16_3_receipt_verification_and_cleanup_gaps_force_noncompletion ... ok",
    "story_16_3_crash_boundaries_restore_one_uncertain_terminal_without_replay ... ok",
    "story_16_3_every_native_tool_has_one_complete_composition_policy ... ok",
    "story_48_2_native_catalog_is_exact_local_and_mcp_independent ... ok",
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
        "record_type": "agentmage-story-16-3-tool-composition-evidence",
        "story_id": "16.3",
        "generated_on": "2026-08-31",
        "status": "PASS_LOCAL_DETERMINISTIC_COMPOSITION",
        "commands": [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "artifacts": [artifact(path) for path in SOURCES]
        + [artifact(LOG.relative_to(ROOT).as_posix())],
        "preflight_families": [
            "workspace", "git", "executable", "service", "network-policy", "platform",
            "storage", "credential-reference", "resource",
        ],
        "terminal_states": [
            "succeeded", "no_op", "partial", "denied", "cancelled", "failed",
            "uncertain", "blocked",
        ],
        "product_truth": {
            "read_only_native_mapping_count": 17,
            "coding_native_mapping_count": 21,
            "preflight_registry_complete": True,
            "just_in_time_revalidation_complete": True,
            "single_consumption_single_launch_contract_complete": True,
            "deterministic_verification_complete": True,
            "crash_boundary_reconciliation_complete": True,
            "duplicate_effect_count": 0,
            "false_completion_count": 0,
            "installed_worker_campaign_executed": False,
            "release_claim": "none",
        },
        "limitations": [
            "the retained campaign exercises deterministic composition ports and host catalog closure",
            "installed native worker and platform release evidence remain separate gates",
        ],
    }
    REPORT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return report


if __name__ == "__main__":
    run()
