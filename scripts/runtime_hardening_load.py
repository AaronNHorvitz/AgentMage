#!/usr/bin/env python3
"""Run and retain the bounded Story 50.2 Linux reference-load campaign."""

from __future__ import annotations

import argparse
import json
import os
import platform
import re
import shutil
import subprocess
import tempfile
from pathlib import Path
from typing import Any

try:
    from scripts.evidence_core import (
        EvidenceError,
        atomic_write,
        bounded_read,
        canonical_json_bytes,
        git_blob,
        git_source_identity,
        read_json_object,
        sha256_bytes,
        sha256_file,
    )
except ModuleNotFoundError:
    from evidence_core import (  # type: ignore[no-redef]
        EvidenceError,
        atomic_write,
        bounded_read,
        canonical_json_bytes,
        git_blob,
        git_source_identity,
        read_json_object,
        sha256_bytes,
        sha256_file,
    )

ROOT = Path(__file__).resolve().parents[1]
PROFILE = "fixtures/runtime-hardening/v1/linux-reference-load-profile.json"
DEFAULT_OUTPUT = Path("artifacts/sprints/sprint-50/story-50.2-runtime-load")
METRIC_PREFIX = "AGENTMAGE_RUNTIME_LOAD_METRICS="
MAXIMUM_COMMAND_OUTPUT_BYTES = 16 * 1024 * 1024
TEST_RESULT = re.compile(
    r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; "
    r"(\d+) measured; (\d+) filtered out"
)


class CampaignError(ValueError):
    """Raised when campaign input or output cannot be admitted."""


def _exact_keys(value: dict[str, Any], expected: set[str], code: str) -> None:
    if set(value) != expected:
        raise CampaignError(code)


def validate_profile(profile: dict[str, Any]) -> None:
    """Validate the closed command and threshold profile before execution."""

    _exact_keys(
        profile,
        {
            "schema_version",
            "profile_id",
            "source_files",
            "workload",
            "metric_thresholds",
            "commands",
            "declared_limitations",
        },
        "runtime.load.profile.fields",
    )
    if profile["schema_version"] != 1 or profile["profile_id"] != (
        "story-50.2-linux-reference-load-v1"
    ):
        raise CampaignError("runtime.load.profile.identity")
    if not isinstance(profile["source_files"], list) or not profile["source_files"]:
        raise CampaignError("runtime.load.profile.sources")
    if len(set(profile["source_files"])) != len(profile["source_files"]):
        raise CampaignError("runtime.load.profile.sources")
    if not isinstance(profile["declared_limitations"], list) or len(
        profile["declared_limitations"]
    ) < 3:
        raise CampaignError("runtime.load.profile.limitations")
    _exact_keys(
        profile["workload"],
        {
            "progress_events",
            "total_events",
            "restart_cycles",
            "slow_subscriber_capacity",
        },
        "runtime.load.profile.workload",
    )
    _exact_keys(
        profile["metric_thresholds"],
        {
            "maximum_journal_elapsed_ms",
            "minimum_journal_events_per_second",
            "maximum_publish_elapsed_ms",
            "minimum_publish_events_per_second",
            "maximum_restart_elapsed_ms",
            "maximum_resident_memory_kib",
            "maximum_retained_disk_bytes",
            "maximum_queued_events",
            "maximum_queued_bytes",
            "expected_lagged_subscribers",
        },
        "runtime.load.profile.thresholds",
    )
    commands = profile["commands"]
    if not isinstance(commands, list) or len(commands) != 8:
        raise CampaignError("runtime.load.profile.commands")
    identities: set[str] = set()
    metric_commands = 0
    for command in commands:
        _exact_keys(
            command,
            {
                "id",
                "argv",
                "minimum_passed",
                "maximum_elapsed_ms",
                "maximum_rss_kib",
                "requires_load_metrics",
            },
            "runtime.load.profile.command_fields",
        )
        identity = command["id"]
        argv = command["argv"]
        if not isinstance(identity, str) or not identity or identity in identities:
            raise CampaignError("runtime.load.profile.command_identity")
        identities.add(identity)
        if (
            not isinstance(argv, list)
            or len(argv) < 6
            or argv[0] != "cargo"
            or argv[1] != "test"
            or any(not isinstance(argument, str) or not argument for argument in argv)
        ):
            raise CampaignError("runtime.load.profile.command_argv")
        if any(
            not isinstance(command[field], int) or command[field] <= 0
            for field in ("minimum_passed", "maximum_elapsed_ms", "maximum_rss_kib")
        ):
            raise CampaignError("runtime.load.profile.command_limit")
        if not isinstance(command["requires_load_metrics"], bool):
            raise CampaignError("runtime.load.profile.command_metric")
        metric_commands += int(command["requires_load_metrics"])
    if metric_commands != 1:
        raise CampaignError("runtime.load.profile.metric_owner")


def parse_test_results(output: str) -> dict[str, int]:
    """Aggregate every Rust harness summary emitted by one bounded command."""

    matches = TEST_RESULT.findall(output)
    if not matches:
        raise CampaignError("runtime.load.command.no_test_result")
    labels = ("passed", "failed", "ignored", "measured", "filtered_out")
    totals = {label: 0 for label in labels}
    for match in matches:
        for label, raw in zip(labels, match, strict=True):
            totals[label] += int(raw)
    return totals


def parse_load_metrics(output: str) -> dict[str, Any]:
    """Load the one closed metric record from the reference test output."""

    records = [
        line.removeprefix(METRIC_PREFIX)
        for line in output.splitlines()
        if line.startswith(METRIC_PREFIX)
    ]
    if len(records) != 1:
        raise CampaignError("runtime.load.metrics.count")
    try:
        value = json.loads(records[0])
    except json.JSONDecodeError as error:
        raise CampaignError("runtime.load.metrics.malformed") from error
    if not isinstance(value, dict):
        raise CampaignError("runtime.load.metrics.type")
    return value


def metric_failures(
    metrics: dict[str, Any], profile: dict[str, Any]
) -> list[str]:
    """Return stable failure codes for workload or threshold drift."""

    workload = profile["workload"]
    limits = profile["metric_thresholds"]
    checks = {
        "runtime.load.event_count": metrics.get("event_count")
        == workload["total_events"],
        "runtime.load.fast_delivery": metrics.get("fast_deliveries")
        == workload["total_events"],
        "runtime.load.restart_count": metrics.get("restart_cycles")
        == workload["restart_cycles"],
        "runtime.load.cleanup": metrics.get("cleanup_verified") is True,
        "runtime.load.lagged_subscribers": metrics.get("lagged_subscribers")
        == limits["expected_lagged_subscribers"],
        "runtime.load.journal_latency": _at_most(
            metrics.get("journal_elapsed_ms"),
            limits["maximum_journal_elapsed_ms"],
        ),
        "runtime.load.journal_throughput": _at_least(
            metrics.get("journal_events_per_second"),
            limits["minimum_journal_events_per_second"],
        ),
        "runtime.load.publish_latency": _at_most(
            metrics.get("publish_elapsed_ms"),
            limits["maximum_publish_elapsed_ms"],
        ),
        "runtime.load.publish_throughput": _at_least(
            metrics.get("publish_events_per_second"),
            limits["minimum_publish_events_per_second"],
        ),
        "runtime.load.restart_latency": _at_most(
            metrics.get("restart_elapsed_ms"),
            limits["maximum_restart_elapsed_ms"],
        ),
        "runtime.load.memory": metrics.get("resident_memory_kib") is None
        or _at_most(
            metrics.get("resident_memory_kib"),
            limits["maximum_resident_memory_kib"],
        ),
        "runtime.load.disk": _at_most(
            metrics.get("retained_disk_bytes"),
            limits["maximum_retained_disk_bytes"],
        ),
        "runtime.load.queue_events": _at_most(
            metrics.get("maximum_queued_events"),
            limits["maximum_queued_events"],
        ),
        "runtime.load.queue_bytes": _at_most(
            metrics.get("maximum_queued_bytes"),
            limits["maximum_queued_bytes"],
        ),
    }
    return [code for code, passed in checks.items() if not passed]


def _at_most(value: Any, maximum: int) -> bool:
    return isinstance(value, int) and not isinstance(value, bool) and 0 <= value <= maximum


def _at_least(value: Any, minimum: int) -> bool:
    return isinstance(value, int) and not isinstance(value, bool) and value >= minimum


def _parse_time_output(path: Path) -> tuple[int, int]:
    values: dict[str, str] = {}
    for line in path.read_text(encoding="ascii").splitlines():
        key, separator, raw = line.partition("=")
        if separator:
            values[key] = raw
    if set(values) != {"elapsed_seconds", "maximum_rss_kib"}:
        raise CampaignError("runtime.load.time_output")
    try:
        elapsed_ms = round(float(values["elapsed_seconds"]) * 1_000)
        maximum_rss_kib = int(values["maximum_rss_kib"])
    except ValueError as error:
        raise CampaignError("runtime.load.time_output") from error
    if elapsed_ms < 0 or maximum_rss_kib < 0:
        raise CampaignError("runtime.load.time_output")
    return elapsed_ms, maximum_rss_kib


def run_command(command: dict[str, Any]) -> tuple[dict[str, Any], bytes]:
    """Run one exact no-shell test command with process-level resource accounting."""

    descriptor, time_name = tempfile.mkstemp(prefix=".runtime-load-time-", dir=ROOT)
    os.close(descriptor)
    time_path = Path(time_name)
    wrapped = [
        "/usr/bin/time",
        "--quiet",
        "--format=elapsed_seconds=%e\\nmaximum_rss_kib=%M",
        f"--output={time_path}",
        *command["argv"],
    ]
    environment = os.environ.copy()
    environment["CARGO_TERM_COLOR"] = "never"
    try:
        result = subprocess.run(
            wrapped,
            cwd=ROOT,
            env=environment,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=(command["maximum_elapsed_ms"] // 1000) + 60,
            check=False,
        )
        if len(result.stdout) > MAXIMUM_COMMAND_OUTPUT_BYTES:
            raise CampaignError("runtime.load.command.output_limit")
        elapsed_ms, maximum_rss_kib = _parse_time_output(time_path)
    except subprocess.TimeoutExpired as error:
        raise CampaignError("runtime.load.command.timeout") from error
    finally:
        time_path.unlink(missing_ok=True)
    output = result.stdout.decode("utf-8", errors="strict")
    tests = parse_test_results(output)
    metrics = parse_load_metrics(output) if command["requires_load_metrics"] else None
    failures: list[str] = []
    if result.returncode != 0:
        failures.append("runtime.load.command.exit")
    if tests["passed"] < command["minimum_passed"] or tests["failed"] != 0:
        failures.append("runtime.load.command.tests")
    if tests["ignored"] != 0 or tests["measured"] != 0:
        failures.append("runtime.load.command.skips")
    if elapsed_ms > command["maximum_elapsed_ms"]:
        failures.append("runtime.load.command.elapsed")
    if maximum_rss_kib > command["maximum_rss_kib"]:
        failures.append("runtime.load.command.memory")
    return (
        {
            "id": command["id"],
            "argv": command["argv"],
            "exit_code": result.returncode,
            "elapsed_ms": elapsed_ms,
            "maximum_rss_kib": maximum_rss_kib,
            "tests": tests,
            "output_sha256": sha256_bytes(result.stdout),
            "failures": failures,
            "metrics": metrics,
        },
        result.stdout,
    )


def _hardware() -> dict[str, Any]:
    cpu_model = None
    cpuinfo = Path("/proc/cpuinfo")
    if cpuinfo.is_file():
        cpu_model = next(
            (
                line.partition(":")[2].strip()
                for line in cpuinfo.read_text(encoding="utf-8").splitlines()
                if line.startswith("model name")
            ),
            None,
        )
    memory_kib = None
    meminfo = Path("/proc/meminfo")
    if meminfo.is_file():
        memory_kib = next(
            (
                int(line.split()[1])
                for line in meminfo.read_text(encoding="ascii").splitlines()
                if line.startswith("MemTotal:")
            ),
            None,
        )
    return {
        "system": platform.system(),
        "release": platform.release(),
        "machine": platform.machine(),
        "cpu_model": cpu_model,
        "logical_cpus": os.cpu_count(),
        "memory_kib": memory_kib,
        "python": platform.python_version(),
    }


def report_failures(
    report: dict[str, Any], profile: dict[str, Any], artifact_root: Path
) -> list[str]:
    """Recompute report, historical-source, and raw-log integrity."""

    failures: list[str] = []
    expected_fields = {
        "schema_version",
        "record_type",
        "profile_id",
        "source",
        "source_sha256",
        "environment",
        "workload",
        "metric_thresholds",
        "commands",
        "metrics",
        "failures",
        "campaign_passed",
        "disposition",
        "declared_limitations",
    }
    if set(report) != expected_fields:
        return ["runtime.load.report.fields"]
    if (
        report["schema_version"] != 1
        or report["record_type"] != "story_50_2_runtime_load"
        or report["profile_id"] != profile["profile_id"]
        or report["workload"] != profile["workload"]
        or report["metric_thresholds"] != profile["metric_thresholds"]
        or report["declared_limitations"] != profile["declared_limitations"]
    ):
        failures.append("runtime.load.report.identity")

    source = report["source"]
    if not isinstance(source, dict) or set(source) != {"revision", "tree"}:
        failures.append("runtime.load.report.source")
        revision = ""
    else:
        revision = source["revision"]
        try:
            if git_source_identity(ROOT, revision) != source:
                failures.append("runtime.load.report.source")
        except EvidenceError:
            failures.append("runtime.load.report.source")

    source_sha256 = report["source_sha256"]
    if not isinstance(source_sha256, dict) or set(source_sha256) != set(
        profile["source_files"]
    ):
        failures.append("runtime.load.report.source_hashes")
    elif revision:
        for path in profile["source_files"]:
            try:
                observed = sha256_bytes(git_blob(ROOT, revision, path))
            except EvidenceError:
                failures.append("runtime.load.report.source_hashes")
                break
            if source_sha256[path] != observed:
                failures.append("runtime.load.report.source_hashes")
                break

    environment = report["environment"]
    if (
        not isinstance(environment, dict)
        or set(environment)
        != {
            "system",
            "release",
            "machine",
            "cpu_model",
            "logical_cpus",
            "memory_kib",
            "python",
        }
        or environment.get("system") != "Linux"
        or not isinstance(environment.get("logical_cpus"), int)
        or environment["logical_cpus"] <= 0
        or not isinstance(environment.get("memory_kib"), int)
        or environment["memory_kib"] <= 0
    ):
        failures.append("runtime.load.report.environment")

    commands = report["commands"]
    expected_campaign_failures: list[str] = []
    metric_records: list[dict[str, Any]] = []
    if not isinstance(commands, list) or len(commands) != len(profile["commands"]):
        failures.append("runtime.load.report.commands")
    else:
        for record, expected in zip(commands, profile["commands"], strict=True):
            command_failures = _report_command_failures(
                record, expected, artifact_root
            )
            if command_failures is None:
                failures.append("runtime.load.report.command")
                continue
            if record["failures"] != command_failures:
                failures.append("runtime.load.report.command_failures")
            expected_campaign_failures.extend(command_failures)
            if record["metrics"] is not None:
                metric_records.append(record["metrics"])

    if len(metric_records) != 1 or report["metrics"] != (
        metric_records[0] if len(metric_records) == 1 else {}
    ):
        expected_campaign_failures.append("runtime.load.metrics.count")
        failures.append("runtime.load.report.metrics")
    else:
        expected_campaign_failures.extend(metric_failures(report["metrics"], profile))

    expected_campaign_failures = sorted(set(expected_campaign_failures))
    expected_pass = not expected_campaign_failures
    if report["failures"] != expected_campaign_failures:
        failures.append("runtime.load.report.failures")
    if report["campaign_passed"] is not expected_pass:
        failures.append("runtime.load.report.disposition")
    expected_disposition = "PARTIAL-PASS" if expected_pass else "FAILED"
    if report["disposition"] != expected_disposition:
        failures.append("runtime.load.report.disposition")
    return sorted(set(failures))


def _report_command_failures(
    record: Any, expected: dict[str, Any], artifact_root: Path
) -> list[str] | None:
    if not isinstance(record, dict) or set(record) != {
        "id",
        "argv",
        "exit_code",
        "elapsed_ms",
        "maximum_rss_kib",
        "tests",
        "output_sha256",
        "failures",
        "metrics",
        "log",
    }:
        return None
    if record["id"] != expected["id"] or record["argv"] != expected["argv"]:
        return None
    tests = record["tests"]
    if not isinstance(tests, dict) or set(tests) != {
        "passed",
        "failed",
        "ignored",
        "measured",
        "filtered_out",
    }:
        return None
    if any(not isinstance(value, int) or value < 0 for value in tests.values()):
        return None
    log_name = record["log"]
    if log_name != f"{expected['id']}.log":
        return None
    try:
        output = bounded_read(
            artifact_root,
            log_name,
            maximum_bytes=MAXIMUM_COMMAND_OUTPUT_BYTES,
        )
    except EvidenceError:
        return None
    if record["output_sha256"] != sha256_bytes(output):
        return None

    failures: list[str] = []
    if record["exit_code"] != 0:
        failures.append("runtime.load.command.exit")
    if tests["passed"] < expected["minimum_passed"] or tests["failed"] != 0:
        failures.append("runtime.load.command.tests")
    if tests["ignored"] != 0 or tests["measured"] != 0:
        failures.append("runtime.load.command.skips")
    if not _at_most(record["elapsed_ms"], expected["maximum_elapsed_ms"]):
        failures.append("runtime.load.command.elapsed")
    if not _at_most(record["maximum_rss_kib"], expected["maximum_rss_kib"]):
        failures.append("runtime.load.command.memory")
    if expected["requires_load_metrics"] != (record["metrics"] is not None):
        failures.append("runtime.load.command.metrics")
    return failures


def _require_clean_tree() -> None:
    result = subprocess.run(
        ["git", "status", "--porcelain"],
        cwd=ROOT,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=30,
        check=False,
    )
    if result.returncode != 0 or result.stdout:
        raise CampaignError("runtime.load.source_not_clean")


def generate(output: Path) -> dict[str, Any]:
    """Execute the exact profile and write one immutable local evidence directory."""

    _require_clean_tree()
    profile = read_json_object(ROOT, PROFILE)
    validate_profile(profile)
    output = (ROOT / output).resolve() if not output.is_absolute() else output.resolve()
    try:
        output.relative_to(ROOT.resolve())
    except ValueError as error:
        raise CampaignError("runtime.load.output.outside_repository") from error
    if output.exists():
        raise CampaignError("runtime.load.output.exists")
    output.parent.mkdir(parents=True, exist_ok=True)
    staging = Path(tempfile.mkdtemp(prefix=".story-50-2-load-", dir=output.parent))
    try:
        command_records: list[dict[str, Any]] = []
        campaign_failures: list[str] = []
        for command in profile["commands"]:
            record, raw_output = run_command(command)
            log_name = f"{command['id']}.log"
            atomic_write(staging / log_name, raw_output)
            record["log"] = log_name
            command_records.append(record)
            campaign_failures.extend(record["failures"])

        metric_records = [
            record["metrics"]
            for record in command_records
            if record["metrics"] is not None
        ]
        if len(metric_records) != 1:
            campaign_failures.append("runtime.load.metrics.count")
            metrics: dict[str, Any] = {}
        else:
            metrics = metric_records[0]
            campaign_failures.extend(metric_failures(metrics, profile))

        source = git_source_identity(ROOT, "HEAD")
        report = {
            "schema_version": 1,
            "record_type": "story_50_2_runtime_load",
            "profile_id": profile["profile_id"],
            "source": source,
            "source_sha256": {
                path: sha256_file(ROOT, path) for path in profile["source_files"]
            },
            "environment": _hardware(),
            "workload": profile["workload"],
            "metric_thresholds": profile["metric_thresholds"],
            "commands": command_records,
            "metrics": metrics,
            "failures": sorted(set(campaign_failures)),
            "campaign_passed": not campaign_failures,
            "disposition": "PARTIAL-PASS" if not campaign_failures else "FAILED",
            "declared_limitations": profile["declared_limitations"],
        }
        validation_failures = report_failures(report, profile, staging)
        if validation_failures:
            raise CampaignError(
                "runtime.load.report.invalid:" + ",".join(validation_failures)
            )
        atomic_write(staging / "report.json", canonical_json_bytes(report))
        os.replace(staging, output)
        return report
    except BaseException:
        shutil.rmtree(staging, ignore_errors=True)
        raise


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    arguments = parser.parse_args()
    try:
        report = generate(arguments.output)
    except (CampaignError, EvidenceError, OSError, UnicodeError) as error:
        print(str(error))
        return 1
    print(json.dumps({"campaign_passed": report["campaign_passed"], "output": str(arguments.output)}))
    return 0 if report["campaign_passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
