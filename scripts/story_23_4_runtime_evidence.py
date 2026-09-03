#!/usr/bin/env python3
"""Run and validate the bounded Story 23.4 source-runtime campaign."""

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
OUTPUT_DIRECTORY: Final = ROOT / "artifacts/sprints/sprint-23/story-23.4"
REPORT_PATH: Final = OUTPUT_DIRECTORY / "runtime-evidence.json"
LOG_PATH: Final = OUTPUT_DIRECTORY / "runtime-evidence.log"
TIME_BINARY: Final = Path("/usr/bin/time")
PERFORMANCE_PREFIX: Final = "AGENTMAGE_STORY_23_4_PERFORMANCE="
RSS_PREFIX: Final = "AGENTMAGE_STORY_23_4_RSS_KIB="
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
TEST_RESULT: Final = re.compile(
    r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; "
    r"(\d+) measured; (\d+) filtered out"
)
MAXIMUM_COMMAND_ELAPSED_MS: Final = 120_000
MAXIMUM_COMMAND_RSS_KIB: Final = 1_048_576

COMMANDS: Final = (
    (
        "engine-session-matrix",
        (
            "cargo",
            "test",
            "-p",
            "agentmage-kernel-engine",
            "--lib",
            "--locked",
            "story_23_4",
            "--",
            "--nocapture",
        ),
        18,
    ),
    (
        "native-read-matrix",
        (
            "cargo",
            "test",
            "-p",
            "agentmage-host",
            "--lib",
            "--locked",
            "story_23_4",
            "--",
            "--nocapture",
        ),
        6,
    ),
    (
        "native-chat-transport",
        (
            "cargo",
            "test",
            "-p",
            "agentmage-host",
            "--lib",
            "--locked",
            "native_chat_runtime::tests::",
            "--",
            "--nocapture",
        ),
        3,
    ),
    (
        "three-client-parity",
        (
            "cargo",
            "test",
            "-p",
            "agentmage-host",
            "--lib",
            "--locked",
            "runtime_parity_tests::story_50_2_read_only_and_coding_packets_are_equal_across_all_three_callers",
            "--",
            "--exact",
            "--nocapture",
        ),
        1,
    ),
    (
        "client-confusion-bypass",
        (
            "cargo",
            "test",
            "-p",
            "agentmage-host",
            "--lib",
            "--locked",
            "cli_runtime::tests::story_50_2_confusion_and_interface_bypasses_never_present_or_complete",
            "--",
            "--exact",
            "--nocapture",
        ),
        1,
    ),
    (
        "workflow-authority-bypass",
        (
            "cargo",
            "test",
            "-p",
            "agentmage-host",
            "--lib",
            "--locked",
            "runtime_parity_tests::story_50_2_narrow_workflow_authority_rejects_every_broadening_without_execution",
            "--",
            "--exact",
            "--nocapture",
        ),
        1,
    ),
    (
        "dependency-direction",
        ("python3", "scripts/dependency_rules.py"),
        0,
    ),
    (
        "effect-mediation",
        ("python3", "scripts/effect_boundary.py"),
        0,
    ),
)

SOURCE_PATHS: Final = (
    "kernel/contracts/src/runtime_run.rs",
    "kernel/engine/src/runtime_coordinator.rs",
    "kernel/engine/src/runtime_loop.rs",
    "kernel/engine/src/runtime_loop_tests.rs",
    "shells/host/src/coding_client.rs",
    "shells/host/src/cli_runtime.rs",
    "shells/host/src/native_chat_runtime.rs",
    "shells/host/src/runtime_parity_tests.rs",
    "shells/host/src/runtime_read_tests.rs",
    "shells/host/src/runtime_tools.rs",
    "scripts/dependency_rules.py",
    "scripts/effect_boundary.py",
    "scripts/story_23_4_runtime_evidence.py",
    "tests/test_story_23_4_runtime_evidence.py",
)

COVERAGE: Final = {
    "approval_wait_and_exact_allow": True,
    "bounded_cancellation": True,
    "budget_exhaustion": True,
    "client_confusion_denied": True,
    "dependency_failure_safe_stop": True,
    "direct_answer": True,
    "effect_mediation_structure": True,
    "malformed_proposal": True,
    "maximum_ephemeral_workload": True,
    "model_claim_cannot_mint_success": True,
    "multiple_native_reads": True,
    "native_chat_replay_and_cancellation": True,
    "no_progress": True,
    "one_native_read": True,
    "read_only_git": True,
    "repeated_call": True,
    "search": True,
    "terminal_tool_failure_no_retry": True,
    "transactional_publication_failure_matrix": True,
    "three_client_parity": True,
    "verified_completion": True,
    "workflow_authority_broadening_denied": True,
}

LIMITATIONS: Final = (
    "The campaign uses deterministic fake-model fixtures and does not claim an installed admitted model or production native Chat runtime factory.",
    "The performance values describe current-host Linux source fixtures and are not supported-platform, release, or end-user latency guarantees.",
    "The optional durable journal, artifact, and checkpoint costs remain measured by their owning Story 21.2, Story 22, and Story 50.2 campaigns.",
    "Source-level transactional publication and client-disconnect safe stops are covered; persistent crash/restart reconstruction and physical dependency faults remain outside this campaign.",
    "Manual fuzzing remains deferred and was not executed by this campaign.",
    "The retained trace replaces the absolute checkout root with the literal <repository-root>; test results and metrics are unchanged.",
)


class RuntimeEvidenceError(ValueError):
    """Raised when Story 23.4 evidence is unavailable, malformed, or stale."""


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def command_sha256(command: tuple[str, ...]) -> str:
    return sha256_bytes("\0".join(command).encode("utf-8"))


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
        raise RuntimeEvidenceError("runtime.story23.source_revision")
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
        raise RuntimeEvidenceError("runtime.story23.source_unavailable")
    return result.stdout


def parse_test_result(output: str, minimum_passed: int) -> dict[str, int]:
    matches = TEST_RESULT.findall(output)
    if not matches:
        raise RuntimeEvidenceError("runtime.story23.test_result_missing")
    labels = ("passed", "failed", "ignored", "measured", "filtered_out")
    totals = {label: 0 for label in labels}
    for match in matches:
        for label, raw in zip(labels, match, strict=True):
            totals[label] += int(raw)
    if totals["passed"] < minimum_passed or totals["failed"] != 0:
        raise RuntimeEvidenceError("runtime.story23.test_result_failed")
    return totals


def parse_performance(output: str) -> dict[str, Any]:
    records = [
        line.removeprefix(PERFORMANCE_PREFIX)
        for line in output.splitlines()
        if line.startswith(PERFORMANCE_PREFIX)
    ]
    if len(records) != 1:
        raise RuntimeEvidenceError("runtime.story23.performance_count")
    try:
        metrics = json.loads(records[0])
    except json.JSONDecodeError as error:
        raise RuntimeEvidenceError("runtime.story23.performance_json") from error
    failures = performance_failures(metrics)
    if failures:
        raise RuntimeEvidenceError("; ".join(failures))
    return metrics


def performance_failures(metrics: Any) -> list[str]:
    if not isinstance(metrics, dict):
        return ["runtime.story23.performance_type"]
    scenarios = {
        "direct": ("success", 6, 1, 0, 1, 23),
        "nominal": ("success", 15, 2, 1, 2, 23),
        "maximum": ("success", 33, 4, 3, 4, 23),
        "over_limit": ("exhausted", 33, 4, 3, 4, 0),
        "cancellation": ("cancelled", 4, 0, 0, 0, 0),
    }
    failures: list[str] = []
    if set(metrics) != {"schema_version", "maximum_scenario_us", *scenarios}:
        return ["runtime.story23.performance_fields"]
    limit = metrics.get("maximum_scenario_us")
    if metrics.get("schema_version") != 1 or limit != 250_000:
        failures.append("runtime.story23.performance_identity")
        return failures
    fields = {
        "state",
        "event_count",
        "model_calls",
        "tool_calls",
        "turns",
        "output_bytes",
        "elapsed_us",
    }
    for name, expected in scenarios.items():
        record = metrics.get(name)
        if not isinstance(record, dict) or set(record) != fields:
            failures.append(f"runtime.story23.performance.{name}.fields")
            continue
        observed = tuple(
            record[field]
            for field in (
                "state",
                "event_count",
                "model_calls",
                "tool_calls",
                "turns",
                "output_bytes",
            )
        )
        elapsed = record.get("elapsed_us")
        if observed != expected:
            failures.append(f"runtime.story23.performance.{name}.result")
        if (
            not isinstance(elapsed, int)
            or isinstance(elapsed, bool)
            or elapsed < 0
            or elapsed > limit
        ):
            failures.append(f"runtime.story23.performance.{name}.latency")
    return failures


def parse_rss(output: str) -> int:
    records = [
        line.removeprefix(RSS_PREFIX)
        for line in output.splitlines()
        if line.startswith(RSS_PREFIX)
    ]
    if len(records) != 1 or not records[0].isdigit():
        raise RuntimeEvidenceError("runtime.story23.rss_missing")
    value = int(records[0])
    if not 0 < value <= MAXIMUM_COMMAND_RSS_KIB:
        raise RuntimeEvidenceError("runtime.story23.rss_limit")
    return value


def source_records(revision: str) -> list[dict[str, Any]]:
    return [
        {"path": relative, "bytes": len(content), "sha256": sha256_bytes(content)}
        for relative in SOURCE_PATHS
        for content in [git_blob(revision, relative)]
    ]


def run_campaign() -> tuple[str, list[dict[str, Any]], dict[str, Any]]:
    if not TIME_BINARY.is_file():
        raise RuntimeEvidenceError("runtime.story23.gnu_time_unavailable")
    traces: list[str] = []
    results: list[dict[str, Any]] = []
    performance: dict[str, Any] | None = None
    for command_id, command, minimum_passed in COMMANDS:
        started = time.monotonic()
        result = subprocess.run(
            [str(TIME_BINARY), "-f", f"{RSS_PREFIX}%M", "--", *command],
            cwd=ROOT,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=MAXIMUM_COMMAND_ELAPSED_MS // 1_000,
            check=False,
            env={
                **os.environ,
                "CARGO_TERM_COLOR": "never",
                "LANG": "C",
                "LC_ALL": "C",
            },
        )
        output = (result.stdout + result.stderr).replace(str(ROOT), "<repository-root>")
        elapsed_ms = max(1, int((time.monotonic() - started) * 1_000))
        if result.returncode or elapsed_ms > MAXIMUM_COMMAND_ELAPSED_MS:
            raise RuntimeEvidenceError(f"runtime.story23.command_failed.{command_id}")
        rss_kib = parse_rss(output)
        test_result = (
            parse_test_result(output, minimum_passed) if minimum_passed else None
        )
        if command_id == "engine-session-matrix":
            performance = parse_performance(output)
        elif PERFORMANCE_PREFIX in output:
            raise RuntimeEvidenceError("runtime.story23.unexpected_performance")
        traces.append(f"=== {command_id} ===\n{output}")
        results.append(
            {
                "command_id": command_id,
                "argv_sha256": command_sha256(command),
                "elapsed_ms": elapsed_ms,
                "maximum_elapsed_ms": MAXIMUM_COMMAND_ELAPSED_MS,
                "maximum_rss_kib": MAXIMUM_COMMAND_RSS_KIB,
                "peak_rss_kib": rss_kib,
                "exit_code": 0,
                "test_result": test_result,
            }
        )
    if performance is None:
        raise RuntimeEvidenceError("runtime.story23.performance_missing")
    return "\n".join(traces), results, performance


def build_report(
    revision: str,
    output: str,
    results: list[dict[str, Any]],
    performance: dict[str, Any],
) -> dict[str, Any]:
    raw = output.encode("utf-8")
    return {
        "schema_version": 1,
        "artifact_id": "story-23.4-source-runtime-campaign",
        "source_revision": revision,
        "status": "pass-current-linux-source-runtime",
        "task_ids": [
            "23.4.3.1",
            "23.4.3.2",
            "23.4.3.3",
            "23.4.3.4",
            "23.4.3.5",
            "RV-05",
            "RV-17",
            "RV-18",
        ],
        "coverage": COVERAGE,
        "performance": performance,
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
        "manual_fuzzing_executed": False,
        "limitations": list(LIMITATIONS),
    }


def validate_report(report: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(report, dict):
        return ["runtime.story23.report_type"]
    if report.get("schema_version") != 1 or report.get("artifact_id") != (
        "story-23.4-source-runtime-campaign"
    ):
        failures.append("runtime.story23.report_identity")
    revision = report.get("source_revision")
    if not isinstance(revision, str) or REVISION.fullmatch(revision) is None:
        failures.append("runtime.story23.report_revision")
    if report.get("coverage") != COVERAGE:
        failures.append("runtime.story23.report_coverage")
    failures.extend(performance_failures(report.get("performance")))
    commands = report.get("commands")
    if not isinstance(commands, list) or len(commands) != len(COMMANDS):
        failures.append("runtime.story23.report_commands")
    else:
        for record, (command_id, command, minimum_passed) in zip(
            commands, COMMANDS, strict=True
        ):
            test_result = record.get("test_result")
            valid_test_result = (
                test_result is None
                if minimum_passed == 0
                else isinstance(test_result, dict)
                and test_result.get("passed", 0) >= minimum_passed
                and test_result.get("failed") == 0
            )
            if (
                record.get("command_id") != command_id
                or record.get("argv_sha256") != command_sha256(command)
                or record.get("exit_code") != 0
                or record.get("maximum_elapsed_ms") != MAXIMUM_COMMAND_ELAPSED_MS
                or record.get("maximum_rss_kib") != MAXIMUM_COMMAND_RSS_KIB
                or not isinstance(record.get("elapsed_ms"), int)
                or isinstance(record.get("elapsed_ms"), bool)
                or not 0 < record["elapsed_ms"] <= MAXIMUM_COMMAND_ELAPSED_MS
                or not isinstance(record.get("peak_rss_kib"), int)
                or isinstance(record.get("peak_rss_kib"), bool)
                or not 0 < record["peak_rss_kib"] <= MAXIMUM_COMMAND_RSS_KIB
                or not valid_test_result
            ):
                failures.append("runtime.story23.report_command")
                break
    raw_trace = report.get("raw_trace")
    if (
        not isinstance(raw_trace, dict)
        or raw_trace.get("path") != str(LOG_PATH.relative_to(ROOT))
        or raw_trace.get("redactions") != ["repository-root"]
    ):
        failures.append("runtime.story23.report_trace")
    elif not LOG_PATH.is_file():
        failures.append("runtime.story23.trace_missing")
    else:
        raw = LOG_PATH.read_bytes()
        if raw_trace.get("bytes") != len(raw) or raw_trace.get("sha256") != sha256_bytes(raw):
            failures.append("runtime.story23.trace_drift")
        elif str(ROOT).encode("utf-8") in raw:
            failures.append("runtime.story23.trace_root_disclosure")
        else:
            try:
                parse_performance(raw.decode("utf-8"))
            except (UnicodeError, RuntimeEvidenceError):
                failures.append("runtime.story23.trace_invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(
        SOURCE_PATHS
    ):
        failures.append("runtime.story23.report_sources")
    elif isinstance(revision, str) and REVISION.fullmatch(revision):
        for record in sources:
            content = git_blob(revision, record["path"])
            if (
                record.get("bytes") != len(content)
                or SHA256.fullmatch(str(record.get("sha256", ""))) is None
                or record["sha256"] != sha256_bytes(content)
            ):
                failures.append("runtime.story23.source_drift")
                break
    if report.get("status") != "pass-current-linux-source-runtime":
        failures.append("runtime.story23.report_status")
    for field in (
        "external_network_used",
        "private_user_data_used",
        "manual_fuzzing_executed",
    ):
        if report.get(field) is not False:
            failures.append(f"runtime.story23.{field}")
    if report.get("limitations") != list(LIMITATIONS):
        failures.append("runtime.story23.report_limitations")
    return failures


def read_report() -> Any:
    try:
        return json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise RuntimeEvidenceError("runtime.story23.report_unavailable") from error


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    if arguments.write:
        revision = git_revision(arguments.source_revision)
        output, results, performance = run_campaign()
        atomic_write(LOG_PATH, output.encode("utf-8"))
        atomic_write(
            REPORT_PATH,
            canonical_json_bytes(build_report(revision, output, results, performance)),
        )
    report = read_report()
    failures = validate_report(report)
    if failures:
        raise RuntimeEvidenceError("; ".join(failures))
    print("Story 23.4 source runtime evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
