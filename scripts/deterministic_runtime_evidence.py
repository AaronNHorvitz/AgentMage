#!/usr/bin/env python3
"""Build and validate S-012-UT01 deterministic runtime evidence."""

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
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-12/story-12.1/deterministic-runtime.json"
SOURCE_PATHS: Final = (
    "docs/verification/s-012-ut01-deterministic-runtime-results.md",
    "kernel/engine/src/agent_runtime.rs",
    "scripts/deterministic_runtime_evidence.py",
    "tests/test_deterministic_runtime_evidence.py",
)
CASE_IDS: Final = tuple(f"DVR-{index:02}" for index in range(1, 6))
DOCUMENT_FRAGMENTS: Final = (
    "Pass for deterministic state, objective, revision, budget, stop, and completion boundaries.",
    "Exercise 20 phase/method pairs",
    "4,097-byte objectives",
    "Actions remain descriptive proposals",
    "manual fuzzing",
)
SOURCE_FRAGMENTS: Final = {
    "kernel/engine/src/agent_runtime.rs": (
        "s_012_ut01_covers_every_legal_and_illegal_loop_transition",
        "s_012_ut01_rejects_empty_and_oversized_objectives_deterministically",
        "s_012_ut01_plan_revisions_advance_once_and_fail_without_mutation",
        "s_012_ut01_budget_boundary_is_inclusive_and_overage_is_sticky",
        "s_012_ut01_stop_conditions_and_completion_claims_are_typed",
    ),
}
COMMAND_SPECS: Final = (
    (
        ("cargo", "test", "-p", "agentmage-kernel-engine", "s_012_ut01", "--locked"),
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
        ("python3", "-m", "unittest", "tests.test_deterministic_runtime_evidence"),
        "Ran 4 tests",
    ),
    (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
)
CLAIMS: Final = {
    "focused_case_count": 5,
    "rust_test_count": 5,
    "loop_phase_count": 5,
    "phase_method_pair_count": 20,
    "legal_phase_method_edge_count": 4,
    "illegal_phase_method_pair_count": 16,
    "legal_stop_edge_count": 4,
    "objective_boundary_case_count": 2,
    "legal_plan_revision_transition_count": 2,
    "illegal_plan_revision_transition_count": 1,
    "budget_boundary_case_count": 2,
    "signaled_stop_condition_count": 5,
    "refused_stop_condition_count": 3,
    "completion_evidence_case_count": 3,
    "model_executions": 0,
    "tool_executions": 0,
    "platform_executions": 0,
    "private_user_data_records": 0,
    "external_network_used": False,
    "manual_fuzzing_executed": False,
    "release_support": False,
}
LIMITATIONS: Final = [
    "The suite exercises deterministic in-process kernel state and accounting only.",
    "Actions remain descriptive proposals and no model, tool, grant, shell worker, platform adapter, or filesystem effect executes.",
    "The matrix does not prove operating-system cancellation, persisted restart recovery, or UI behavior.",
    "Fixtures are synthetic and contain no private user data or external network use.",
    "Cross-platform execution, packaging, release acceptance, and manual fuzzing remain later gates.",
]
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class EvidenceError(ValueError):
    pass


def validate_sources(values: dict[str, str]) -> list[str]:
    failures = []
    document = values[SOURCE_PATHS[0]]
    if tuple(re.findall(r"`(DVR-\d{2})`", document)) != CASE_IDS:
        failures.append("S-012-UT01 case closure changed")
    failures.extend(
        f"S-012-UT01 document fragment changed: {index}"
        for index, fragment in enumerate(DOCUMENT_FRAGMENTS, 1)
        if document.count(fragment) != 1
    )
    for path, fragments in SOURCE_FRAGMENTS.items():
        failures.extend(
            f"S-012-UT01 source fragment changed: {path}:{index}"
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
        "artifact_id": "s-012-ut01-deterministic-runtime",
        "case_ids": list(CASE_IDS),
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "source_revision": revision,
        "sources": source_records(revision),
        "status": "pass-deterministic-runtime-boundaries",
        "task_ids": ["12.1.3.1", "S-012-UT01"],
        "verification_commands": [
            run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS
        ],
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    exact = {
        "artifact_id": "s-012-ut01-deterministic-runtime",
        "case_ids": list(CASE_IDS),
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "status": "pass-deterministic-runtime-boundaries",
        "task_ids": ["12.1.3.1", "S-012-UT01"],
        "verification_commands": expected_commands(),
    }
    failures = [
        f"S-012-UT01 evidence {key} changed"
        for key, value in exact.items()
        if report.get(key) != value
    ]
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        failures.append("S-012-UT01 evidence revision invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(
        SOURCE_PATHS
    ):
        failures.append("S-012-UT01 evidence sources changed")
    elif any(
        not isinstance(item.get("bytes"), int)
        or item["bytes"] <= 0
        or SHA256.fullmatch(str(item.get("sha256", ""))) is None
        for item in sources
    ):
        failures.append("S-012-UT01 evidence source identity invalid")
    return failures


def validate_current(report: dict[str, Any]) -> list[str]:
    failures = validate_report(report)
    revision = report.get("source_revision")
    if REVISION.fullmatch(str(revision or "")) is None:
        return failures
    try:
        expected_sources = source_records(str(revision))
    except EvidenceError as error:
        failures.append(str(error))
    else:
        if report.get("sources") != expected_sources:
            failures.append("S-012-UT01 evidence source bytes changed")
    return failures


def load_report() -> dict[str, Any]:
    try:
        value = json.loads(REPORT_PATH.read_text())
    except (OSError, json.JSONDecodeError) as error:
        raise EvidenceError(f"evidence report unavailable: {error}") from error
    if not isinstance(value, dict):
        raise EvidenceError("evidence report root invalid")
    return value


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    args = parser.parse_args()
    try:
        if args.write:
            revision = git_revision(args.source_revision)
            atomic_write(REPORT_PATH, canonical_json_bytes(build_report(revision)))
        failures = validate_current(load_report())
    except EvidenceError as error:
        print(f"S-012-UT01 deterministic runtime evidence: FAIL: {error}")
        return 1
    if failures:
        for failure in failures:
            print(f"S-012-UT01 deterministic runtime evidence: FAIL: {failure}")
        return 1
    print("S-012-UT01 deterministic runtime evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
