#!/usr/bin/env python3
"""Build and validate S-012-I04 reasoning-verification evidence."""

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
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-12/story-12.1/reasoning-verification.json"
SOURCE_PATHS: Final = (
    "docs/verification/s-012-i04-reasoning-verification-results.md",
    "kernel/contracts/src/reasoning.rs",
    "kernel/contracts/src/lib.rs",
    "kernel/engine/src/reasoning.rs",
    "kernel/engine/src/authority.rs",
    "kernel/engine/src/lib.rs",
    "scripts/reasoning_verification_evidence.py",
    "tests/test_reasoning_verification_evidence.py",
)
CASE_IDS: Final = tuple(f"RVF-{index:02}" for index in range(1, 6))
DOCUMENT_FRAGMENTS: Final = (
    "Pass for bounded deterministic reasoning records",
    "Focused cases closed: **5 of 5**.",
    "result, acceptance checks, and evidence without inheriting first-pass",
    "natural-language semantic contradiction discovery remains later work.",
    "Manual fuzzing remains deferred",
)
SOURCE_FRAGMENTS: Final = {
    "kernel/contracts/src/reasoning.rs": (
        "pub struct ProblemFrame {",
        "pub struct AssumptionRecord {",
        "pub struct HypothesisRecord {",
        "pub struct ContradictionRecord {",
        "pub struct ClarificationQuestion {",
        "pub struct IndependentVerificationRequest {",
        "pub struct IndependentVerificationResult {",
    ),
    "kernel/contracts/src/lib.rs": ("AssumptionRecord, AssumptionRisk, AssumptionStatus",),
    "kernel/engine/src/reasoning.rs": (
        "pub struct ReasoningLedger {",
        "pub fn register_assumption(",
        "pub fn resolve_hypothesis(",
        "pub fn clarification_state(&self) -> ClarificationState",
        "pub fn find_contradictions(",
        "pub fn complete_independent_verification(",
        "!object.contains_key(\"first_pass_conclusions\")",
    ),
    "kernel/engine/src/authority.rs": (
        "ReasoningRecord,",
        "impl_non_authoritative!(ReasoningRecord =>",
    ),
    "kernel/engine/src/lib.rs": ("pub mod reasoning;",),
}
COMMAND_SPECS: Final = (
    (
        (
            "cargo",
            "test",
            "-p",
            "agentmage-kernel-engine",
            "reasoning::tests",
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
        ("python3", "-m", "unittest", "tests.test_reasoning_verification_evidence"),
        "Ran 4 tests",
    ),
    (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
)
CLAIMS: Final = {
    "focused_case_count": 5,
    "compact_problem_frame_implemented": True,
    "assumption_and_hypothesis_lifecycles_implemented": True,
    "exact_key_contradiction_detection_implemented": True,
    "material_clarification_gate_implemented": True,
    "independent_verification_contract_implemented": True,
    "reasoning_authority_admitted_count": 0,
    "private_chain_of_thought_field_count": 0,
    "natural_language_contradiction_detection_implemented": False,
    "persistent_reasoning_implemented": False,
    "exact_completion_claim_proof_implemented": False,
    "tool_or_model_executions": 0,
    "external_network_used": False,
    "manual_fuzzing_executed": False,
    "release_support": False,
}
LIMITATIONS: Final = [
    "Contradiction detection compares exact typed claim keys and canonical values; natural-language semantic contradiction discovery remains later work.",
    "The ledger is in memory only; persistence, restart reconstruction, UI presentation, and session integration remain later work.",
    "Independent verification is structural and uses supplied evidence; no local model, cross-model review, tool, or platform worker runs.",
    "Exact material-claim proof remains Sub-task 12.1.1.5.",
    "Manual fuzzing remains deferred and is not represented by these deterministic tests.",
]
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class EvidenceError(ValueError):
    pass


def validate_sources(values: dict[str, str]) -> list[str]:
    failures = []
    document = values[SOURCE_PATHS[0]]
    if tuple(re.findall(r"`(RVF-\d{2})`", document)) != CASE_IDS:
        failures.append("S-012-I04 case closure changed")
    failures.extend(
        f"S-012-I04 document fragment changed: {index}"
        for index, fragment in enumerate(DOCUMENT_FRAGMENTS, 1)
        if document.count(fragment) != 1
    )
    for path, fragments in SOURCE_FRAGMENTS.items():
        failures.extend(
            f"S-012-I04 source fragment changed: {path}:{index}"
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
        "artifact_id": "s-012-i04-reasoning-verification",
        "case_ids": list(CASE_IDS),
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "source_revision": revision,
        "sources": source_records(revision),
        "status": "pass-bounded-deterministic-reasoning-records",
        "task_ids": ["12.1.1.4", "S-012-I04"],
        "verification_commands": [
            run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS
        ],
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    exact = {
        "artifact_id": "s-012-i04-reasoning-verification",
        "case_ids": list(CASE_IDS),
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "status": "pass-bounded-deterministic-reasoning-records",
        "task_ids": ["12.1.1.4", "S-012-I04"],
        "verification_commands": expected_commands(),
    }
    failures = [
        f"S-012-I04 evidence {key} changed"
        for key, value in exact.items()
        if report.get(key) != value
    ]
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        failures.append("S-012-I04 evidence revision invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(
        SOURCE_PATHS
    ):
        failures.append("S-012-I04 evidence sources changed")
    elif any(
        not isinstance(item.get("bytes"), int)
        or item["bytes"] <= 0
        or SHA256.fullmatch(str(item.get("sha256", ""))) is None
        for item in sources
    ):
        failures.append("S-012-I04 evidence source records invalid")
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
    print("S-012-I04 reasoning verification evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
