#!/usr/bin/env python3
"""Build and validate S-012-I03 agent-progress evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
from pathlib import Path
from typing import Any, Final

try:
    from scripts.evidence_core import atomic_write, canonical_json_bytes
except ModuleNotFoundError:
    from evidence_core import atomic_write, canonical_json_bytes

ROOT: Final = Path(__file__).resolve().parents[1]
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-12/story-12.1/agent-progress.json"
SOURCE_PATHS: Final = (
    "docs/verification/s-012-i03-agent-progress-results.md",
    "kernel/contracts/src/agent.rs",
    "kernel/contracts/src/lib.rs",
    "kernel/engine/src/agent_progress.rs",
    "kernel/engine/src/propagation.rs",
    "kernel/engine/src/lib.rs",
    "scripts/agent_progress_evidence.py",
    "tests/test_agent_progress_evidence.py",
)
CASE_IDS: Final = tuple(f"AP-{index:02}" for index in range(1, 6))
DOCUMENT_FRAGMENTS: Final = (
    "Pass for bounded in-memory progress behavior",
    "Focused cases closed: **5 of 5**.",
    "terminal-plan interruption fail without partial token cancellation.",
    "proof remains Sub-task `12.1.1.5`.",
    "Manual fuzzing remains deferred",
)
SOURCE_FRAGMENTS: Final = {
    "kernel/contracts/src/agent.rs": (
        "pub enum AgentProgressKind {",
        "pub struct AgentProgressEvent {",
        "pub struct AgentStatusResponse {",
        "pub enum UserMessageIntent {",
        "pub enum AgentFinalState {",
        "pub struct AgentFinalResponse {",
    ),
    "kernel/contracts/src/lib.rs": ("AgentFinalResponse, AgentFinalState, AgentProgressEvent",),
    "kernel/engine/src/agent_progress.rs": (
        "pub struct PlanProgressController {",
        "pub fn transition_step(",
        "pub fn handle_user_message(",
        "pub fn status(&self) -> AgentStatusResponse",
        "pub fn build_final_response(",
        "PlanState::Proposed | PlanState::Current | PlanState::InProgress",
        "evidence.is_empty() || !unresolved.is_empty()",
    ),
    "kernel/engine/src/propagation.rs": ("pub struct CancellationToken {",),
    "kernel/engine/src/lib.rs": ("pub mod agent_progress;",),
}
COMMAND_SPECS: Final = (
    (
        (
            "cargo",
            "test",
            "-p",
            "agentmage-kernel-engine",
            "agent_progress::tests",
            "--locked",
        ),
        "5 passed; 0 failed",
    ),
    (
        (
            "cargo",
            "clippy",
            "-p",
            "agentmage-kernel-engine",
            "--all-targets",
            "--locked",
            "--",
            "-D",
            "warnings",
        ),
        "Finished `dev` profile",
    ),
    (
        ("python3", "-m", "unittest", "tests.test_agent_progress_evidence"),
        "Ran 4 tests",
    ),
    (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
)
CLAIMS: Final = {
    "focused_case_count": 5,
    "one_active_step_enforced": True,
    "append_only_in_memory_plan_history": True,
    "content_free_progress_and_status": True,
    "typed_descendant_cancellation": True,
    "completion_requires_evidence": True,
    "completion_rejects_unresolved_items": True,
    "persistent_history_implemented": False,
    "exact_material_claim_proof_implemented": False,
    "tool_or_model_executions": 0,
    "external_network_used": False,
    "manual_fuzzing_executed": False,
    "release_support": False,
}
LIMITATIONS: Final = [
    "Plan history and events are in memory only; persistence, restart reconstruction, and terminal-result reconciliation remain later work.",
    "Completion evidence validation is structural; exact material-claim proof remains Sub-task 12.1.1.5.",
    "No production session host, UI transcript, tool, model, worker, platform, package, release, or cross-platform workflow is exercised or claimed.",
    "Manual fuzzing remains deferred and is not represented by these deterministic tests.",
]
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class EvidenceError(ValueError):
    pass


def validate_sources(values: dict[str, str]) -> list[str]:
    failures = []
    document = values[SOURCE_PATHS[0]]
    if tuple(re.findall(r"`(AP-\d{2})`", document)) != CASE_IDS:
        failures.append("S-012-I03 case closure changed")
    failures.extend(
        f"S-012-I03 document fragment changed: {index}"
        for index, fragment in enumerate(DOCUMENT_FRAGMENTS, 1)
        if document.count(fragment) != 1
    )
    for path, fragments in SOURCE_FRAGMENTS.items():
        failures.extend(
            f"S-012-I03 source fragment changed: {path}:{index}"
            for index, fragment in enumerate(fragments, 1)
            if values[path].count(fragment) != 1
        )
    return failures


def git_revision(candidate: str) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=30,
        check=False,
    )
    revision = result.stdout.strip()
    if result.returncode or REVISION.fullmatch(revision) is None:
        raise EvidenceError("source revision unavailable")
    return revision


def git_bytes(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=60,
        check=False,
    )
    if result.returncode or not result.stdout:
        raise EvidenceError(f"committed source unavailable: {path}")
    return result.stdout


def command_record(arguments: tuple[str, ...], marker: str) -> dict[str, Any]:
    return {
        "command_id": hashlib.sha256("\0".join(arguments).encode()).hexdigest(),
        "expected_marker_sha256": hashlib.sha256(marker.encode()).hexdigest(),
        "exit_code": 0,
        "status": "pass",
    }


def expected_commands() -> list[dict[str, Any]]:
    return [command_record(arguments, marker) for arguments, marker in COMMAND_SPECS]


def run_checked(arguments: tuple[str, ...], marker: str) -> dict[str, Any]:
    result = subprocess.run(
        list(arguments),
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=300,
        check=False,
        env={**os.environ, "LANG": "C", "LC_ALL": "C"},
    )
    if result.returncode or marker not in result.stdout + result.stderr:
        raise EvidenceError(f"verification failed: {arguments[0]}")
    return command_record(arguments, marker)


def source_records(revision: str) -> list[dict[str, Any]]:
    values = {}
    records = []
    for path in SOURCE_PATHS:
        data = git_bytes(revision, path)
        values[path] = data.decode()
        records.append(
            {"bytes": len(data), "path": path, "sha256": hashlib.sha256(data).hexdigest()}
        )
    if failures := validate_sources(values):
        raise EvidenceError("; ".join(failures))
    return records


def build_report(revision: str) -> dict[str, Any]:
    return {
        "artifact_id": "s-012-i03-agent-progress",
        "case_ids": list(CASE_IDS),
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "source_revision": revision,
        "sources": source_records(revision),
        "status": "pass-bounded-in-memory-progress",
        "task_ids": ["12.1.1.3", "S-012-I03"],
        "verification_commands": [
            run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS
        ],
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    exact = {
        "artifact_id": "s-012-i03-agent-progress",
        "case_ids": list(CASE_IDS),
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "status": "pass-bounded-in-memory-progress",
        "task_ids": ["12.1.1.3", "S-012-I03"],
        "verification_commands": expected_commands(),
    }
    failures = [
        f"S-012-I03 evidence {key} changed"
        for key, value in exact.items()
        if report.get(key) != value
    ]
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        failures.append("S-012-I03 evidence revision invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(
        SOURCE_PATHS
    ):
        failures.append("S-012-I03 evidence sources changed")
    elif any(
        not isinstance(item.get("bytes"), int)
        or item["bytes"] <= 0
        or SHA256.fullmatch(str(item.get("sha256", ""))) is None
        for item in sources
    ):
        failures.append("S-012-I03 evidence source records invalid")
    return failures


def validate_committed(report: dict[str, Any]) -> None:
    for item in report["sources"]:
        digest = hashlib.sha256(git_bytes(report["source_revision"], item["path"])).hexdigest()
        if digest != item["sha256"]:
            raise EvidenceError(f"committed source binding changed: {item['path']}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    revision = git_revision(arguments.source_revision)
    report = build_report(revision) if arguments.write else json.loads(REPORT_PATH.read_text())
    if arguments.write:
        atomic_write(REPORT_PATH, canonical_json_bytes(report))
    if failures := validate_report(report):
        raise EvidenceError("; ".join(failures))
    validate_committed(report)
    print("S-012-I03 agent progress evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
