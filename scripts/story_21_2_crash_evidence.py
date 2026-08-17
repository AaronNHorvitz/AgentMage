#!/usr/bin/env python3
"""Run and validate the deterministic Story 21.2 journal stop matrix."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import time
from pathlib import Path
from typing import Any, Final

try:
    from scripts.evidence_core import atomic_write, canonical_json_bytes
except ModuleNotFoundError:
    from evidence_core import atomic_write, canonical_json_bytes


ROOT: Final = Path(__file__).resolve().parents[1]
OUTPUT_DIRECTORY: Final = ROOT / "artifacts/sprints/sprint-21/story-21.2"
REPORT_PATH: Final = OUTPUT_DIRECTORY / "crash-matrix.json"
LOG_PATH: Final = OUTPUT_DIRECTORY / "crash-matrix.log"
METRIC_PREFIX: Final = "AGENTMAGE_RUNTIME_CRASH_MATRIX="
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
BOUNDARIES: Final = (
    "queue-admission",
    "batch-flush",
    "correctness-transaction",
    "subscriber-publication",
)
POSITIONS: Final = ("before", "after")
SOURCE_PATHS: Final = (
    "kernel/engine/src/runtime_journal.rs",
    "scripts/story_21_2_crash_evidence.py",
    "tests/test_story_21_2_crash_evidence.py",
)
COMMAND: Final = (
    "cargo",
    "test",
    "-p",
    "agentmage-kernel-engine",
    "--lib",
    "--locked",
    "runtime_journal::tests::story_21_2_process_stop_matrix_preserves_one_truthful_replay",
    "--",
    "--exact",
    "--nocapture",
)


class CrashEvidenceError(ValueError):
    """Raised when crash evidence is unavailable, malformed, or stale."""


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


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
        raise CrashEvidenceError("runtime.crash.source_revision")
    return revision


def git_blob(revision: str, relative: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{relative}"],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=60,
        check=False,
    )
    if result.returncode or not result.stdout:
        raise CrashEvidenceError("runtime.crash.source_unavailable")
    return result.stdout


def expected_cases() -> list[dict[str, Any]]:
    cases = []
    for boundary in BOUNDARIES:
        for position in POSITIONS:
            middle_was_durable = (
                (boundary == "batch-flush" and position == "after")
                or (boundary == "correctness-transaction" and position == "after")
                or boundary == "subscriber-publication"
            )
            cases.append(
                {
                    "boundary": boundary,
                    "position": position,
                    "recovered_event_count": 2 if middle_was_durable else 1,
                    "middle_was_durable": middle_was_durable,
                    "bounded_progress_loss": (
                        (boundary == "queue-admission" and position == "after")
                        or (boundary == "batch-flush" and position == "before")
                    ),
                    "subscriber_delivered": (
                        boundary == "subscriber-publication" and position == "after"
                    ),
                    "false_terminal_before_recovery": False,
                    "final_event_count": 3,
                    "second_reopen_verified": True,
                }
            )
    return cases


def expected_metrics() -> dict[str, Any]:
    return {
        "boundary_count": len(BOUNDARIES),
        "position_count": len(POSITIONS),
        "case_count": len(BOUNDARIES) * len(POSITIONS),
        "cases": expected_cases(),
        "external_network_used": False,
        "false_terminal_count": 0,
        "manual_fuzzing_executed": False,
    }


def parse_metrics(output: str) -> dict[str, Any]:
    records = [
        line.removeprefix(METRIC_PREFIX)
        for line in output.splitlines()
        if line.startswith(METRIC_PREFIX)
    ]
    if len(records) != 1:
        raise CrashEvidenceError("runtime.crash.metric_count")
    try:
        metrics = json.loads(records[0])
    except json.JSONDecodeError as error:
        raise CrashEvidenceError("runtime.crash.metric_json") from error
    if metrics != expected_metrics():
        raise CrashEvidenceError("runtime.crash.metric_drift")
    return metrics


def source_records(revision: str) -> list[dict[str, Any]]:
    return [
        {
            "path": relative,
            "bytes": len(content),
            "sha256": sha256_bytes(content),
        }
        for relative in SOURCE_PATHS
        for content in [git_blob(revision, relative)]
    ]


def run_matrix() -> tuple[str, int]:
    started = time.monotonic()
    result = subprocess.run(
        list(COMMAND),
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        timeout=300,
        check=False,
        env={
            **os.environ,
            "CARGO_TERM_COLOR": "never",
            "LANG": "C",
            "LC_ALL": "C",
        },
    )
    output = (result.stdout + result.stderr).replace(str(ROOT), "<repository-root>")
    if result.returncode or "1 passed; 0 failed" not in output:
        raise CrashEvidenceError("runtime.crash.command_failed")
    parse_metrics(output)
    elapsed_ms = max(1, int((time.monotonic() - started) * 1_000))
    return output, elapsed_ms


def build_report(revision: str, output: str, elapsed_ms: int) -> dict[str, Any]:
    metrics = parse_metrics(output)
    raw = output.encode("utf-8")
    return {
        "schema_version": 1,
        "artifact_id": "story-21.2-runtime-journal-crash-matrix",
        "source_revision": revision,
        "status": "pass-current-linux-source-boundary",
        "task_ids": ["21.2.3.2", "RV-18"],
        "boundaries": list(BOUNDARIES),
        "positions": list(POSITIONS),
        "metrics": metrics,
        "verification": {
            "command_id": sha256_bytes("\0".join(COMMAND).encode("utf-8")),
            "elapsed_ms": elapsed_ms,
            "exit_code": 0,
        },
        "raw_trace": {
            "path": str(LOG_PATH.relative_to(ROOT)),
            "bytes": len(raw),
            "sha256": sha256_bytes(raw),
            "redactions": ["repository-root"],
        },
        "sources": source_records(revision),
        "external_network_used": False,
        "private_user_data_used": False,
        "limitations": [
            "The child uses deterministic process exit without unwinding; kernel SIGKILL, host power loss, torn sectors, controller failure, and filesystem corruption are not claimed.",
            "This matrix covers the journal queue, batch flush, correctness transaction, and subscriber publication boundaries; integrated physical-effect recovery remains separate.",
            "The result is current-host Linux source evidence, not installed-platform or release evidence.",
            "Manual fuzzing remains deferred and was not executed by this campaign.",
            "The retained command trace replaces the absolute checkout root with the literal <repository-root>; event metrics and test results are unchanged.",
        ],
    }


def validate_report(report: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(report, dict):
        return ["runtime.crash.report_type"]
    if report.get("schema_version") != 1 or report.get("artifact_id") != (
        "story-21.2-runtime-journal-crash-matrix"
    ):
        failures.append("runtime.crash.report_identity")
    revision = report.get("source_revision")
    if not isinstance(revision, str) or REVISION.fullmatch(revision) is None:
        failures.append("runtime.crash.report_revision")
    if report.get("boundaries") != list(BOUNDARIES) or report.get("positions") != list(
        POSITIONS
    ):
        failures.append("runtime.crash.report_matrix")
    if report.get("metrics") != expected_metrics():
        failures.append("runtime.crash.report_metrics")
    verification = report.get("verification")
    if not isinstance(verification, dict) or verification.get("command_id") != sha256_bytes(
        "\0".join(COMMAND).encode("utf-8")
    ):
        failures.append("runtime.crash.report_command")
    elif (
        verification.get("exit_code") != 0
        or not isinstance(verification.get("elapsed_ms"), int)
        or verification["elapsed_ms"] <= 0
    ):
        failures.append("runtime.crash.report_result")
    raw_trace = report.get("raw_trace")
    if (
        not isinstance(raw_trace, dict)
        or raw_trace.get("path") != str(LOG_PATH.relative_to(ROOT))
        or raw_trace.get("redactions") != ["repository-root"]
    ):
        failures.append("runtime.crash.report_trace")
    elif not LOG_PATH.is_file():
        failures.append("runtime.crash.trace_missing")
    else:
        raw = LOG_PATH.read_bytes()
        if raw_trace.get("bytes") != len(raw) or raw_trace.get("sha256") != sha256_bytes(raw):
            failures.append("runtime.crash.trace_drift")
        else:
            try:
                parse_metrics(raw.decode("utf-8"))
            except (UnicodeError, CrashEvidenceError):
                failures.append("runtime.crash.trace_invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(
        SOURCE_PATHS
    ):
        failures.append("runtime.crash.report_sources")
    elif isinstance(revision, str) and REVISION.fullmatch(revision):
        for record in sources:
            content = git_blob(revision, record["path"])
            if (
                record.get("bytes") != len(content)
                or SHA256.fullmatch(str(record.get("sha256", ""))) is None
                or record["sha256"] != sha256_bytes(content)
            ):
                failures.append("runtime.crash.source_drift")
                break
    for field in ("external_network_used", "private_user_data_used"):
        if report.get(field) is not False:
            failures.append(f"runtime.crash.{field}")
    if report.get("status") != "pass-current-linux-source-boundary" or report.get(
        "task_ids"
    ) != ["21.2.3.2", "RV-18"]:
        failures.append("runtime.crash.report_disposition")
    limitations = report.get("limitations")
    if not isinstance(limitations, list) or len(limitations) != 5:
        failures.append("runtime.crash.report_limitations")
    return failures


def read_report() -> Any:
    try:
        return json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise CrashEvidenceError("runtime.crash.report_unavailable") from error


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    if arguments.write:
        revision = git_revision(arguments.source_revision)
        output, elapsed_ms = run_matrix()
        atomic_write(LOG_PATH, output.encode("utf-8"))
        atomic_write(
            REPORT_PATH,
            canonical_json_bytes(build_report(revision, output, elapsed_ms)),
        )
    report = read_report()
    failures = validate_report(report)
    if failures:
        raise CrashEvidenceError("; ".join(failures))
    print("Story 21.2 journal crash evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
