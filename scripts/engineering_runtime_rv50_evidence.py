#!/usr/bin/env python3
"""Execute and retain the Story 1.3-applicable portions of reviewer protocol RV-50."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts" / "sprints" / "sprint-1" / "story-1.3"
RAW_PATH: Final = EVIDENCE_DIR / "rv50-applicable-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "rv50-applicability.json"
COMMANDS: Final = [
    "cargo test -p agentmage-kernel-engine engineering_records::tests --locked",
    "cargo test -p agentmage-kernel-engine --test engineering_runtime_record_corpus --locked",
    "cargo test -p agentmage-host runtime_parity_tests --locked",
    "cargo test -p agentmage-host headless::tests::replay_resume_cancellation_and_transport_failure_do_not_duplicate_launch --locked",
    "cargo test -p agentmage-host runtime_read_tests::story_23_4_fake_model_uses_existing_native_read_tool_then_verifies_completion --locked",
]
MARKERS: Final = [
    "workflow_transition_table_is_closed_and_terminal_states_are_absorbing ... ok",
    "runtime_record_set_binds_request_task_session_source_workflow_and_proof ... ok",
    "success_and_delivery_records_fail_closed_on_missing_proof ... ok",
    "cyclic_and_stale_records_fail_at_their_exact_trusted_boundaries ... ok",
    "story_50_2_read_only_and_coding_packets_are_equal_across_all_three_callers ... ok",
    "story_50_2_narrow_workflow_authority_rejects_every_broadening_without_execution ... ok",
    "replay_resume_cancellation_and_transport_failure_do_not_duplicate_launch ... ok",
    "story_23_4_fake_model_uses_existing_native_read_tool_then_verifies_completion ... ok",
]


def _sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _artifact(relative: str) -> dict[str, str]:
    return {"path": relative, "sha256": _sha(ROOT / relative)}


def expected_report() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "engineering-runtime-rv50-applicability",
        "decision_id": "ADR-0043",
        "story_id": "1.3",
        "task_id": "1.3.3.2",
        "protocol_id": "RV-50",
        "generated_on": "2026-08-29",
        "status": "PARTIAL_LOCAL_CONTRACT_EVIDENCE",
        "protocol_complete": False,
        "commands": list(COMMANDS),
        "required_markers": list(MARKERS),
        "applicable_results": [
            {
                "scenario": "rust-owned-state-and-transition-closure",
                "status": "PASS_LOCAL_CONTRACT",
                "evidence": "18-by-18 transition closure, immutable identity/version, monotonic sequence/history, and absorbing terminals",
            },
            {
                "scenario": "malformed-transitions-and-stale-authority",
                "status": "PASS_LOCAL_CONTRACT",
                "evidence": "invalid transitions, cyclic graphs, unsupported versions, and stale request bindings fail closed",
            },
            {
                "scenario": "missing-observation-and-false-completion",
                "status": "PASS_LOCAL_CONTRACT",
                "evidence": "every state attempt requires exactly one observation and success requires current verifier evidence",
            },
            {
                "scenario": "chat-cli-headless-caller-parity",
                "status": "PASS_IN_MEMORY_FIXTURE",
                "evidence": "native Chat, interactive CLI, and workflow/headless caller projections match without acquiring authority",
            },
            {
                "scenario": "replay-cancellation-and-fake-model-verification",
                "status": "PASS_IN_MEMORY_FIXTURE",
                "evidence": "replay and prelaunch cancellation do not duplicate launch; fake-model read requires deterministic verification",
            },
        ],
        "later_runtime_scenarios": [
            {
                "scenario": "hostile-model-proposal-complete-lifecycle",
                "status": "BLOCKED_LATER_STORY",
                "owner": "5.3",
                "reason": "verifier-only workflow execution and hostile proposal campaign are not owned by Story 1.3",
            },
            {
                "scenario": "budget-exhaustion-bounded-termination",
                "status": "BLOCKED_LATER_STORY",
                "owner": "5.3",
                "reason": "integrated workflow budget enforcement and exhaustion campaign remain open",
            },
            {
                "scenario": "crash-and-resume-with-zero-effect-replay",
                "status": "BLOCKED_LATER_STORY",
                "owner": "11.3",
                "reason": "transactional workflow persistence and restart reconciliation remain open",
            },
            {
                "scenario": "one-terminal-observation-per-live-tool-call",
                "status": "BLOCKED_LATER_STORY",
                "owner": "16.4",
                "reason": "complete live tool terminal-state and resource campaign remains open",
            },
            {
                "scenario": "complete-source-to-terminal-fake-model-vertical-slice",
                "status": "BLOCKED_LATER_STORY",
                "owner": "22.5",
                "reason": "new canonical records are not yet composed through one installed source-to-terminal workflow",
            },
        ],
        "acceptance_tests": [
            {"id": "AT-ERT-001", "status": "PARTIAL_LOCAL_CONTRACT", "owner": "1.3"},
            {"id": "AT-WKF-001", "status": "NOT_EXECUTED_OWNER_OPEN", "owner": "5.3"},
            {"id": "AT-WKF-002", "status": "NOT_EXECUTED_OWNER_OPEN", "owner": "11.3"},
            {"id": "AT-WKF-003", "status": "NOT_EXECUTED_OWNER_OPEN", "owner": "11.3"},
            {"id": "AT-RESUME-002", "status": "NOT_EXECUTED_OWNER_OPEN", "owner": "11.3"},
            {"id": "AT-TIO-001", "status": "NOT_EXECUTED_OWNER_OPEN", "owner": "16.4"},
            {"id": "AT-TIO-002", "status": "NOT_EXECUTED_OWNER_OPEN", "owner": "16.4"},
            {"id": "AT-VER-001", "status": "NOT_EXECUTED_OWNER_OPEN", "owner": "5.3"},
        ],
        "platform_runtime_evidence": [
            {
                "tuple": "platform-neutral-dev-tests/deterministic-fake/in-memory-callers",
                "status": "EXECUTED_LOCAL",
                "substitutes_for": [],
            },
            {
                "tuple": "fedora-installed-package/native-chat-cli-headless/native-workers",
                "status": "BLOCKED_EXTERNAL",
                "reason": "no signed installed candidate and root-owned native worker fixture were provided to this story",
                "substitutes_for": [],
            },
            {
                "tuple": "ubuntu-installed-package/native-chat-cli-headless/native-workers",
                "status": "BLOCKED_EXTERNAL",
                "reason": "no Ubuntu installed-candidate runner was available to this story",
                "substitutes_for": [],
            },
            {
                "tuple": "windows-installed-package/native-chat-cli-headless/native-workers",
                "status": "BLOCKED_EXTERNAL",
                "reason": "Windows native runtime implementation and runner remain unavailable",
                "substitutes_for": [],
            },
            {
                "tuple": "macos-installed-package/native-chat-cli-headless/native-workers",
                "status": "BLOCKED_EXTERNAL",
                "reason": "macOS native runtime implementation and runner remain unavailable",
                "substitutes_for": [],
            },
            {
                "tuple": "real-local-model/complete-rv50-lifecycle",
                "status": "BLOCKED_EXTERNAL",
                "reason": "no qualified installed model tuple is active and fake evidence cannot substitute",
                "substitutes_for": [],
            },
        ],
        "artifacts": [
            _artifact("SECURITY-REVIEW.md"),
            _artifact("kernel/engine/src/engineering_records.rs"),
            _artifact("kernel/engine/tests/engineering_runtime_record_corpus.rs"),
            _artifact("shells/host/src/runtime_parity_tests.rs"),
            _artifact("shells/host/src/headless.rs"),
            _artifact("shells/host/src/runtime_read_tests.rs"),
            _artifact("artifacts/sprints/sprint-1/story-1.3/canonical-record-evidence-index.json"),
            _artifact("artifacts/sprints/sprint-1/story-1.3/rv50-applicable-results.log"),
        ],
        "limitations": [
            "The executed caller parity is an in-memory deterministic fixture, not installed native-client evidence.",
            "The executed fake-model read is supporting regression evidence and does not compose every new canonical Story 1.3 record.",
            "No later-story acceptance test, platform tuple, live worker, real model, release gate, or external review is represented as passing.",
        ],
    }


def render(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, ensure_ascii=True) + "\n"


def validate_raw(text: str) -> list[str]:
    failures = [f"RV-50 raw results missing marker: {marker}" for marker in MARKERS if marker not in text]
    for prohibited in ("FAILED (", "test result: FAILED", "not ok ", "Traceback (most recent call last)"):
        if prohibited in text:
            failures.append(f"RV-50 raw results contain failure marker: {prohibited}")
    return failures


def validate_report(value: Any) -> list[str]:
    if value != expected_report():
        return ["RV-50 applicability record is stale, incomplete, reordered, or widened"]
    if value.get("protocol_complete") is not False or value.get("status") == "PASS":
        return ["RV-50 was falsely represented as complete"]
    if any(item.get("substitutes_for") for item in value.get("platform_runtime_evidence", [])):
        return ["RV-50 platform or runtime evidence was substituted"]
    return []


def capture() -> tuple[str, int]:
    chunks: list[str] = []
    for command in COMMANDS:
        chunks.append(f"$ {command}\n")
        result = subprocess.run(
            command.split(),
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
            print("RV-50 applicable evidence build failed", file=sys.stderr)
            return 1
        RAW_PATH.write_text(raw, encoding="utf-8")
        failures = validate_raw(raw)
        if failures:
            for failure in failures:
                print(f"RV-50 applicable evidence build failed: {failure}", file=sys.stderr)
            return 1
        REPORT_PATH.write_text(render(expected_report()), encoding="utf-8")

    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(f"RV-50 applicability validation failed: {error}", file=sys.stderr)
        return 1
    failures = validate_raw(raw) + validate_report(report)
    if failures:
        for failure in failures:
            print(f"RV-50 applicability validation failed: {failure}", file=sys.stderr)
        return 1
    print("Task 1.3.3.2 applicable RV-50 evidence validated with external blockers open")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
