#!/usr/bin/env python3
"""Build and validate S-012-UT02 hostile-model-candidate evidence."""

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
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-12/story-12.1/hostile-model-candidates.json"
SOURCE_PATHS: Final = (
    "docs/verification/s-012-ut02-hostile-model-candidate-results.md",
    "kernel/engine/src/s012_ut02.rs",
    "kernel/engine/src/lib.rs",
    "scripts/hostile_model_candidate_evidence.py",
    "tests/test_hostile_model_candidate_evidence.py",
)
CASE_IDS: Final = tuple(f"HMC-{index:02}" for index in range(1, 6))
DOCUMENT_FRAGMENTS: Final = (
    "Pass for false, malformed, contradictory, and authority-seeking model plan and tool-call candidates.",
    "exact two-attempt ceiling",
    "Every tool receipt reports `NotChanged`",
    "test-only verification logic",
    "manual fuzzing",
)
SOURCE_FRAGMENTS: Final = {
    "kernel/engine/src/s012_ut02.rs": (
        "const REPAIR_ATTEMPT_LIMIT: usize = 2;",
        "s_012_ut02_false_malformed_and_contradictory_plans_are_inert",
        "s_012_ut02_plan_repair_is_bounded_and_exhaustion_becomes_unknown",
        "s_012_ut02_exact_contradictions_produce_an_explicit_unknown_result",
        "s_012_ut02_authority_seeking_plan_and_call_cannot_authorize_execution",
        "s_012_ut02_tool_repair_is_bounded_and_corrected_calls_still_safe_stop",
    ),
    "kernel/engine/src/lib.rs": ("mod s012_ut02;",),
}
COMMAND_SPECS: Final = (
    (
        ("cargo", "test", "-p", "agentmage-kernel-engine", "s_012_ut02", "--locked"),
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
        ("python3", "-m", "unittest", "tests.test_hostile_model_candidate_evidence"),
        "Ran 4 tests",
    ),
    (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
)
CLAIMS: Final = {
    "focused_case_count": 5,
    "rust_test_count": 5,
    "repair_attempt_limit": 2,
    "invalid_plan_class_count": 3,
    "explicit_unknown_result_count": 2,
    "exact_contradiction_count": 1,
    "authority_seeking_candidate_count": 2,
    "tool_candidate_class_count": 5,
    "tool_executions": 0,
    "model_executions": 0,
    "platform_executions": 0,
    "runtime_repair_implementation_added": 0,
    "private_user_data_records": 0,
    "external_network_used": False,
    "manual_fuzzing_executed": False,
    "release_support": False,
}
LIMITATIONS: Final = [
    "The bounded repair controller is test-only verification logic; production codecs, streaming validation, and retry orchestration remain Sprint 13 work.",
    "No real model, model runtime, tool executor, grant issuer, worker, platform adapter, filesystem effect, or network request executes.",
    "Exact claim-key contradiction detection does not provide natural-language contradiction inference.",
    "The dispatcher proves only a pre-grant safe stop; later grant-mediated execution remains outside this artifact.",
    "Cross-platform execution, persistence, restart recovery, packaging, release acceptance, and manual fuzzing remain later gates.",
]
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class EvidenceError(ValueError):
    pass


def validate_sources(values: dict[str, str]) -> list[str]:
    failures = []
    document = values[SOURCE_PATHS[0]]
    if tuple(re.findall(r"`(HMC-\d{2})`", document)) != CASE_IDS:
        failures.append("S-012-UT02 case closure changed")
    failures.extend(
        f"S-012-UT02 document fragment changed: {index}"
        for index, fragment in enumerate(DOCUMENT_FRAGMENTS, 1)
        if document.count(fragment) != 1
    )
    for path, fragments in SOURCE_FRAGMENTS.items():
        failures.extend(
            f"S-012-UT02 source fragment changed: {path}:{index}"
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
        "artifact_id": "s-012-ut02-hostile-model-candidates",
        "case_ids": list(CASE_IDS),
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "source_revision": revision,
        "sources": source_records(revision),
        "status": "pass-hostile-model-candidate-boundaries",
        "task_ids": ["12.1.3.2", "S-012-UT02"],
        "verification_commands": [
            run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS
        ],
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    exact = {
        "artifact_id": "s-012-ut02-hostile-model-candidates",
        "case_ids": list(CASE_IDS),
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "status": "pass-hostile-model-candidate-boundaries",
        "task_ids": ["12.1.3.2", "S-012-UT02"],
        "verification_commands": expected_commands(),
    }
    failures = [
        f"S-012-UT02 evidence {key} changed"
        for key, value in exact.items()
        if report.get(key) != value
    ]
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        failures.append("S-012-UT02 evidence revision invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(
        SOURCE_PATHS
    ):
        failures.append("S-012-UT02 evidence sources changed")
    elif any(
        not isinstance(item.get("bytes"), int)
        or item["bytes"] <= 0
        or SHA256.fullmatch(str(item.get("sha256", ""))) is None
        for item in sources
    ):
        failures.append("S-012-UT02 evidence source identity invalid")
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
            failures.append("S-012-UT02 evidence source bytes changed")
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
        print(f"S-012-UT02 hostile model candidate evidence: FAIL: {error}")
        return 1
    if failures:
        for failure in failures:
            print(f"S-012-UT02 hostile model candidate evidence: FAIL: {failure}")
        return 1
    print("S-012-UT02 hostile model candidate evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
