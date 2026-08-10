#!/usr/bin/env python3
"""Build and verify the S-004-ST01 pre-grant dispatcher denial traces."""

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
    / "kernel-dispatch-security-report.json"
)
MARKER = "AGENTMAGE_DISPATCH_TRACES="
SOURCE_PATHS = (
    "Cargo.lock",
    "Cargo.toml",
    "kernel/contracts/src/tool.rs",
    "kernel/engine/Cargo.toml",
    "kernel/engine/src/tooling.rs",
    "scripts/kernel_dispatch_security.py",
    "tests/test_kernel_dispatch_security.py",
)
EXPECTED_CASES = (
    "dispatch.shell",
    "dispatch.model",
    "dispatch.tool",
    "dispatch.capability_pack",
    "dispatch.unregistered_caller",
    "dispatch.forged_description",
    "dispatch.unregistered_tool",
)


class DispatchSecurityError(ValueError):
    """Raised when the dispatcher trace is incomplete or unsafe."""


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


def run_dispatch_trace(root: Path = ROOT) -> list[dict[str, Any]]:
    environment = os.environ.copy()
    environment["AGENTMAGE_EMIT_DISPATCH_TRACES"] = "1"
    result = subprocess.run(
        [
            "cargo",
            "test",
            "-p",
            "agentmage-kernel-engine",
            "--locked",
            "tooling::tests::every_proposal_origin_has_an_exact_zero_execution_receipt",
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
        raise DispatchSecurityError("Rust test emitted an invalid dispatcher trace count")
    traces = json.loads(lines[0][len(MARKER) :])
    if not isinstance(traces, list) or not all(isinstance(item, dict) for item in traces):
        raise DispatchSecurityError("Rust dispatcher trace must be an object array")
    return traces


def trace_failures(traces: Any) -> list[str]:
    if not isinstance(traces, list):
        return ["dispatcher traces must be an array"]
    failures: list[str] = []
    case_ids = [item.get("case_id") for item in traces if isinstance(item, dict)]
    if tuple(case_ids) != EXPECTED_CASES:
        failures.append("dispatcher trace case order or closure is incomplete")
    for item in traces:
        if not isinstance(item, dict):
            failures.append("every dispatcher trace must be an object")
            continue
        case_id = item.get("case_id", "unknown")
        expected_outcome = "failed" if case_id == "dispatch.unregistered_tool" else "denied"
        expected_disposition = {
            "dispatch.unregistered_caller": "unregistered_caller",
            "dispatch.unregistered_tool": "invalid_call",
        }.get(case_id, "grant_required")
        expected_error = {
            "dispatch.unregistered_caller": "tool.dispatch.caller_not_registered",
            "dispatch.unregistered_tool": "tool.dispatch.not_registered",
        }.get(case_id, "tool.dispatch.grant_required")
        if item.get("outcome") != expected_outcome:
            failures.append(f"{case_id} outcome is not exact")
        if item.get("disposition") != expected_disposition:
            failures.append(f"{case_id} disposition is not exact")
        if item.get("error_code") != expected_error:
            failures.append(f"{case_id} error code is not exact")
        if item.get("state_change") != "not_changed":
            failures.append(f"{case_id} changed state")
        if item.get("elapsed_ms") != 0:
            failures.append(f"{case_id} reports execution time")
        if item.get("output_present") is not False or item.get("evidence_count") != 0:
            failures.append(f"{case_id} returned output or execution evidence")
        if case_id != "dispatch.unregistered_tool" and item.get("validation_issue_codes") != []:
            failures.append(f"{case_id} contains unexpected validation findings")
    return failures


def build_report(source_revision: str, root: Path = ROOT) -> dict[str, Any]:
    traces = run_dispatch_trace(root)
    failures = trace_failures(traces)
    return {
        "schema_version": 1,
        "test_id": "S-004-ST01",
        "task_id": "4.1.3.3",
        "artifact_id": "kernel-dispatch-security-report",
        "status": "pass-linux-zero-execution" if not failures else "fail",
        "source_revision": source_revision,
        "source_records": [
            {"path": relative, "sha256": sha256_file(root / relative)}
            for relative in SOURCE_PATHS
        ],
        "trace_count": len(traces),
        "traces": traces,
        "failures": failures,
        "executor_callback_available": False,
        "positive_dispatch_path_available": False,
        "network_authority": "none",
        "platform_status": {
            "linux_test": "verified-local",
            "macos_test": "blocked-macos",
            "macos_implementation_claim": "none",
        },
        "limitations": [
            "Proposal origin is untrusted denial provenance, not caller authentication.",
            "The receipt is an in-process pre-grant record, not durable receipt-ledger storage.",
            "No positive execution path or macOS execution evidence is claimed.",
        ],
    }


def validate_report(report: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(report, dict):
        return ["dispatcher security report must be an object"]
    revision = report.get("source_revision")
    if not isinstance(revision, str) or len(revision) != 40:
        return ["dispatcher security report source revision is invalid"]
    try:
        expected = build_report(revision, root)
    except (OSError, ValueError, subprocess.CalledProcessError, json.JSONDecodeError) as error:
        return [f"cannot rebuild dispatcher security report: {error}"]
    failures: list[str] = []
    if report != expected:
        failures.append("dispatcher security report is stale or malformed")
    if report.get("status") != "pass-linux-zero-execution":
        failures.append("dispatcher security report is not passing")
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
        return [f"cannot read dispatcher security report: {error}"]
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
            print(f"dispatcher security validation failed: {failure}", file=sys.stderr)
        return 1
    print(f"Validated {len(EXPECTED_CASES)} pre-grant dispatcher denial traces")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
