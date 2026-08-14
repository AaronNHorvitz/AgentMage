#!/usr/bin/env python3
"""Build and validate Task 12.1.2.2 runtime-fixture evidence."""

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
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-12/story-12.1/runtime-fixtures.json"
SOURCE_PATHS: Final = (
    "docs/verification/task-12-1-2-2-runtime-fixture-results.md",
    "fixtures/runtime/v1/planning-fixtures.json",
    "fixtures/runtime/v1/reasoning-mode-fixtures.json",
    "fixtures/runtime/v1/completion-evidence-fixtures.json",
    "scripts/runtime_fixture_bundle.py",
    "tests/test_runtime_fixture_bundle.py",
    "scripts/runtime_fixture_evidence.py",
    "tests/test_runtime_fixture_evidence.py",
)
CASE_IDS: Final = tuple(f"FBC-{index:02}" for index in range(1, 6))
DOCUMENT_FRAGMENTS: Final = (
    "Pass for planning, reasoning-mode, and completion-evidence fixtures",
    "Focused cases closed: **5 of 5**, spanning **11 fixture cases**.",
    "never authority or evidence standards",
    "zero private user data",
    "manual fuzzing",
)
SOURCE_FRAGMENTS: Final = {
    "fixtures/runtime/v1/planning-fixtures.json": (
        '"fixture_set_id": "agentmage-runtime-planning-v1"',
        '"case_id": "PLN-04"',
        '"code": "competing-running-step"',
        '"code": "dependency-incomplete"',
    ),
    "fixtures/runtime/v1/reasoning-mode-fixtures.json": (
        '"fixture_set_id": "agentmage-reasoning-modes-v1"',
        '"mode": "concise"',
        '"mode": "deep"',
        '"mode_changes_authority": false',
        '"mode_changes_evidence_standard": false',
    ),
    "fixtures/runtime/v1/completion-evidence-fixtures.json": (
        '"case_id": "CMP-05"',
        '"code": "complete"',
        '"code": "missing-evidence"',
        '"code": "stale-evidence"',
        '"code": "unverified-prerequisite"',
    ),
    "scripts/runtime_fixture_bundle.py": (
        "def planning_fixtures()",
        "def reasoning_fixtures()",
        "def completion_fixtures()",
        "def semantic_failures(",
        "def validate_current()",
    ),
    "tests/test_runtime_fixture_bundle.py": (
        "test_current_files_are_canonical_and_semantically_closed",
        "test_generation_is_deterministic",
        "test_planning_mutations_fail",
        "test_reasoning_authority_evidence_and_private_thought_mutations_fail",
        "test_completion_case_and_disposition_mutations_fail",
    ),
}
COMMAND_SPECS: Final = (
    (
        ("python3", "scripts/runtime_fixture_bundle.py"),
        "pass (4 planning, 2 reasoning, 5 completion cases)",
    ),
    (
        ("python3", "-m", "unittest", "tests.test_runtime_fixture_bundle"),
        "Ran 5 tests",
    ),
    (
        ("python3", "-m", "unittest", "tests.test_runtime_fixture_evidence"),
        "Ran 4 tests",
    ),
    (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
)
CLAIMS: Final = {
    "focused_case_count": 5,
    "fixture_family_count": 3,
    "fixture_case_count": 11,
    "planning_case_count": 4,
    "reasoning_mode_case_count": 2,
    "completion_case_count": 5,
    "successful_completion_case_count": 1,
    "reasoning_mode_authority_changes": 0,
    "reasoning_mode_evidence_standard_changes": 0,
    "private_user_data_records": 0,
    "model_executions": 0,
    "tool_executions": 0,
    "platform_executions": 0,
    "external_network_used": False,
    "manual_fuzzing_executed": False,
    "release_support": False,
}
LIMITATIONS: Final = [
    "Fixtures are deterministic synthetic contract inputs and expected dispositions, not production model, tool, or platform runs.",
    "Reasoning modes are review profiles for later composition and add no mode router or model implementation.",
    "Completion references are synthetic and claim no live file, command, Git remote, publication, or provider effect.",
    "Fixture validation is not persisted replay, crash recovery, or UI transcript evidence.",
    "Cross-platform execution, packaging, release acceptance, and manual fuzzing remain later gates.",
]
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class EvidenceError(ValueError):
    pass


def validate_sources(values: dict[str, str]) -> list[str]:
    failures = []
    document = values[SOURCE_PATHS[0]]
    if tuple(re.findall(r"`(FBC-\d{2})`", document)) != CASE_IDS:
        failures.append("Task 12.1.2.2 case closure changed")
    failures.extend(
        f"Task 12.1.2.2 document fragment changed: {index}"
        for index, fragment in enumerate(DOCUMENT_FRAGMENTS, 1)
        if document.count(fragment) != 1
    )
    for path, fragments in SOURCE_FRAGMENTS.items():
        failures.extend(
            f"Task 12.1.2.2 source fragment changed: {path}:{index}"
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
        "artifact_id": "task-12-1-2-2-runtime-fixtures",
        "case_ids": list(CASE_IDS),
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "source_revision": revision,
        "sources": source_records(revision),
        "status": "pass-runtime-fixture-bundle",
        "task_ids": ["12.1.2.2"],
        "verification_commands": [
            run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS
        ],
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    exact = {
        "artifact_id": "task-12-1-2-2-runtime-fixtures",
        "case_ids": list(CASE_IDS),
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "status": "pass-runtime-fixture-bundle",
        "task_ids": ["12.1.2.2"],
        "verification_commands": expected_commands(),
    }
    failures = [
        f"Task 12.1.2.2 evidence {key} changed"
        for key, value in exact.items()
        if report.get(key) != value
    ]
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        failures.append("Task 12.1.2.2 evidence revision invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(
        SOURCE_PATHS
    ):
        failures.append("Task 12.1.2.2 evidence sources changed")
    elif any(
        not isinstance(item.get("bytes"), int)
        or item["bytes"] <= 0
        or SHA256.fullmatch(str(item.get("sha256", ""))) is None
        for item in sources
    ):
        failures.append("Task 12.1.2.2 evidence source records invalid")
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
    args = parser.parse_args()
    try:
        if args.write:
            revision = git_revision(args.source_revision)
            atomic_write(REPORT_PATH, canonical_json_bytes(build_report(revision)))
        report = json.loads(REPORT_PATH.read_text())
        if failures := validate_report(report):
            raise EvidenceError("; ".join(failures))
        validate_committed(report)
    except (EvidenceError, OSError, json.JSONDecodeError) as error:
        print(f"Task 12.1.2.2 runtime fixture evidence: FAIL: {error}")
        return 1
    print("Task 12.1.2.2 runtime fixture evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
