#!/usr/bin/env python3
"""Run and validate the Story 22.1 current-product canary sweep."""

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
REPORT_PATH: Final = OUTPUT_DIRECTORY / "current-product-canary-sweep.json"
LOG_PATH: Final = OUTPUT_DIRECTORY / "current-product-canary-sweep.log"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
TASK_IDS: Final = [
    "22.1.3.5",
    "SR-DAT-002",
    "SR-DAT-003",
    "SR-AI-008",
    "SR-AI-010",
    "SR-OPS-003",
]
SOURCE_PATHS: Final = (
    "kernel/engine/src/context_management.rs",
    "kernel/engine/src/persistence.rs",
    "kernel/engine/src/operational_store.rs",
    "kernel/engine/src/runtime_projection.rs",
    "kernel/engine/src/runtime_artifact.rs",
    "kernel/engine/src/runtime_loop_tests.rs",
    "platforms/linux/src/runtime_artifact_store.rs",
    "scripts/strict_local_source_audit.py",
    "scripts/story_22_1_canary_evidence.py",
    "tests/test_story_22_1_canary_evidence.py",
)
COMMANDS: Final = (
    (
        "context-admission",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine", "--lib", "--locked",
            "context_management::tests::s020_ut01_denied_stale_and_secret_canary_never_enter_packet_content",
        ),
        "1 passed; 0 failed",
        "s020_ut01_denied_stale_and_secret_canary_never_enter_packet_content",
    ),
    (
        "persistence-policy",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine", "--lib", "--locked",
            "persistence::tests::synthetic_canary_campaign_covers_every_family_and_field_boundary",
        ),
        "1 passed; 0 failed",
        "synthetic_canary_campaign_covers_every_family_and_field_boundary",
    ),
    (
        "encrypted-store-export",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine", "--lib", "--locked",
            "operational_store::tests::synthetic_canary_is_absent_from_encrypted_and_derived_artifacts",
        ),
        "1 passed; 0 failed",
        "synthetic_canary_is_absent_from_encrypted_and_derived_artifacts",
    ),
    (
        "runtime-projections",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine", "--lib", "--locked",
            "runtime_projection::tests::story_21_2_sensitive_canaries_are_excluded_or_restricted_in_every_projection",
        ),
        "1 passed; 0 failed",
        "story_21_2_sensitive_canaries_are_excluded_or_restricted_in_every_projection",
    ),
    (
        "runtime-artifacts",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine", "--lib", "--locked",
            "runtime_artifact::tests::story_21_2_artifact_canaries_require_an_exact_owner_bound_payload_read",
        ),
        "1 passed; 0 failed",
        "story_21_2_artifact_canaries_require_an_exact_owner_bound_payload_read",
    ),
    (
        "runtime-model-tool-events",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine", "--lib", "--locked",
            "runtime_loop_tests::story_23_4_runtime_events_exclude_raw_model_and_tool_canaries",
        ),
        "1 passed; 0 failed",
        "story_23_4_runtime_events_exclude_raw_model_and_tool_canaries",
    ),
    (
        "native-encrypted-payloads",
        (
            "cargo", "test", "-p", "agentmage-platform-linux", "--lib", "--locked",
            "runtime_artifact_store::tests::staging_and_objects_are_encrypted_randomized_and_key_bound",
        ),
        "1 passed; 0 failed",
        "staging_and_objects_are_encrypted_randomized_and_key_bound",
    ),
    (
        "strict-local-telemetry",
        ("python3", "scripts/strict_local_source_audit.py"),
        "zero undeclared network paths",
        "strict local source audit",
    ),
)
COVERAGE: Final = {
    "context-and-checkpoint-input": ["context-admission"],
    "classification-minimization-and-secret-detection": ["persistence-policy"],
    "sqlcipher-backup-export-and-crash-diagnostics": ["encrypted-store-export"],
    "transcript-event-diagnostic-and-metric-projections": ["runtime-projections"],
    "owner-bound-private-artifact-content": ["runtime-artifacts"],
    "model-and-tool-event-content": ["runtime-model-tool-events"],
    "native-staging-and-object-ciphertext": ["native-encrypted-payloads"],
    "external-telemetry-dependency": ["strict-local-telemetry"],
}


class CanaryEvidenceError(ValueError):
    """Raised when current-product canary evidence is unavailable or stale."""


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def command_id(argv: tuple[str, ...]) -> str:
    return sha256_bytes("\0".join(argv).encode("utf-8"))


def git_revision(candidate: str) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"],
        cwd=ROOT, capture_output=True, text=True, timeout=30, check=False,
    )
    revision = result.stdout.strip()
    if result.returncode or REVISION.fullmatch(revision) is None:
        raise CanaryEvidenceError("runtime.canary.source_revision")
    return revision


def git_blob(revision: str, relative: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{relative}"], cwd=ROOT,
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=60, check=False,
    )
    if result.returncode or not result.stdout:
        raise CanaryEvidenceError("runtime.canary.source_unavailable")
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
            "required_test": test_name,
        }
        for identifier, argv, marker, test_name in COMMANDS
    ]


def run_campaign() -> tuple[str, list[dict[str, Any]]]:
    if platform.system() != "Linux":
        raise CanaryEvidenceError("runtime.canary.platform")
    traces: list[str] = []
    records: list[dict[str, Any]] = []
    environment = {
        **os.environ, "CARGO_TERM_COLOR": "never", "LANG": "C", "LC_ALL": "C", "NO_COLOR": "1"
    }
    for identifier, argv, marker, test_name in COMMANDS:
        started = time.monotonic()
        result = subprocess.run(
            list(argv), cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            text=True, timeout=360, check=False, env=environment,
        )
        output = (result.stdout + result.stderr).replace(str(ROOT), "<repository-root>")
        if result.returncode or marker not in output:
            raise CanaryEvidenceError(f"runtime.canary.command_failed.{identifier}")
        if identifier != "strict-local-telemetry" and test_name not in output:
            raise CanaryEvidenceError(f"runtime.canary.test_missing.{identifier}")
        records.append({
            "id": identifier,
            "command_id": command_id(argv),
            "expected_marker": marker,
            "required_test": test_name,
            "elapsed_ms": max(1, int((time.monotonic() - started) * 1_000)),
            "exit_code": 0,
        })
        traces.append(f"===== {identifier} =====\n{output.rstrip()}\n")
    return "\n".join(traces), records


def build_report(revision: str, output: str, commands: list[dict[str, Any]]) -> dict[str, Any]:
    raw = output.encode("utf-8")
    return {
        "schema_version": 1,
        "artifact_id": "story-22.1-current-product-canary-sweep",
        "source_revision": revision,
        "status": "pass-current-linux-source-surface-boundary",
        "task_ids": TASK_IDS,
        "coverage": COVERAGE,
        "commands": commands,
        "raw_canary_values_retained_in_report": 0,
        "external_network_used": False,
        "private_user_data_used": False,
        "raw_trace": {
            "path": str(LOG_PATH.relative_to(ROOT)), "bytes": len(raw),
            "sha256": sha256_bytes(raw), "redactions": ["repository-root"],
        },
        "sources": source_records(revision),
        "limitations": [
            "This sweep composes every currently reachable named context, persistence, runtime projection, artifact, event, native payload, and telemetry source check; future surfaces invalidate it until extended.",
            "Authorized private artifact reads may return exact payload content; the test proves path-free metadata and cross-owner denial, not blanket content deletion.",
            "Deterministic source and byte scans are not process-memory-dump, swap, SSD-remanence, filesystem-snapshot, or external host-instrumentation evidence.",
            "The deterministic fake-model runtime path is active, but an installed real local model and package-level workflow are not exercised here.",
            "Windows, macOS, clean-image Linux, release, and independent-review evidence are not claimed.",
            "Manual fuzzing remains deferred and was not executed by this campaign.",
        ],
    }


def validate_report(report: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(report, dict):
        return ["runtime.canary.report_type"]
    if report.get("schema_version") != 1 or report.get("artifact_id") != "story-22.1-current-product-canary-sweep":
        failures.append("runtime.canary.report_identity")
    revision = report.get("source_revision")
    if not isinstance(revision, str) or REVISION.fullmatch(revision) is None:
        failures.append("runtime.canary.report_revision")
    if report.get("status") != "pass-current-linux-source-surface-boundary" or report.get("coverage") != COVERAGE:
        failures.append("runtime.canary.report_disposition")
    if report.get("task_ids") != TASK_IDS:
        failures.append("runtime.canary.report_task_ids")
    commands = report.get("commands")
    if not isinstance(commands, list) or len(commands) != len(COMMANDS):
        failures.append("runtime.canary.report_commands")
    else:
        for record, expected in zip(commands, expected_command_records(), strict=True):
            if any(record.get(key) != value for key, value in expected.items()) or record.get("exit_code") != 0 or not isinstance(record.get("elapsed_ms"), int) or record["elapsed_ms"] <= 0:
                failures.append(f"runtime.canary.command.{expected['id']}")
    if report.get("raw_canary_values_retained_in_report") != 0:
        failures.append("runtime.canary.raw_value_claim")
    raw_trace = report.get("raw_trace")
    if not isinstance(raw_trace, dict) or raw_trace.get("path") != str(LOG_PATH.relative_to(ROOT)) or raw_trace.get("redactions") != ["repository-root"]:
        failures.append("runtime.canary.report_trace")
    elif not LOG_PATH.is_file():
        failures.append("runtime.canary.trace_missing")
    else:
        raw = LOG_PATH.read_bytes()
        if raw_trace.get("bytes") != len(raw) or raw_trace.get("sha256") != sha256_bytes(raw):
            failures.append("runtime.canary.trace_drift")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(SOURCE_PATHS):
        failures.append("runtime.canary.report_sources")
    elif isinstance(revision, str) and REVISION.fullmatch(revision):
        for record in sources:
            content = git_blob(revision, record["path"])
            if record.get("bytes") != len(content) or SHA256.fullmatch(str(record.get("sha256", ""))) is None or record["sha256"] != sha256_bytes(content):
                failures.append("runtime.canary.source_drift")
                break
    for field in ("external_network_used", "private_user_data_used"):
        if report.get(field) is not False:
            failures.append(f"runtime.canary.{field}")
    if not isinstance(report.get("limitations"), list) or len(report["limitations"]) != 6:
        failures.append("runtime.canary.report_limitations")
    return failures


def read_report() -> Any:
    try:
        return json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise CanaryEvidenceError("runtime.canary.report_unavailable") from error


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    if arguments.write:
        revision = git_revision(arguments.source_revision)
        output, commands = run_campaign()
        atomic_write(LOG_PATH, output.encode("utf-8"))
        atomic_write(REPORT_PATH, canonical_json_bytes(build_report(revision, output, commands)))
    failures = validate_report(read_report())
    if failures:
        raise CanaryEvidenceError("; ".join(failures))
    print("Story 22.1 current-product canary evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
