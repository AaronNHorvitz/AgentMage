#!/usr/bin/env python3
"""Build and validate Task 12.1.2.3 runtime-transcript evidence."""

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
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-12/story-12.1/runtime-transcripts.json"
SOURCE_PATHS: Final = (
    "docs/verification/task-12-1-2-3-runtime-transcript-results.md",
    "fixtures/runtime/v1/session-transcripts.json",
    "scripts/runtime_transcript_bundle.py",
    "tests/test_runtime_transcript_bundle.py",
    "scripts/runtime_transcript_evidence.py",
    "tests/test_runtime_transcript_evidence.py",
)
CASE_IDS: Final = tuple(f"TSC-{index:02}" for index in range(1, 6))
DOCUMENT_FRAGMENTS: Final = (
    "Pass for deterministic interruption, cancellation, status, and final-response transcripts.",
    "spanning **12 transcript cases** and all **6 final-response states**",
    "emit no false success",
    "zero private user data",
    "manual fuzzing",
)
SOURCE_FRAGMENTS: Final = {
    "fixtures/runtime/v1/session-transcripts.json": (
        '"fixture_set_id": "agentmage-runtime-transcripts-v1"',
        '"case_id": "TRN-01"',
        '"case_id": "TRN-12"',
        '"contract_authority": "descriptive-only"',
        '"live_model_tool_or_platform_execution": false',
    ),
    "scripts/runtime_transcript_bundle.py": (
        "def transcript_fixture()",
        "def semantic_failures(",
        "def validate_current()",
        "def write_fixture()",
    ),
    "tests/test_runtime_transcript_bundle.py": (
        "test_current_fixture_is_canonical_and_semantically_closed",
        "test_status_and_refusal_mutations_fail",
        "test_interruption_order_and_cancellation_mutations_fail",
        "test_final_response_and_authority_mutations_fail",
    ),
}
COMMAND_SPECS: Final = (
    (
        ("python3", "scripts/runtime_transcript_bundle.py"),
        "pass (12 transcripts, 6 final states)",
    ),
    (
        ("python3", "-m", "unittest", "tests.test_runtime_transcript_bundle"),
        "Ran 5 tests",
    ),
    (
        ("python3", "-m", "unittest", "tests.test_runtime_transcript_evidence"),
        "Ran 4 tests",
    ),
    (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
)
CLAIMS: Final = {
    "focused_case_count": 5,
    "transcript_case_count": 12,
    "status_case_count": 1,
    "accepted_interruption_case_count": 2,
    "refused_interruption_case_count": 3,
    "final_response_state_count": 6,
    "false_success_claim_count": 0,
    "private_user_data_records": 0,
    "model_executions": 0,
    "tool_executions": 0,
    "platform_executions": 0,
    "external_network_used": False,
    "manual_fuzzing_executed": False,
    "release_support": False,
}
LIMITATIONS: Final = [
    "The retained records are deterministic synthetic transcripts, not production session recordings.",
    "Declared descendant propagation does not prove operating-system process termination or cleanup.",
    "No live model, tool, shell, platform adapter, filesystem effect, or network request executes.",
    "The artifact contains no private user data and retains only synthetic identities and content.",
    "Persistence, restart recovery, UI integration, cross-platform execution, packaging, release acceptance, and manual fuzzing remain later gates.",
]
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class EvidenceError(ValueError):
    pass


def validate_sources(values: dict[str, str]) -> list[str]:
    failures = []
    document = values[SOURCE_PATHS[0]]
    if tuple(re.findall(r"`(TSC-\d{2})`", document)) != CASE_IDS:
        failures.append("Task 12.1.2.3 case closure changed")
    failures.extend(
        f"Task 12.1.2.3 document fragment changed: {index}"
        for index, fragment in enumerate(DOCUMENT_FRAGMENTS, 1)
        if document.count(fragment) != 1
    )
    for path, fragments in SOURCE_FRAGMENTS.items():
        failures.extend(
            f"Task 12.1.2.3 source fragment changed: {path}:{index}"
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
        "artifact_id": "task-12-1-2-3-runtime-transcripts",
        "case_ids": list(CASE_IDS),
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "source_revision": revision,
        "sources": source_records(revision),
        "status": "pass-runtime-transcripts",
        "task_ids": ["12.1.2.3"],
        "verification_commands": [
            run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS
        ],
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    exact = {
        "artifact_id": "task-12-1-2-3-runtime-transcripts",
        "case_ids": list(CASE_IDS),
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "status": "pass-runtime-transcripts",
        "task_ids": ["12.1.2.3"],
        "verification_commands": expected_commands(),
    }
    failures = [
        f"Task 12.1.2.3 evidence {key} changed"
        for key, value in exact.items()
        if report.get(key) != value
    ]
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        failures.append("Task 12.1.2.3 evidence revision invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(
        SOURCE_PATHS
    ):
        failures.append("Task 12.1.2.3 evidence sources changed")
    elif any(
        not isinstance(item.get("bytes"), int)
        or item["bytes"] <= 0
        or SHA256.fullmatch(str(item.get("sha256", ""))) is None
        for item in sources
    ):
        failures.append("Task 12.1.2.3 evidence source identity invalid")
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
            failures.append("Task 12.1.2.3 evidence source bytes changed")
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
        print(f"Task 12.1.2.3 runtime transcript evidence: FAIL: {error}")
        return 1
    if failures:
        for failure in failures:
            print(f"Task 12.1.2.3 runtime transcript evidence: FAIL: {failure}")
        return 1
    print("Task 12.1.2.3 runtime transcript evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
