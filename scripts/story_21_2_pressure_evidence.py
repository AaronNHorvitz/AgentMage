#!/usr/bin/env python3
"""Run and validate the bounded Story 21.2 journal pressure evidence."""

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
REPORT_PATH: Final = OUTPUT_DIRECTORY / "pressure-report.json"
LOG_PATH: Final = OUTPUT_DIRECTORY / "pressure-report.log"
METRIC_PREFIX: Final = "AGENTMAGE_RUNTIME_PRESSURE_METRICS="
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
COMMANDS: Final = (
    (
        "canonical-byte-boundary",
        (
            "cargo",
            "test",
            "-p",
            "agentmage-kernel-engine",
            "--lib",
            "--locked",
            "runtime_journal::tests::story_21_2_worker_enforces_the_exact_canonical_byte_boundary",
            "--",
            "--exact",
            "--nocapture",
        ),
    ),
    (
        "delayed-sqlcipher-cancellation",
        (
            "cargo",
            "test",
            "-p",
            "agentmage-kernel-engine",
            "--lib",
            "--locked",
            "runtime_loop::tests::story_21_2_durable_model_progress_and_cancellation_survive_delayed_sqlcipher",
            "--",
            "--exact",
            "--nocapture",
        ),
    ),
)
SOURCE_PATHS: Final = (
    "kernel/engine/src/runtime_journal.rs",
    "kernel/engine/src/runtime_loop.rs",
    "kernel/engine/src/runtime_loop_tests.rs",
    "scripts/story_21_2_pressure_evidence.py",
    "tests/test_story_21_2_pressure_evidence.py",
)
COVERAGE: Final = {
    "one_byte_below_queue_capacity_rejected": True,
    "exact_queue_byte_capacity_accepted": True,
    "rejected_event_retry_replayed_once": True,
    "model_progress_while_store_delayed": True,
    "client_progress_while_store_delayed": True,
    "terminal_waited_for_correctness_durability": True,
    "cancellation_order_replayed_exactly": True,
}
LIMITATIONS: Final = (
    "The SQLCipher delay is induced by holding the process-owned sole-store mutex; physical device, controller, filesystem, and kernel fault injection are not claimed.",
    "The model boundary is a deterministic source fixture; installed model-runtime and native-client performance are not claimed.",
    "The result is current-host Linux source evidence, not an additional supported-platform profile or release evidence.",
    "Manual fuzzing remains deferred and was not executed by this campaign.",
    "The retained command trace replaces the absolute checkout root with the literal <repository-root>; metrics and test results are unchanged.",
)


class PressureEvidenceError(ValueError):
    """Raised when pressure evidence is unavailable, malformed, or stale."""


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
        raise PressureEvidenceError("runtime.pressure.source_revision")
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
        raise PressureEvidenceError("runtime.pressure.source_unavailable")
    return result.stdout


def command_sha256(command: tuple[str, ...]) -> str:
    return sha256_bytes("\0".join(command).encode("utf-8"))


def parse_metrics(output: str) -> dict[str, Any]:
    records = [
        line.removeprefix(METRIC_PREFIX)
        for line in output.splitlines()
        if line.startswith(METRIC_PREFIX)
    ]
    if len(records) != 1:
        raise PressureEvidenceError("runtime.pressure.metric_count")
    try:
        metrics = json.loads(records[0])
    except json.JSONDecodeError as error:
        raise PressureEvidenceError("runtime.pressure.metric_json") from error
    failures = metric_failures(metrics)
    if failures:
        raise PressureEvidenceError("; ".join(failures))
    return metrics


def metric_failures(metrics: Any) -> list[str]:
    if not isinstance(metrics, dict):
        return ["runtime.pressure.metric_type"]
    expected_fields = {
        "cancellation_latency_us",
        "cancellation_limit_ms",
        "client_progress_while_store_delayed",
        "external_network_used",
        "fragment_count_before_cancellation",
        "store_delay_ms",
        "terminal_waited_for_correctness_durability",
    }
    failures: list[str] = []
    if set(metrics) != expected_fields:
        failures.append("runtime.pressure.metric_fields")
        return failures
    cancellation_latency = metrics["cancellation_latency_us"]
    if (
        not isinstance(cancellation_latency, int)
        or isinstance(cancellation_latency, bool)
        or not 0 <= cancellation_latency < 250_000
        or metrics["cancellation_limit_ms"] != 250
    ):
        failures.append("runtime.pressure.cancellation_latency")
    fragments = metrics["fragment_count_before_cancellation"]
    if (
        not isinstance(fragments, int)
        or isinstance(fragments, bool)
        or not 16 <= fragments <= 4_096
    ):
        failures.append("runtime.pressure.fragment_count")
    store_delay = metrics["store_delay_ms"]
    if (
        not isinstance(store_delay, int)
        or isinstance(store_delay, bool)
        or not 1 <= store_delay <= 2_000
    ):
        failures.append("runtime.pressure.store_delay")
    for field in (
        "client_progress_while_store_delayed",
        "terminal_waited_for_correctness_durability",
    ):
        if metrics[field] is not True:
            failures.append(f"runtime.pressure.{field}")
    if metrics["external_network_used"] is not False:
        failures.append("runtime.pressure.external_network")
    return failures


def source_records(revision: str) -> list[dict[str, Any]]:
    return [
        {"path": relative, "bytes": len(content), "sha256": sha256_bytes(content)}
        for relative in SOURCE_PATHS
        for content in [git_blob(revision, relative)]
    ]


def run_campaign() -> tuple[str, list[dict[str, Any]], dict[str, Any]]:
    traces: list[str] = []
    results: list[dict[str, Any]] = []
    pressure_metrics: dict[str, Any] | None = None
    for command_id, command in COMMANDS:
        started = time.monotonic()
        result = subprocess.run(
            list(command),
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
        output = (result.stdout + result.stderr).replace(
            str(ROOT), "<repository-root>"
        )
        if result.returncode or "1 passed; 0 failed" not in output:
            raise PressureEvidenceError(f"runtime.pressure.command_failed.{command_id}")
        if command_id == "delayed-sqlcipher-cancellation":
            pressure_metrics = parse_metrics(output)
        elif METRIC_PREFIX in output:
            raise PressureEvidenceError("runtime.pressure.unexpected_metric")
        elapsed_ms = max(1, int((time.monotonic() - started) * 1_000))
        traces.append(f"=== {command_id} ===\n{output}")
        results.append(
            {
                "command_id": command_id,
                "argv_sha256": command_sha256(command),
                "elapsed_ms": elapsed_ms,
                "exit_code": 0,
                "passed": 1,
                "failed": 0,
            }
        )
    if pressure_metrics is None:
        raise PressureEvidenceError("runtime.pressure.metric_missing")
    return "\n".join(traces), results, pressure_metrics


def build_report(
    revision: str,
    output: str,
    results: list[dict[str, Any]],
    metrics: dict[str, Any],
) -> dict[str, Any]:
    raw = output.encode("utf-8")
    return {
        "schema_version": 1,
        "artifact_id": "story-21.2-runtime-journal-pressure",
        "source_revision": revision,
        "status": "pass-current-linux-source-pressure",
        "task_ids": ["21.2.3.3", "RV-18"],
        "coverage": COVERAGE,
        "metrics": metrics,
        "commands": results,
        "raw_trace": {
            "path": str(LOG_PATH.relative_to(ROOT)),
            "bytes": len(raw),
            "sha256": sha256_bytes(raw),
            "redactions": ["repository-root"],
        },
        "sources": source_records(revision),
        "external_network_used": False,
        "private_user_data_used": False,
        "limitations": list(LIMITATIONS),
    }


def validate_report(report: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(report, dict):
        return ["runtime.pressure.report_type"]
    if report.get("schema_version") != 1 or report.get("artifact_id") != (
        "story-21.2-runtime-journal-pressure"
    ):
        failures.append("runtime.pressure.report_identity")
    revision = report.get("source_revision")
    if not isinstance(revision, str) or REVISION.fullmatch(revision) is None:
        failures.append("runtime.pressure.report_revision")
    failures.extend(metric_failures(report.get("metrics")))
    if report.get("coverage") != COVERAGE:
        failures.append("runtime.pressure.report_coverage")
    commands = report.get("commands")
    if not isinstance(commands, list) or len(commands) != len(COMMANDS):
        failures.append("runtime.pressure.report_commands")
    else:
        for record, (command_id, command) in zip(commands, COMMANDS, strict=True):
            if (
                record.get("command_id") != command_id
                or record.get("argv_sha256") != command_sha256(command)
                or record.get("exit_code") != 0
                or record.get("passed") != 1
                or record.get("failed") != 0
                or not isinstance(record.get("elapsed_ms"), int)
                or record["elapsed_ms"] <= 0
            ):
                failures.append("runtime.pressure.report_command")
                break
    raw_trace = report.get("raw_trace")
    if (
        not isinstance(raw_trace, dict)
        or raw_trace.get("path") != str(LOG_PATH.relative_to(ROOT))
        or raw_trace.get("redactions") != ["repository-root"]
    ):
        failures.append("runtime.pressure.report_trace")
    elif not LOG_PATH.is_file():
        failures.append("runtime.pressure.trace_missing")
    else:
        raw = LOG_PATH.read_bytes()
        if raw_trace.get("bytes") != len(raw) or raw_trace.get("sha256") != sha256_bytes(raw):
            failures.append("runtime.pressure.trace_drift")
        else:
            try:
                parse_metrics(raw.decode("utf-8"))
            except (UnicodeError, PressureEvidenceError):
                failures.append("runtime.pressure.trace_invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(
        SOURCE_PATHS
    ):
        failures.append("runtime.pressure.report_sources")
    elif isinstance(revision, str) and REVISION.fullmatch(revision):
        for record in sources:
            content = git_blob(revision, record["path"])
            if (
                record.get("bytes") != len(content)
                or SHA256.fullmatch(str(record.get("sha256", ""))) is None
                or record["sha256"] != sha256_bytes(content)
            ):
                failures.append("runtime.pressure.source_drift")
                break
    if report.get("status") != "pass-current-linux-source-pressure" or report.get(
        "task_ids"
    ) != ["21.2.3.3", "RV-18"]:
        failures.append("runtime.pressure.report_disposition")
    for field in ("external_network_used", "private_user_data_used"):
        if report.get(field) is not False:
            failures.append(f"runtime.pressure.{field}")
    if report.get("limitations") != list(LIMITATIONS):
        failures.append("runtime.pressure.report_limitations")
    return failures


def read_report() -> Any:
    try:
        return json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise PressureEvidenceError("runtime.pressure.report_unavailable") from error


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    if arguments.write:
        revision = git_revision(arguments.source_revision)
        output, results, metrics = run_campaign()
        atomic_write(LOG_PATH, output.encode("utf-8"))
        atomic_write(
            REPORT_PATH,
            canonical_json_bytes(build_report(revision, output, results, metrics)),
        )
    report = read_report()
    failures = validate_report(report)
    if failures:
        raise PressureEvidenceError("; ".join(failures))
    print("Story 21.2 journal pressure evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
