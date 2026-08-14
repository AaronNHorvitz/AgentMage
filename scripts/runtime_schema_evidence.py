#!/usr/bin/env python3
"""Build and validate Task 12.1.2.1 runtime-schema evidence."""

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
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-12/story-12.1/runtime-schemas.json"
SOURCE_PATHS: Final = (
    "docs/verification/task-12-1-2-1-runtime-schema-results.md",
    "schemas/runtime/single-agent-state-machine.schema.json",
    "schemas/runtime/agent-progress-event.schema.json",
    "schemas/runtime/examples/single-agent-state-machine.valid.json",
    "schemas/runtime/examples/agent-progress-event.valid.json",
    "scripts/validate_planning_schemas.mjs",
    "tests/test_planning_schemas.mjs",
    "scripts/runtime_schema_evidence.py",
    "tests/test_runtime_schema_evidence.py",
)
CASE_IDS: Final = tuple(f"RSC-{index:02}" for index in range(1, 6))
DOCUMENT_FRAGMENTS: Final = (
    "Pass for single-agent state-machine and progress-event schemas",
    "Focused cases closed: **5 of 5**.",
    "Five ordered phases, eight ordered legal edges",
    "do not add a second runtime implementation",
    "manual fuzzing",
)
SOURCE_FRAGMENTS: Final = {
    "schemas/runtime/single-agent-state-machine.schema.json": (
        '"record_type": {',
        '"const": "single-agent-state-machine"',
        '"const": "agentmage-single-agent-loop-v1"',
        '"const": "descriptive-only"',
        '"minItems": 8',
        '"maxItems": 8',
    ),
    "schemas/runtime/agent-progress-event.schema.json": (
        '"sequence": {',
        '"plan_revision": {',
        '"plan_step_id": {\n      "oneOf"',
        '"const": "plan_current"',
        '"enum": [\n              "step_ready",\n              "step_started"',
        '"additionalProperties": false',
    ),
    "scripts/validate_planning_schemas.mjs": (
        "export const RUNTIME_RECORD_TYPES",
        "export function createRuntimeValidators()",
        "export function validateRuntimeRecord(",
        "export function validateRuntimeFixtures()",
    ),
    "tests/test_planning_schemas.mjs": (
        "runtime state event and environment fixtures satisfy closed schemas",
        "runtime schemas reject missing and unknown fields",
        "state-machine schema rejects phase edge and authority drift",
        "progress events enforce sequence revision step identity and content-free shape",
        "unknown runtime record types fail explicitly",
    ),
}
COMMAND_SPECS: Final = (
    (("npm", "run", "schemas:validate"), "3 runtime record(s)."),
    (
        ("node", "--test", "tests/test_planning_schemas.mjs"),
        "unknown runtime record types fail explicitly",
    ),
    (
        ("python3", "-m", "unittest", "tests.test_runtime_schema_evidence"),
        "Ran 4 tests",
    ),
    (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
)
CLAIMS: Final = {
    "focused_case_count": 5,
    "runtime_schema_count": 2,
    "canonical_fixture_count": 2,
    "agent_phase_count": 5,
    "agent_transition_count": 8,
    "agent_terminal_phase_count": 1,
    "progress_event_kind_count": 7,
    "strict_schema_validation": True,
    "unknown_fields_admitted_count": 0,
    "runtime_implementations_added": 0,
    "model_executions": 0,
    "tool_executions": 0,
    "external_network_used": False,
    "manual_fuzzing_executed": False,
    "release_support": False,
}
LIMITATIONS: Final = [
    "The schemas are formal review artifacts for current Rust behavior and do not add a second runtime implementation.",
    "Validation executes no model, tool, platform adapter, or state transition.",
    "Status, interruption, final-response, and persisted terminal-result schemas remain later work; the separately evidenced session-environment schema is outside this task's two-schema claim.",
    "Fixture digests are synthetic review values; no release-signing or package-attestation claim is made.",
    "Cross-platform execution, packaging, release acceptance, and manual fuzzing remain later gates.",
]
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class EvidenceError(ValueError):
    pass


def validate_sources(values: dict[str, str]) -> list[str]:
    failures = []
    document = values[SOURCE_PATHS[0]]
    if tuple(re.findall(r"`(RSC-\d{2})`", document)) != CASE_IDS:
        failures.append("Task 12.1.2.1 case closure changed")
    failures.extend(
        f"Task 12.1.2.1 document fragment changed: {index}"
        for index, fragment in enumerate(DOCUMENT_FRAGMENTS, 1)
        if document.count(fragment) != 1
    )
    for path, fragments in SOURCE_FRAGMENTS.items():
        failures.extend(
            f"Task 12.1.2.1 source fragment changed: {path}:{index}"
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
        "artifact_id": "task-12-1-2-1-runtime-schemas",
        "case_ids": list(CASE_IDS),
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "source_revision": revision,
        "sources": source_records(revision),
        "status": "pass-runtime-state-event-schemas",
        "task_ids": ["12.1.2.1"],
        "verification_commands": [
            run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS
        ],
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    exact = {
        "artifact_id": "task-12-1-2-1-runtime-schemas",
        "case_ids": list(CASE_IDS),
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "status": "pass-runtime-state-event-schemas",
        "task_ids": ["12.1.2.1"],
        "verification_commands": expected_commands(),
    }
    failures = [
        f"Task 12.1.2.1 evidence {key} changed"
        for key, value in exact.items()
        if report.get(key) != value
    ]
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        failures.append("Task 12.1.2.1 evidence revision invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(
        SOURCE_PATHS
    ):
        failures.append("Task 12.1.2.1 evidence sources changed")
    elif any(
        not isinstance(item.get("bytes"), int)
        or item["bytes"] <= 0
        or SHA256.fullmatch(str(item.get("sha256", ""))) is None
        for item in sources
    ):
        failures.append("Task 12.1.2.1 evidence source records invalid")
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
        print(f"Task 12.1.2.1 runtime schema evidence: FAIL: {error}")
        return 1
    print("Task 12.1.2.1 runtime schema evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
