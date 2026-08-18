#!/usr/bin/env python3
"""Run and validate the native Story 22.2 artifact-resume campaign."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
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
OUTPUT_DIRECTORY: Final = ROOT / "artifacts/sprints/sprint-22/story-22.2"
REPORT_PATH: Final = OUTPUT_DIRECTORY / "native-artifact-resume.json"
LOG_PATH: Final = OUTPUT_DIRECTORY / "native-artifact-resume.log"
METRIC_PREFIX: Final = "AGENTMAGE_ARTIFACT_RESUME="
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
SOURCE_PATHS: Final = (
    "shells/host/src/linux_coding_runtime.rs",
    "scripts/story_22_2_artifact_resume_evidence.py",
    "tests/test_story_22_2_artifact_resume_evidence.py",
)
TEST_NAMES: Final = (
    "story_22_2_linux_restart_recovers_large_terminal_model_artifact",
    "story_22_2_linux_restart_restores_large_command_and_test_artifacts_without_replay",
    "story_22_2_linux_restart_restores_checkpoint_without_replaying_the_tool",
    "story_22_2_linux_restart_rejects_lost_continuation_without_replaying_the_tool",
)
COMMAND: Final = (
    "cargo",
    "test",
    "-p",
    "agentmage-host",
    "--lib",
    "--locked",
    "story_22_2_linux_restart_",
    "--",
    "--nocapture",
)


class ArtifactResumeEvidenceError(ValueError):
    """Raised when artifact-resume evidence is unavailable, malformed, or stale."""


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def expected_metrics() -> dict[str, dict[str, Any]]:
    return {
        "continuation-integrity-loss": {
            "active_continuation_objects_after_reconcile": 0,
            "cases": ["missing", "corrupt"],
            "cases_blocked": 2,
            "operator_cleanup": "blocked",
            "operator_integrity_by_case": {
                "corrupt": "corrupt",
                "missing": "missing",
            },
            "operator_lifecycle": "quarantined",
            "post_failure_total_tool_executions_per_case": 1,
            "pre_restart_tool_executions_per_case": 1,
            "scenario": "continuation-integrity-loss",
        },
        "exact-checkpoint-resume": {
            "artifact_set": "exact",
            "model_runtime_drift": "blocked",
            "policy_drift": "blocked",
            "post_resume_receipts": 1,
            "post_resume_total_tool_executions": 1,
            "pre_restart_receipts": 1,
            "pre_restart_tool_executions": 1,
            "repository_drift": "blocked",
            "scenario": "exact-checkpoint-resume",
            "terminal_state": "no_op",
        },
        "large-command-test-resume": {
            "artifact_kinds": ["standard_output", "test_log"],
            "artifact_payload_bytes_each": 70 * 1024,
            "exact_artifact_set_restored": True,
            "external_network_used": False,
            "manual_fuzzing_executed": False,
            "post_resume_command_executions": 2,
            "pre_restart_command_executions": 2,
            "scenario": "large-command-test-resume",
            "terminal_state": "no_op",
            "tool_call_count": 3,
        },
        "large-model-terminal-restart": {
            "artifact_kind": "model_output",
            "artifact_payload_bytes": 70 * 1024,
            "external_network_used": False,
            "manual_fuzzing_executed": False,
            "payload_recovered_exactly": True,
            "scenario": "large-model-terminal-restart",
            "terminal_state": "blocked",
        },
    }


def parse_metrics(output: str) -> dict[str, dict[str, Any]]:
    records: dict[str, dict[str, Any]] = {}
    for line in output.splitlines():
        if not line.startswith(METRIC_PREFIX):
            continue
        try:
            metric = json.loads(line.removeprefix(METRIC_PREFIX))
        except json.JSONDecodeError as error:
            raise ArtifactResumeEvidenceError(
                "runtime.artifact_resume.metric_json"
            ) from error
        if not isinstance(metric, dict) or not isinstance(metric.get("scenario"), str):
            raise ArtifactResumeEvidenceError("runtime.artifact_resume.metric_shape")
        scenario = metric["scenario"]
        if scenario in records:
            raise ArtifactResumeEvidenceError("runtime.artifact_resume.metric_duplicate")
        records[scenario] = metric
    if records != expected_metrics():
        raise ArtifactResumeEvidenceError("runtime.artifact_resume.metric_drift")
    return records


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
        raise ArtifactResumeEvidenceError("runtime.artifact_resume.source_revision")
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
        raise ArtifactResumeEvidenceError("runtime.artifact_resume.source_unavailable")
    return result.stdout


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


def run_campaign() -> tuple[str, int]:
    if platform.system() != "Linux":
        raise ArtifactResumeEvidenceError("runtime.artifact_resume.platform")
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
    if result.returncode or "4 passed; 0 failed" not in output:
        raise ArtifactResumeEvidenceError("runtime.artifact_resume.command_failed")
    for test_name in TEST_NAMES:
        if f"{test_name} ... ok" not in output:
            raise ArtifactResumeEvidenceError("runtime.artifact_resume.test_missing")
    parse_metrics(output)
    elapsed_ms = max(1, int((time.monotonic() - started) * 1_000))
    return output, elapsed_ms


def build_report(revision: str, output: str, elapsed_ms: int) -> dict[str, Any]:
    raw = output.encode("utf-8")
    return {
        "schema_version": 1,
        "artifact_id": "story-22.2-native-runtime-artifact-resume",
        "source_revision": revision,
        "status": "partial-pass-current-linux-native-source-boundary",
        "task_ids": ["22.2.3.3", "RV-17", "RV-18"],
        "host": {
            "system": platform.system(),
            "machine": platform.machine(),
        },
        "tests": list(TEST_NAMES),
        "metrics": parse_metrics(output),
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
            "The campaign uses the production coordinator and native encrypted Linux authority with a deterministic fake model and instrumented Git executor; it is not installed-package or real-model evidence.",
            "Exact continuation and artifact sets resume after protected Git, large command, and large validation operations; large terminal model output is recovered exactly after restart without being represented as resumable work.",
            "Missing and corrupt continuation payloads are reconciled at startup, remain visible through a path-free quarantined/blocked operator view, and block resume explicitly; long/concurrent collection and physical storage failure are not claimed.",
            "This current-host Linux source campaign is not Windows, macOS, release, or independent-review evidence.",
            "The four focused tests are not the separate one-hundred-run native tool-terminal campaign owned by Story 22.1.",
            "Manual fuzzing remains deferred and was not executed by this campaign.",
        ],
    }


def validate_report(report: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(report, dict):
        return ["runtime.artifact_resume.report_type"]
    if report.get("schema_version") != 1 or report.get("artifact_id") != (
        "story-22.2-native-runtime-artifact-resume"
    ):
        failures.append("runtime.artifact_resume.report_identity")
    revision = report.get("source_revision")
    if not isinstance(revision, str) or REVISION.fullmatch(revision) is None:
        failures.append("runtime.artifact_resume.report_revision")
    if report.get("status") != "partial-pass-current-linux-native-source-boundary" or report.get(
        "task_ids"
    ) != ["22.2.3.3", "RV-17", "RV-18"]:
        failures.append("runtime.artifact_resume.report_disposition")
    host = report.get("host")
    if not isinstance(host, dict) or host.get("system") != "Linux" or not isinstance(
        host.get("machine"), str
    ):
        failures.append("runtime.artifact_resume.report_host")
    if report.get("tests") != list(TEST_NAMES) or report.get("metrics") != expected_metrics():
        failures.append("runtime.artifact_resume.report_results")
    verification = report.get("verification")
    if not isinstance(verification, dict) or verification.get("command_id") != sha256_bytes(
        "\0".join(COMMAND).encode("utf-8")
    ):
        failures.append("runtime.artifact_resume.report_command")
    elif (
        verification.get("exit_code") != 0
        or not isinstance(verification.get("elapsed_ms"), int)
        or verification["elapsed_ms"] <= 0
    ):
        failures.append("runtime.artifact_resume.report_result")
    raw_trace = report.get("raw_trace")
    if (
        not isinstance(raw_trace, dict)
        or raw_trace.get("path") != str(LOG_PATH.relative_to(ROOT))
        or raw_trace.get("redactions") != ["repository-root"]
    ):
        failures.append("runtime.artifact_resume.report_trace")
    elif not LOG_PATH.is_file():
        failures.append("runtime.artifact_resume.trace_missing")
    else:
        raw = LOG_PATH.read_bytes()
        if raw_trace.get("bytes") != len(raw) or raw_trace.get("sha256") != sha256_bytes(raw):
            failures.append("runtime.artifact_resume.trace_drift")
        else:
            try:
                parse_metrics(raw.decode("utf-8"))
            except (UnicodeError, ArtifactResumeEvidenceError):
                failures.append("runtime.artifact_resume.trace_invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(
        SOURCE_PATHS
    ):
        failures.append("runtime.artifact_resume.report_sources")
    elif isinstance(revision, str) and REVISION.fullmatch(revision):
        for record in sources:
            content = git_blob(revision, record["path"])
            if (
                record.get("bytes") != len(content)
                or SHA256.fullmatch(str(record.get("sha256", ""))) is None
                or record["sha256"] != sha256_bytes(content)
            ):
                failures.append("runtime.artifact_resume.source_drift")
                break
    for field in ("external_network_used", "private_user_data_used"):
        if report.get(field) is not False:
            failures.append(f"runtime.artifact_resume.{field}")
    limitations = report.get("limitations")
    if not isinstance(limitations, list) or len(limitations) != 6:
        failures.append("runtime.artifact_resume.report_limitations")
    return failures


def read_report() -> Any:
    try:
        return json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise ArtifactResumeEvidenceError(
            "runtime.artifact_resume.report_unavailable"
        ) from error


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    if arguments.write:
        revision = git_revision(arguments.source_revision)
        output, elapsed_ms = run_campaign()
        atomic_write(LOG_PATH, output.encode("utf-8"))
        atomic_write(
            REPORT_PATH,
            canonical_json_bytes(build_report(revision, output, elapsed_ms)),
        )
    failures = validate_report(read_report())
    if failures:
        raise ArtifactResumeEvidenceError("; ".join(failures))
    print("Story 22.2 native artifact resume evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
