#!/usr/bin/env python3
"""Run and validate the Story 22.1 native tool-terminal resume matrix."""

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
OUTPUT_DIRECTORY: Final = ROOT / "artifacts/sprints/sprint-22/story-22.1"
REPORT_PATH: Final = OUTPUT_DIRECTORY / "native-tool-terminal-resume.json"
LOG_PATH: Final = OUTPUT_DIRECTORY / "native-tool-terminal-resume.log"
METRIC_PREFIX: Final = "AGENTMAGE_STORY_22_1_RESUME="
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
SOURCE_PATHS: Final = (
    "shells/host/src/linux_coding_runtime.rs",
    "scripts/story_22_1_native_resume_evidence.py",
    "tests/test_story_22_1_native_resume_evidence.py",
)
COMMAND: Final = (
    "cargo",
    "test",
    "-p",
    "agentmage-host",
    "--lib",
    "--locked",
    "linux_coding_runtime::tests::story_22_1_native_tool_terminal_resume_matrix_never_replays_or_invents_state",
    "--",
    "--exact",
    "--ignored",
    "--nocapture",
    "--test-threads=1",
)
EXPECTED_METRIC: Final = {
    "boundaries": [
        "before-tool-terminal-commit",
        "after-tool-terminal-commit",
        "before-checkpoint-commit",
        "after-checkpoint-commit",
    ],
    "case_count": 100,
    "checkpoint_after_commit_cases": 25,
    "external_network_used": False,
    "false_checkpoint_count": 0,
    "false_terminal_count": 0,
    "manual_fuzzing_executed": False,
    "repetitions_per_boundary": 25,
    "total_worker_launches_per_case": 1,
}


class NativeResumeEvidenceError(ValueError):
    """Raised when native resume evidence is unavailable, malformed, or stale."""


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def parse_metric(output: str) -> dict[str, Any]:
    records = [
        line.split(METRIC_PREFIX, 1)[1]
        for line in output.splitlines()
        if METRIC_PREFIX in line
    ]
    if len(records) != 1:
        raise NativeResumeEvidenceError("runtime.native_resume.metric_count")
    try:
        metric = json.loads(records[0])
    except json.JSONDecodeError as error:
        raise NativeResumeEvidenceError("runtime.native_resume.metric_json") from error
    if metric != EXPECTED_METRIC:
        raise NativeResumeEvidenceError("runtime.native_resume.metric_drift")
    return metric


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
        raise NativeResumeEvidenceError("runtime.native_resume.source_revision")
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
        raise NativeResumeEvidenceError("runtime.native_resume.source_unavailable")
    return result.stdout


def source_records(revision: str) -> list[dict[str, Any]]:
    return [
        {"path": relative, "bytes": len(content), "sha256": sha256_bytes(content)}
        for relative in SOURCE_PATHS
        for content in [git_blob(revision, relative)]
    ]


def run_campaign() -> tuple[str, int]:
    if platform.system() != "Linux":
        raise NativeResumeEvidenceError("runtime.native_resume.platform")
    started = time.monotonic()
    result = subprocess.run(
        list(COMMAND),
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        timeout=600,
        check=False,
        env={
            **os.environ,
            "CARGO_TERM_COLOR": "never",
            "LANG": "C",
            "LC_ALL": "C",
            "NO_COLOR": "1",
        },
    )
    output = (result.stdout + result.stderr).replace(str(ROOT), "<repository-root>")
    if result.returncode or "1 passed; 0 failed" not in output:
        raise NativeResumeEvidenceError("runtime.native_resume.command_failed")
    parse_metric(output)
    return output, max(1, int((time.monotonic() - started) * 1_000))


def build_report(revision: str, output: str, elapsed_ms: int) -> dict[str, Any]:
    raw = output.encode("utf-8")
    return {
        "schema_version": 1,
        "artifact_id": "story-22.1-native-tool-terminal-resume",
        "source_revision": revision,
        "status": "pass-current-linux-native-source-boundary",
        "task_ids": ["22.1.3.3", "AT-CRASH-001", "AT-RESUME-001"],
        "host": {"system": platform.system(), "machine": platform.machine()},
        "metrics": parse_metric(output),
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
            "The campaign uses the production Linux coordinator and encrypted authority with a deterministic fake model and Git executor; it is not real-model or installed-package evidence.",
            "No-unwind exits occur immediately before and after the declared tool-terminal event and checkpoint transactions, not inside a filesystem or SQLite syscall.",
            "The durable launch counter proves one worker execution in every case; the campaign does not claim physical power-loss or storage-controller fault coverage.",
            "Long-session model/runtime/configuration/repository drift remains separate Story 22.1 integration work.",
            "Windows, macOS, release, and independent-review evidence are not claimed.",
            "Manual fuzzing remains deferred and was not executed by this campaign.",
        ],
    }


def validate_report(report: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(report, dict):
        return ["runtime.native_resume.report_type"]
    if report.get("schema_version") != 1 or report.get("artifact_id") != (
        "story-22.1-native-tool-terminal-resume"
    ):
        failures.append("runtime.native_resume.report_identity")
    revision = report.get("source_revision")
    if not isinstance(revision, str) or REVISION.fullmatch(revision) is None:
        failures.append("runtime.native_resume.report_revision")
    if report.get("status") != "pass-current-linux-native-source-boundary" or report.get(
        "task_ids"
    ) != ["22.1.3.3", "AT-CRASH-001", "AT-RESUME-001"]:
        failures.append("runtime.native_resume.report_disposition")
    if report.get("metrics") != EXPECTED_METRIC:
        failures.append("runtime.native_resume.report_metrics")
    verification = report.get("verification")
    if not isinstance(verification, dict) or verification.get("command_id") != sha256_bytes(
        "\0".join(COMMAND).encode("utf-8")
    ):
        failures.append("runtime.native_resume.report_command")
    elif (
        verification.get("exit_code") != 0
        or not isinstance(verification.get("elapsed_ms"), int)
        or verification["elapsed_ms"] <= 0
        or verification["elapsed_ms"] > 600_000
    ):
        failures.append("runtime.native_resume.report_result")
    raw_trace = report.get("raw_trace")
    if (
        not isinstance(raw_trace, dict)
        or raw_trace.get("path") != str(LOG_PATH.relative_to(ROOT))
        or raw_trace.get("redactions") != ["repository-root"]
    ):
        failures.append("runtime.native_resume.report_trace")
    elif not LOG_PATH.is_file():
        failures.append("runtime.native_resume.trace_missing")
    else:
        raw = LOG_PATH.read_bytes()
        if raw_trace.get("bytes") != len(raw) or raw_trace.get("sha256") != sha256_bytes(raw):
            failures.append("runtime.native_resume.trace_drift")
        else:
            try:
                parse_metric(raw.decode("utf-8"))
            except (UnicodeError, NativeResumeEvidenceError):
                failures.append("runtime.native_resume.trace_invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(
        SOURCE_PATHS
    ):
        failures.append("runtime.native_resume.report_sources")
    elif isinstance(revision, str) and REVISION.fullmatch(revision):
        for record in sources:
            content = git_blob(revision, record["path"])
            if (
                record.get("bytes") != len(content)
                or SHA256.fullmatch(str(record.get("sha256", ""))) is None
                or record["sha256"] != sha256_bytes(content)
            ):
                failures.append("runtime.native_resume.source_drift")
                break
    for field in ("external_network_used", "private_user_data_used"):
        if report.get(field) is not False:
            failures.append(f"runtime.native_resume.{field}")
    limitations = report.get("limitations")
    if not isinstance(limitations, list) or len(limitations) != 6:
        failures.append("runtime.native_resume.report_limitations")
    return failures


def read_report() -> Any:
    try:
        return json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise NativeResumeEvidenceError("runtime.native_resume.report_unavailable") from error


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    if arguments.write:
        revision = git_revision(arguments.source_revision)
        output, elapsed_ms = run_campaign()
        atomic_write(LOG_PATH, output.encode("utf-8"))
        atomic_write(REPORT_PATH, canonical_json_bytes(build_report(revision, output, elapsed_ms)))
    failures = validate_report(read_report())
    if failures:
        raise NativeResumeEvidenceError("; ".join(failures))
    print("Story 22.1 native tool-terminal resume evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
