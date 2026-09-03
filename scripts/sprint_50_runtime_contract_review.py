#!/usr/bin/env python3
"""Build the gate-owned Sprint 50 shared-runtime contract review."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
REPORT: Final = ROOT / "artifacts/sprints/sprint-50/runtime-contract-review.json"
SOURCES: Final = (
    "kernel/contracts/Cargo.toml",
    "kernel/contracts/src/runtime_run.rs",
    "kernel/contracts/src/runtime_event.rs",
    "kernel/contracts/src/runtime_artifact.rs",
    "kernel/engine/src/runtime_coordinator.rs",
    "kernel/engine/src/runtime_hardening.rs",
    "kernel/engine/src/runtime_lifecycle.rs",
    "kernel/engine/src/runtime_loop.rs",
    "kernel/engine/src/runtime_loop_tests.rs",
    "kernel/engine/src/runtime_recovery.rs",
)
CEILINGS: Final = (
    "run",
    "turn",
    "model",
    "context",
    "tool",
    "process",
    "event_queue",
    "artifact",
    "output",
    "memory",
    "disk",
    "elapsed_time",
    "retry",
    "denial",
    "parser_failure",
    "no_progress",
)
RECOVERY_BOUNDARIES: Final = (
    "model",
    "context",
    "approval",
    "write",
    "command",
    "validation",
    "event",
    "artifact",
    "checkpoint",
    "terminal",
)


def git_bytes(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"],
        cwd=ROOT,
        capture_output=True,
        check=False,
        timeout=30,
    )
    if result.returncode:
        raise ValueError(f"runtime review source unavailable: {path}")
    return result.stdout


def _has_all(text: str, tokens: tuple[str, ...]) -> bool:
    return all(token in text for token in tokens)


def expected(revision: str) -> dict[str, Any]:
    sources = {path: git_bytes(revision, path) for path in SOURCES}
    decoded = {path: data.decode("utf-8") for path, data in sources.items()}
    request = decoded["kernel/contracts/src/runtime_run.rs"]
    artifact = decoded["kernel/contracts/src/runtime_artifact.rs"]
    coordinator = decoded["kernel/engine/src/runtime_coordinator.rs"]
    hardening = decoded["kernel/engine/src/runtime_hardening.rs"]
    lifecycle = decoded["kernel/engine/src/runtime_lifecycle.rs"]
    runtime_loop = decoded["kernel/engine/src/runtime_loop.rs"]
    loop_tests = decoded["kernel/engine/src/runtime_loop_tests.rs"]
    recovery = decoded["kernel/engine/src/runtime_recovery.rs"]
    contracts_manifest = decoded["kernel/contracts/Cargo.toml"]
    contract_sources = "\n".join(
        decoded[path]
        for path in SOURCES
        if path.startswith("kernel/contracts/src/runtime_")
    )

    checks = {
        "sixteen_runtime_ceilings_are_enforced": _has_all(
            request,
            (
                "max_turns",
                "max_model_calls",
                "max_tool_calls",
                "max_no_progress_turns",
                "max_context_refreshes",
                "max_events",
                "max_elapsed_ms",
                "max_output_bytes",
            ),
        )
        and _has_all(
            hardening,
            (
                "BudgetResource::PlanSteps",
                "BudgetResource::ModelCalls",
                "BudgetResource::ToolCalls",
                "BudgetResource::InputBytes",
                "BudgetResource::OutputBytes",
                "BudgetResource::ElapsedMilliseconds",
                "BudgetResource::MemoryBytes",
                "BudgetResource::DiskBytes",
                "BudgetResource::ProcessCount",
                "client_queue_events",
                "artifact_count",
                "retry_attempts: 0",
                "denials: 1",
                "parser_failures: 1",
            ),
        ),
        "one_visible_exhaustion_result_is_bound": _has_all(
            runtime_loop,
            (
                "AgentStateKind::Exhausted",
                '"runtime.budget.exhausted"',
                '"runtime.no_progress.exhausted"',
            ),
        )
        and _has_all(
            loop_tests,
            (
                "story_50_2_cumulative_budget_overage_is_non_mutating",
                "story_50_2_peak_memory_and_failure_ceilings_are_non_mutating",
                "story_50_2_event_and_artifact_overage_is_non_mutating",
            ),
        ),
        "ten_restart_boundaries_choose_one_safe_action": _has_all(
            recovery,
            tuple(f"Self::{name.title()}" for name in RECOVERY_BOUNDARIES)
            + (
                "pub const ALL: [Self; 10]",
                "pub enum RuntimeSafeNextAction",
                "replay_effect_allowed: false",
                "story_50_2_every_boundary_selects_one_closed_safe_action",
                "story_50_2_consumed_or_uncertain_effect_is_never_replayed",
            ),
        ),
        "durable_usage_cannot_reset_or_drift": _has_all(
            hardening,
            (
                "pub fn restore(",
                "pub fn reconcile_durable_projection(",
                "RuntimeHardeningError::InvalidLimits",
            ),
        )
        and "story_50_2_durable_restore_rejects_usage_drift" in loop_tests,
        "six_lifecycle_domains_keep_independent_owners": _has_all(
            lifecycle,
            (
                "pub const ALL: [Self; 6]",
                "pub const ALL: [Self; 5]",
                "RuntimeLifecycleMode::SafeMode",
                "carries_authority: false",
                "story_50_2_every_domain_and_action_retains_one_exact_owner",
                "story_50_2_safe_mode_allows_only_diagnostics_export_and_preserving_hold",
                "story_50_2_optional_transcript_and_metric_collection_requires_current_opt_in",
                "story_50_2_deletion_requires_release_dependencies_and_worktree_cleanup",
            ),
        ),
        "runtime_contracts_are_interface_neutral": (
            "description = \"Interface-independent AgentMage contracts\""
            in contracts_manifest
            and all(
                token not in contract_sources
                for token in (
                    "shells::",
                    "vscode::",
                    "desktop::",
                    "terminal::",
                    "WorkflowRendering",
                    "VisualStudioCode",
                )
            )
            and _has_all(
                coordinator,
                (
                    "validate_runtime_run_request_shape",
                    "validate_runtime_outcome_shape",
                ),
            )
            and _has_all(
                artifact,
                ("Path-free runtime artifact", "RuntimeResumeBinding"),
            )
        ),
    }
    return {
        "schema_version": 1,
        "record_type": "agentmage-sprint-50-runtime-contract-review",
        "source_revision": revision,
        "reviewer_identity": "scripts/sprint_50_runtime_contract_review.py",
        "review_class": "gate-owned-automated-shared-runtime-contract-review",
        "independent_human_review_performed": False,
        "source_sha256": {
            path: hashlib.sha256(data).hexdigest() for path, data in sources.items()
        },
        "ceiling_inventory": list(CEILINGS),
        "recovery_boundary_inventory": list(RECOVERY_BOUNDARIES),
        "checks": checks,
        "status": "PASS_LOCAL_RUNTIME_CONTRACT_REVIEW"
        if all(checks.values())
        else "FAIL",
        "limitations": [
            "This is gate-owned automated review, not a human-review claim.",
            "It proves local contract, deterministic fake-runtime, and source boundaries only.",
            "Installed clients, live models, physical fault injection, and platform campaigns remain absent.",
            "It grants no model, interface, platform, integration, or release support.",
        ],
    }


def validate(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["review is not an object"]
    revision = value.get("source_revision")
    if (
        not isinstance(revision, str)
        or len(revision) != 40
        or any(character not in "0123456789abcdef" for character in revision)
    ):
        return ["source revision invalid"]
    try:
        return [] if value == expected(revision) else ["review is stale, incomplete, or widened"]
    except (UnicodeDecodeError, ValueError) as error:
        return [str(error)]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    revision = subprocess.check_output(
        ["git", "rev-parse", arguments.source_revision], cwd=ROOT, text=True
    ).strip()
    if arguments.write:
        REPORT.parent.mkdir(parents=True, exist_ok=True)
        REPORT.write_text(
            json.dumps(expected(revision), indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
    try:
        value = json.loads(REPORT.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(error, file=sys.stderr)
        return 1
    failures = validate(value)
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    print("Sprint 50 shared-runtime contract review passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
