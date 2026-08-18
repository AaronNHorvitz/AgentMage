#!/usr/bin/env python3
"""Build and validate the Sprint 15 diagnostic canary matrix."""

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
OUTPUT_DIRECTORY: Final = ROOT / "artifacts/sprints/sprint-15/story-15.2"
REPORT_PATH: Final = OUTPUT_DIRECTORY / "diagnostic-canary-matrix.json"
LOG_PATH: Final = OUTPUT_DIRECTORY / "diagnostic-canary-matrix.log"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
CANARY_PREFIX: Final = "AM-S15-CANARY-"
SOURCE_FAMILIES: Final = (
    "prompt",
    "file_content",
    "credential",
    "private_key",
    "environment_value",
    "absolute_path",
    "hostname",
    "username",
    "device_identifier",
)
SOURCE_PATHS: Final = (
    "kernel/contracts/src/diagnostics.rs",
    "kernel/engine/src/diagnostics.rs",
    "shells/host/src/diagnostic_export.rs",
    "shells/vscode/src/host_bridge.ts",
    "shells/vscode/src/provider.ts",
    "shells/vscode/test/host_bridge.test.ts",
    "shells/vscode/test/provider.test.ts",
    "scripts/sprint_15_diagnostic_canary_evidence.py",
    "tests/test_sprint_15_diagnostic_canary_evidence.py",
    "docs/verification/sprint-15-diagnostic-canary-matrix.md",
)
COMMANDS: Final = (
    (
        "closed-kernel-harness",
        (
            "cargo",
            "test",
            "-p",
            "agentmage-kernel-engine",
            "prohibited_raw_sources_are_not_members_of_the_closed_observation_schema",
            "--locked",
        ),
        "1 passed; 0 failed",
    ),
    (
        "preview-receipt-export",
        (
            "cargo",
            "test",
            "-p",
            "agentmage-host",
            "prohibited_source_canaries_never_reach_preview_receipt_or_export",
            "--locked",
        ),
        "1 passed; 0 failed",
    ),
    (
        "authenticated-chat-and-renderer",
        ("npm", "--prefix", "shells/vscode", "test"),
        "fail 0",
    ),
)
COVERAGE: Final = {
    "internal-harness": ["closed-kernel-harness"],
    "native-chat": ["authenticated-chat-and-renderer"],
    "authenticated-host-transport": ["authenticated-chat-and-renderer"],
    "command-output-log": [identifier for identifier, _, _ in COMMANDS],
    "preview": ["preview-receipt-export"],
    "receipt": ["preview-receipt-export"],
    "exported-diagnostics": ["preview-receipt-export"],
}
SEMANTIC_RECONCILIATION: Final = {
    "kernel_report_equal_after_rejection": True,
    "exported_report_equal_to_safe_report": True,
    "unknown_transport_fields_fail_closed": True,
    "chat_disclosure_count": 0,
    "harness_disclosure_count": 0,
    "log_disclosure_count": 0,
    "preview_disclosure_count": 0,
    "receipt_disclosure_count": 0,
    "export_disclosure_count": 0,
}


class DiagnosticCanaryEvidenceError(ValueError):
    """Raised when diagnostic canary evidence is absent, stale, or overstated."""


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def command_id(argv: tuple[str, ...]) -> str:
    return sha256_bytes("\0".join(argv).encode("utf-8"))


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
        raise DiagnosticCanaryEvidenceError("diagnostic.canary.source_revision")
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
        raise DiagnosticCanaryEvidenceError("diagnostic.canary.source_unavailable")
    return result.stdout


def source_records(revision: str) -> list[dict[str, Any]]:
    return [
        {"path": path, "bytes": len(content), "sha256": sha256_bytes(content)}
        for path in SOURCE_PATHS
        for content in [git_blob(revision, path)]
    ]


def expected_command_records() -> list[dict[str, Any]]:
    return [
        {
            "id": identifier,
            "command_id": command_id(argv),
            "expected_marker": marker,
        }
        for identifier, argv, marker in COMMANDS
    ]


def run_campaign() -> tuple[bytes, list[dict[str, Any]]]:
    if platform.system() != "Linux":
        raise DiagnosticCanaryEvidenceError("diagnostic.canary.platform")
    environment = {
        **os.environ,
        "CARGO_TERM_COLOR": "never",
        "LANG": "C",
        "LC_ALL": "C",
        "NO_COLOR": "1",
    }
    traces: list[str] = []
    records: list[dict[str, Any]] = []
    for identifier, argv, marker in COMMANDS:
        started = time.monotonic()
        result = subprocess.run(
            list(argv),
            cwd=ROOT,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=900,
            check=False,
            env=environment,
        )
        output = (result.stdout + result.stderr).replace(
            str(ROOT).encode("utf-8"), b"<repository-root>"
        )
        if result.returncode or marker.encode("utf-8") not in output:
            raise DiagnosticCanaryEvidenceError(
                f"diagnostic.canary.command_failed.{identifier}"
            )
        if CANARY_PREFIX.encode("utf-8") in output:
            raise DiagnosticCanaryEvidenceError(
                f"diagnostic.canary.command_disclosed.{identifier}"
            )
        records.append(
            {
                "id": identifier,
                "command_id": command_id(argv),
                "expected_marker": marker,
                "elapsed_ms": max(1, int((time.monotonic() - started) * 1_000)),
                "exit_code": 0,
                "output_bytes": len(output),
                "output_sha256": sha256_bytes(output),
                "canary_match_count": 0,
            }
        )
        traces.append(f"===== {identifier} =====\n{output.decode('utf-8', errors='replace').rstrip()}\n")
    return "\n".join(traces).encode("utf-8"), records


def build_report(
    revision: str, log_bytes: bytes, commands: list[dict[str, Any]]
) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "artifact_id": "sprint-15-diagnostic-canary-matrix",
        "source_revision": revision,
        "status": "pass-current-linux-diagnostic-surfaces",
        "task_ids": ["15.2.2.2", "15.2.AC2"],
        "prohibited_source_families": list(SOURCE_FAMILIES),
        "canary_prefix_sha256": sha256_bytes(CANARY_PREFIX.encode("utf-8")),
        "coverage": COVERAGE,
        "commands": commands,
        "semantic_reconciliation": SEMANTIC_RECONCILIATION,
        "external_network_used": False,
        "private_user_data_used": False,
        "raw_canary_values_retained_in_report": 0,
        "log": {
            "path": str(LOG_PATH.relative_to(ROOT)),
            "bytes": len(log_bytes),
            "sha256": sha256_bytes(log_bytes),
            "repository_path_redacted": True,
        },
        "sources": source_records(revision),
        "limitations": [
            "The campaign proves the current closed Linux diagnostic schema, internal harness, authenticated host transport, native Chat renderer, command output, preview, receipt, and reviewed export surfaces only.",
            "The raw prohibited values remain test-owned and cannot enter the production DiagnosticObservation schema; unknown transport fields fail closed rather than being silently retained.",
            "No production diagnostic logging sink exists in this path; the retained command output is scanned as the currently reachable log surface.",
            "Native keyboard and screen-reader behavior, macOS, Windows, installed packages, independent review, process-memory inspection, and storage remanence are not claimed.",
            "Manual fuzzing remains deferred and was not executed.",
        ],
    }


def validate_report(report: Any, *, verify_sources: bool = True) -> list[str]:
    failures: list[str] = []
    if not isinstance(report, dict):
        return ["diagnostic.canary.report_type"]
    if (
        report.get("schema_version") != 1
        or report.get("artifact_id") != "sprint-15-diagnostic-canary-matrix"
    ):
        failures.append("diagnostic.canary.report_identity")
    revision = report.get("source_revision")
    if not isinstance(revision, str) or REVISION.fullmatch(revision) is None:
        failures.append("diagnostic.canary.report_revision")
    if (
        report.get("status") != "pass-current-linux-diagnostic-surfaces"
        or report.get("task_ids") != ["15.2.2.2", "15.2.AC2"]
        or report.get("coverage") != COVERAGE
        or report.get("prohibited_source_families") != list(SOURCE_FAMILIES)
    ):
        failures.append("diagnostic.canary.report_scope")
    if report.get("canary_prefix_sha256") != sha256_bytes(CANARY_PREFIX.encode("utf-8")):
        failures.append("diagnostic.canary.prefix")
    expected = expected_command_records()
    commands = report.get("commands")
    if not isinstance(commands, list) or len(commands) != len(expected):
        failures.append("diagnostic.canary.commands")
    else:
        for actual, planned in zip(commands, expected, strict=True):
            if any(actual.get(key) != value for key, value in planned.items()):
                failures.append(f"diagnostic.canary.command.{planned['id']}")
            if (
                actual.get("exit_code") != 0
                or actual.get("canary_match_count") != 0
                or not isinstance(actual.get("output_bytes"), int)
                or actual.get("output_bytes", 0) <= 0
                or not isinstance(actual.get("elapsed_ms"), int)
                or actual.get("elapsed_ms", 0) <= 0
                or SHA256.fullmatch(str(actual.get("output_sha256", ""))) is None
            ):
                failures.append(f"diagnostic.canary.result.{planned['id']}")
    if report.get("semantic_reconciliation") != SEMANTIC_RECONCILIATION:
        failures.append("diagnostic.canary.semantic_reconciliation")
    if (
        report.get("external_network_used") is not False
        or report.get("private_user_data_used") is not False
        or report.get("raw_canary_values_retained_in_report") != 0
    ):
        failures.append("diagnostic.canary.disclosure_claim")
    log = report.get("log")
    if not isinstance(log, dict) or log.get("path") != str(LOG_PATH.relative_to(ROOT)):
        failures.append("diagnostic.canary.log")
    elif verify_sources and LOG_PATH.is_file():
        log_bytes = LOG_PATH.read_bytes()
        if (
            log.get("bytes") != len(log_bytes)
            or log.get("sha256") != sha256_bytes(log_bytes)
            or CANARY_PREFIX.encode("utf-8") in log_bytes
        ):
            failures.append("diagnostic.canary.log_integrity")
    elif verify_sources:
        failures.append("diagnostic.canary.log_missing")
    sources = report.get("sources")
    if verify_sources and isinstance(revision, str) and REVISION.fullmatch(revision):
        try:
            if sources != source_records(revision):
                failures.append("diagnostic.canary.sources")
        except DiagnosticCanaryEvidenceError:
            failures.append("diagnostic.canary.sources")
    elif not isinstance(sources, list) or len(sources) != len(SOURCE_PATHS):
        failures.append("diagnostic.canary.sources")
    limitations = report.get("limitations")
    if not isinstance(limitations, list) or len(limitations) < 5:
        failures.append("diagnostic.canary.limitations")
    return sorted(set(failures))


def read_report() -> dict[str, Any]:
    value = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise DiagnosticCanaryEvidenceError("diagnostic.canary.report_type")
    return value


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    if arguments.check:
        report = read_report()
    else:
        revision = git_revision(arguments.source_revision)
        log_bytes, commands = run_campaign()
        OUTPUT_DIRECTORY.mkdir(parents=True, exist_ok=True)
        atomic_write(LOG_PATH, log_bytes, mode=0o600)
        report = build_report(revision, log_bytes, commands)
        atomic_write(REPORT_PATH, canonical_json_bytes(report))
    failures = validate_report(report)
    if failures:
        for failure in failures:
            print(failure)
        return 1
    print("Sprint 15 diagnostic canary evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
