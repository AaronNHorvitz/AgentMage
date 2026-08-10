#!/usr/bin/env python3
"""Build and verify the S-004-IT01 boundary integration traces."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
REPORT_PATH = (
    ROOT
    / "artifacts"
    / "sprints"
    / "sprint-4"
    / "story-4.1"
    / "kernel-boundary-integration-report.json"
)
MARKER = "AGENTMAGE_BOUNDARY_TRACES="
SOURCE_PATHS = (
    "Cargo.lock",
    "kernel/contracts/src/boundary.rs",
    "kernel/contracts/src/tool.rs",
    "kernel/engine/src/propagation.rs",
    "kernel/engine/tests/boundary_workflow.rs",
    "scripts/kernel_boundary_integration.py",
    "tests/test_kernel_boundary_integration.py",
)
EXPECTED_CASES = (
    "boundary.success",
    "boundary.denied",
    "boundary.cancelled",
    "boundary.timed_out",
    "boundary.failed",
)
EXPECTED_ROUTES = {
    "boundary.success": ["tool", "platform_adapter", "kernel", "shell"],
    "boundary.denied": ["tool", "platform_adapter", "kernel", "shell"],
    "boundary.cancelled": ["tool", "platform_adapter", "kernel", "shell"],
    "boundary.timed_out": ["model", "kernel", "shell"],
    "boundary.failed": ["model", "kernel", "shell"],
}


class BoundaryIntegrationError(ValueError):
    """Raised when the boundary integration trace is incomplete."""


def canonical_json(value: Any) -> str:
    return json.dumps(value, indent=2, sort_keys=True) + "\n"


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def git_revision(root: Path = ROOT) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout.strip()


def run_boundary_trace(root: Path = ROOT) -> list[dict[str, Any]]:
    environment = os.environ.copy()
    environment["AGENTMAGE_EMIT_BOUNDARY_TRACES"] = "1"
    result = subprocess.run(
        [
            "cargo",
            "test",
            "-p",
            "agentmage-kernel-engine",
            "--test",
            "boundary_workflow",
            "--locked",
            "success_and_every_failure_class_preserve_boundary_context",
            "--",
            "--exact",
            "--nocapture",
        ],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
        env=environment,
    )
    lines = [line for line in result.stdout.splitlines() if line.startswith(MARKER)]
    if len(lines) != 1:
        raise BoundaryIntegrationError("Rust test emitted an invalid boundary trace count")
    traces = json.loads(lines[0][len(MARKER) :])
    if not isinstance(traces, list) or not all(isinstance(item, dict) for item in traces):
        raise BoundaryIntegrationError("Rust boundary trace must be an object array")
    return traces


def trace_failures(traces: Any) -> list[str]:
    if not isinstance(traces, list):
        return ["boundary traces must be an array"]
    failures: list[str] = []
    case_ids = [item.get("case_id") for item in traces if isinstance(item, dict)]
    if tuple(case_ids) != EXPECTED_CASES:
        failures.append("boundary trace case order or closure is incomplete")
    correlation_ids = {
        item.get("correlation_id") for item in traces if isinstance(item, dict)
    }
    task_ids = {item.get("task_id") for item in traces if isinstance(item, dict)}
    if correlation_ids != {"correlation-integration-0001"}:
        failures.append("boundary correlation identity was lost or changed")
    if task_ids != {"task-integration-0001"}:
        failures.append("boundary task identity was lost or changed")
    for item in traces:
        if not isinstance(item, dict):
            failures.append("every boundary trace must be an object")
            continue
        case_id = item.get("case_id", "unknown")
        if item.get("route") != EXPECTED_ROUTES.get(case_id):
            failures.append(f"{case_id} route is not exact")
        if case_id == "boundary.success":
            if item.get("outcome") != "succeeded" or item.get("error_code") is not None:
                failures.append("success trace contains failure context")
            continue
        expected_outcome = case_id.removeprefix("boundary.")
        expected_code = "fixture." + expected_outcome
        if item.get("outcome") != expected_outcome:
            failures.append(f"{case_id} outcome is not exact")
        if item.get("error_code") != expected_code:
            failures.append(f"{case_id} error context was lost")
        if item.get("error_id") != f"error-{expected_code}":
            failures.append(f"{case_id} error identity was lost")
        if item.get("error_field_path") != ["fixture"]:
            failures.append(f"{case_id} error field path was lost")
        if item.get("caused_by") != "error-upstream-0001":
            failures.append(f"{case_id} causal identity was lost")
        has_cancellation = item.get("cancellation_id") is not None
        if has_cancellation != (case_id == "boundary.cancelled"):
            failures.append(f"{case_id} cancellation context is inconsistent")
    return failures


def build_report(source_revision: str, root: Path = ROOT) -> dict[str, Any]:
    traces = run_boundary_trace(root)
    failures = trace_failures(traces)
    return {
        "schema_version": 1,
        "test_id": "S-004-IT01",
        "task_id": "4.1.3.4",
        "artifact_id": "kernel-boundary-integration-report",
        "status": "pass-linux-boundary-integration" if not failures else "fail",
        "source_revision": source_revision,
        "source_records": [
            {"path": relative, "sha256": sha256_file(root / relative)}
            for relative in SOURCE_PATHS
        ],
        "trace_count": len(traces),
        "traces": traces,
        "failures": failures,
        "success_contract": "ToolResult",
        "non_success_contract": "BoundaryFailure",
        "network_authority": "none",
        "platform_status": {
            "linux_test": "verified-local",
            "macos_test": "blocked-macos",
            "macos_implementation_claim": "none",
        },
        "limitations": [
            "Success is a normal ToolResult and is not represented as BoundaryFailure.",
            "The test uses fake in-process observers; production shell, model, and tool adapters are later stories.",
            "No execution, durable transport, or macOS evidence is claimed.",
        ],
    }


def validate_report(report: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(report, dict):
        return ["boundary integration report must be an object"]
    revision = report.get("source_revision")
    if not isinstance(revision, str) or len(revision) != 40:
        return ["boundary integration source revision is invalid"]
    try:
        expected = build_report(revision, root)
    except (OSError, ValueError, subprocess.CalledProcessError, json.JSONDecodeError) as error:
        return [f"cannot rebuild boundary integration report: {error}"]
    failures: list[str] = []
    if report != expected:
        failures.append("boundary integration report is stale or malformed")
    if report.get("status") != "pass-linux-boundary-integration":
        failures.append("boundary integration report is not passing")
    return failures


def write_report(root: Path = ROOT) -> None:
    REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
    REPORT_PATH.write_text(
        canonical_json(build_report(git_revision(root), root)), encoding="utf-8"
    )


def check_report(root: Path = ROOT) -> list[str]:
    try:
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read boundary integration report: {error}"]
    return validate_report(report, root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    if args.write:
        write_report()
    failures = check_report()
    if failures:
        for failure in failures:
            print(f"boundary integration validation failed: {failure}", file=sys.stderr)
        return 1
    print(f"Validated {len(EXPECTED_CASES)} boundary integration traces")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
