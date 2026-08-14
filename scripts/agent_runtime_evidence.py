#!/usr/bin/env python3
"""Build and validate S-012-I01 bounded-agent-runtime evidence."""

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
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-12/story-12.1/agent-runtime.json"
SOURCE_PATHS: Final = (
    "docs/verification/s-012-i01-agent-runtime-results.md",
    "kernel/engine/src/agent_runtime.rs",
    "kernel/engine/src/configuration.rs",
    "kernel/engine/src/run_control.rs",
    "kernel/engine/src/work_packet.rs",
    "kernel/engine/src/lib.rs",
    "scripts/agent_runtime_evidence.py",
    "tests/test_agent_runtime_evidence.py",
)
CASE_IDS: Final = tuple(f"AR-{index:02}" for index in range(1, 9))
DOCUMENT_FRAGMENTS: Final = (
    "Pass for the current in-process, non-executing kernel boundary",
    "Focused cases closed: **8 of 8**.",
    "Tool executions: **0**.",
    "Model invocations: **0**.",
    "External network attempts: **0**.",
    "Private user data used: **0**.",
    "Manual fuzzing remains deferred",
)
SOURCE_FRAGMENTS: Final = {
    "kernel/engine/src/agent_runtime.rs": (
        "pub struct AgentRuntime {",
        "pub enum AgentLoopPhase {",
        "pub enum AgentReviewOutcome {",
        "pub fn observe(",
        "pub fn plan(&mut self)",
        "pub fn act(&mut self, action: &Action)",
        "pub fn review(",
        "pub fn accept_completion(",
        "if action.kind == ActionKind::ModelInference && !self.configuration.model_enabled()",
        "A successful return does not execute or authorize the action.",
    ),
    "kernel/engine/src/configuration.rs": (
        "pub const fn model_enabled(&self) -> bool",
    ),
    "kernel/engine/src/run_control.rs": (
        "pub struct RunController {",
        "pub fn consume(&mut self, resource: BudgetResource, amount: u64)",
        "pub fn accept_completion(",
    ),
    "kernel/engine/src/work_packet.rs": (
        "pub fn adapt_packet_to_plan(",
        "pub fn validate_completion(packet: &WorkPacket)",
    ),
    "kernel/engine/src/lib.rs": ("pub mod agent_runtime;",),
}
COMMAND_SPECS: Final = (
    (("cargo", "test", "-p", "agentmage-kernel-engine", "agent_runtime::tests", "--locked"), "8 passed; 0 failed"),
    (("cargo", "clippy", "-p", "agentmage-kernel-engine", "--all-targets", "--locked", "--", "-D", "warnings"), "Finished `dev` profile"),
    (("python3", "-m", "unittest", "tests.test_agent_runtime_evidence"), "Ran 4 tests"),
    (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
)
CLAIMS: Final = {
    "focused_case_count": 8,
    "loop_phase_count": 5,
    "metered_resource_count": 6,
    "typed_review_outcome_count": 6,
    "tool_executions": 0,
    "model_invocations": 0,
    "model_enabled_in_fixture": False,
    "completion_requires_current_evidence": True,
    "action_proposal_grants_authority": False,
    "persistent_runtime_state_implemented": False,
    "external_network_used": False,
    "manual_fuzzing_executed": False,
    "release_support": False,
}
LIMITATIONS: Final = [
    "This is an in-process deterministic runtime boundary; persisted terminal-state and restart reconciliation remain Story 12.2 work.",
    "Plan history, one-active-step transitions, progress/status events, new-user-message handling, and cancellation transcripts remain later Story 12.1 tasks.",
    "No production tool, model, worker, interface, platform, package, or release workflow is exercised or claimed.",
    "Manual fuzzing remains deferred and is not represented by the deterministic tests.",
]
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class EvidenceError(ValueError):
    pass


def validate_sources(values: dict[str, str]) -> list[str]:
    failures = []
    document = values[SOURCE_PATHS[0]]
    if tuple(re.findall(r"`(AR-\d{2})`", document)) != CASE_IDS:
        failures.append("S-012-I01 case closure changed")
    failures.extend(
        f"S-012-I01 document fragment changed: {index}"
        for index, fragment in enumerate(DOCUMENT_FRAGMENTS, 1)
        if document.count(fragment) != 1
    )
    for path, fragments in SOURCE_FRAGMENTS.items():
        failures.extend(
            f"S-012-I01 source fragment changed: {path}:{index}"
            for index, fragment in enumerate(fragments, 1)
            if values[path].count(fragment) != 1
        )
    return failures


def git_revision(candidate: str) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"], cwd=ROOT,
        capture_output=True, text=True, timeout=30, check=False,
    )
    revision = result.stdout.strip()
    if result.returncode or REVISION.fullmatch(revision) is None:
        raise EvidenceError("source revision unavailable")
    return revision


def git_bytes(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL, timeout=60, check=False,
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
        list(arguments), cwd=ROOT, capture_output=True, text=True, timeout=300, check=False,
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
        records.append({"bytes": len(data), "path": path, "sha256": hashlib.sha256(data).hexdigest()})
    if failures := validate_sources(values):
        raise EvidenceError("; ".join(failures))
    return records


def build_report(revision: str) -> dict[str, Any]:
    return {
        "artifact_id": "s-012-i01-bounded-agent-runtime",
        "case_ids": list(CASE_IDS),
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "source_revision": revision,
        "sources": source_records(revision),
        "status": "pass-current-in-process-non-executing-boundary",
        "task_ids": ["12.1.1.1", "S-012-I01"],
        "verification_commands": [run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS],
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    exact = {
        "artifact_id": "s-012-i01-bounded-agent-runtime",
        "case_ids": list(CASE_IDS),
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "status": "pass-current-in-process-non-executing-boundary",
        "task_ids": ["12.1.1.1", "S-012-I01"],
        "verification_commands": expected_commands(),
    }
    failures = [f"S-012-I01 evidence {key} changed" for key, value in exact.items() if report.get(key) != value]
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        failures.append("S-012-I01 evidence revision invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(SOURCE_PATHS):
        failures.append("S-012-I01 evidence sources changed")
    elif any(not isinstance(item.get("bytes"), int) or item["bytes"] <= 0 or SHA256.fullmatch(str(item.get("sha256", ""))) is None for item in sources):
        failures.append("S-012-I01 evidence source records invalid")
    return failures


def validate_committed(report: dict[str, Any]) -> None:
    for item in report["sources"]:
        if hashlib.sha256(git_bytes(report["source_revision"], item["path"])).hexdigest() != item["sha256"]:
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
    print("S-012-I01 agent runtime evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
