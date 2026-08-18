#!/usr/bin/env python3
"""Run and validate the native Story 22.2 artifact ceiling campaign."""

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
OUTPUT_DIRECTORY: Final = ROOT / "artifacts/sprints/sprint-22/story-22.2"
REPORT_PATH: Final = OUTPUT_DIRECTORY / "native-artifact-pressure.json"
LOG_PATH: Final = OUTPUT_DIRECTORY / "native-artifact-pressure.log"
METRIC_PREFIX: Final = "AGENTMAGE_ARTIFACT_PRESSURE="
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
SOURCE_PATHS: Final = (
    "platforms/linux/src/runtime_artifact_store.rs",
    "scripts/story_22_2_artifact_pressure_evidence.py",
    "tests/test_story_22_2_artifact_pressure_evidence.py",
)
COMMAND: Final = (
    "cargo",
    "test",
    "-p",
    "agentmage-platform-linux",
    "--lib",
    "--locked",
    "runtime_artifact_store::tests::story_22_2_native_artifact_pressure_reaches_declared_ceilings",
    "--",
    "--exact",
    "--ignored",
    "--nocapture",
)
FIXED_METRICS: Final = {
    "maximum_payload_bytes": 64 * 1024 * 1024,
    "page_bytes": 4 * 1024,
    "mixed_object_count": 64,
    "mixed_total_payload_bytes": sum(index * 1024 for index in range(1, 65)),
    "mixed_final_active_object_count": 0,
    "checkpoint_reference_count": 1_024,
    "overflow_reference_count_rejected": 1_025,
    "deduplicated_reference_count": 1_023,
    "active_object_count_at_checkpoint": 1,
    "final_active_object_count": 0,
    "latency_ceiling_ms": 300_000,
    "resident_delta_ceiling_kib": 512 * 1_024,
    "retained_disk_ceiling_bytes": 128 * 1_024 * 1_024,
    "checkpoint_release_blocked": True,
    "final_reopen_verified": True,
    "external_network_used": False,
    "manual_fuzzing_executed": False,
}
ELAPSED_FIELDS: Final = (
    "maximum_elapsed_ms",
    "mixed_publish_elapsed_ms",
    "mixed_collection_elapsed_ms",
    "reference_elapsed_ms",
    "reopen_elapsed_ms",
    "collection_elapsed_ms",
    "total_elapsed_ms",
)
DISK_FIELDS: Final = (
    "maximum_disk_bytes",
    "mixed_disk_bytes",
    "reference_disk_bytes",
    "final_disk_bytes",
)


class ArtifactPressureEvidenceError(ValueError):
    """Raised when artifact pressure evidence is unavailable, malformed, or stale."""


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
        raise ArtifactPressureEvidenceError("runtime.artifact_pressure.source_revision")
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
        raise ArtifactPressureEvidenceError("runtime.artifact_pressure.source_unavailable")
    return result.stdout


def validate_metrics(metrics: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(metrics, dict):
        return ["runtime.artifact_pressure.metric_type"]
    expected_fields = (
        set(FIXED_METRICS)
        | set(ELAPSED_FIELDS)
        | set(DISK_FIELDS)
        | {"resident_start_kib", "resident_peak_kib", "resident_delta_kib"}
    )
    if set(metrics) != expected_fields:
        failures.append("runtime.artifact_pressure.metric_fields")
        return failures
    for field, expected in FIXED_METRICS.items():
        if metrics.get(field) != expected:
            failures.append(f"runtime.artifact_pressure.{field}")
    ceiling = FIXED_METRICS["latency_ceiling_ms"]
    for field in ELAPSED_FIELDS:
        value = metrics.get(field)
        if not isinstance(value, int) or isinstance(value, bool) or value <= 0 or value > ceiling:
            failures.append(f"runtime.artifact_pressure.{field}")
    if all(isinstance(metrics.get(field), int) for field in ELAPSED_FIELDS):
        if metrics["total_elapsed_ms"] < max(metrics[field] for field in ELAPSED_FIELDS[:-1]):
            failures.append("runtime.artifact_pressure.elapsed_relationship")
    disk_ceiling = FIXED_METRICS["retained_disk_ceiling_bytes"]
    for field in DISK_FIELDS:
        value = metrics.get(field)
        if not isinstance(value, int) or isinstance(value, bool) or value <= 0 or value > disk_ceiling:
            failures.append(f"runtime.artifact_pressure.{field}")
    maximum_disk = metrics.get("maximum_disk_bytes")
    if isinstance(maximum_disk, int) and maximum_disk < FIXED_METRICS["maximum_payload_bytes"]:
        failures.append("runtime.artifact_pressure.maximum_disk_underreported")
    resident_start = metrics.get("resident_start_kib")
    resident_peak = metrics.get("resident_peak_kib")
    resident_delta = metrics.get("resident_delta_kib")
    if not all(
        isinstance(value, int) and not isinstance(value, bool) and value > 0
        for value in (resident_start, resident_peak)
    ) or not isinstance(resident_delta, int) or isinstance(resident_delta, bool):
        failures.append("runtime.artifact_pressure.resident_metrics")
    elif resident_peak < resident_start or resident_delta != resident_peak - resident_start:
        failures.append("runtime.artifact_pressure.resident_relationship")
    elif resident_delta > FIXED_METRICS["resident_delta_ceiling_kib"]:
        failures.append("runtime.artifact_pressure.resident_ceiling")
    return failures


def parse_metrics(output: str) -> dict[str, Any]:
    records = [
        line.removeprefix(METRIC_PREFIX)
        for line in output.splitlines()
        if line.startswith(METRIC_PREFIX)
    ]
    if len(records) != 1:
        raise ArtifactPressureEvidenceError("runtime.artifact_pressure.metric_count")
    try:
        metrics = json.loads(records[0])
    except json.JSONDecodeError as error:
        raise ArtifactPressureEvidenceError("runtime.artifact_pressure.metric_json") from error
    failures = validate_metrics(metrics)
    if failures:
        raise ArtifactPressureEvidenceError("; ".join(failures))
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


def run_campaign() -> tuple[str, int]:
    started = time.monotonic()
    result = subprocess.run(
        list(COMMAND),
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        timeout=360,
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
        raise ArtifactPressureEvidenceError("runtime.artifact_pressure.command_failed")
    parse_metrics(output)
    return output, max(1, int((time.monotonic() - started) * 1_000))


def build_report(revision: str, output: str, command_elapsed_ms: int) -> dict[str, Any]:
    raw = output.encode("utf-8")
    return {
        "schema_version": 1,
        "artifact_id": "story-22.2-native-runtime-artifact-pressure",
        "source_revision": revision,
        "status": "pass-current-linux-native-reference-host",
        "task_ids": ["22.2.3.5", "RV-16", "RV-17", "RV-18"],
        "metrics": parse_metrics(output),
        "verification": {
            "command_id": sha256_bytes("\0".join(COMMAND).encode("utf-8")),
            "command_elapsed_ms": command_elapsed_ms,
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
            "Measurements describe one current Fedora x86-64 source-test process and are not a cross-platform or installed-package performance guarantee.",
            "The campaign uses a debug test binary and deliberately re-verifies the complete 64 MiB encrypted object for each bounded page read.",
            "The campaign covers 64 mixed-size unique objects and 1,024 logical references deduplicated onto one small payload; larger unique populations and installed-package capacity remain open.",
            "Physical disk-full, device-latency, power-loss, controller-failure, and filesystem-corruption injection are not claimed.",
            "Windows and macOS native artifact-store evidence and independent review remain open.",
            "Manual fuzzing remains deferred and was not executed by this campaign.",
        ],
    }


def validate_report(report: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(report, dict):
        return ["runtime.artifact_pressure.report_type"]
    if report.get("schema_version") != 1 or report.get("artifact_id") != (
        "story-22.2-native-runtime-artifact-pressure"
    ):
        failures.append("runtime.artifact_pressure.report_identity")
    revision = report.get("source_revision")
    if not isinstance(revision, str) or REVISION.fullmatch(revision) is None:
        failures.append("runtime.artifact_pressure.report_revision")
    failures.extend(validate_metrics(report.get("metrics")))
    verification = report.get("verification")
    if not isinstance(verification, dict) or verification.get("command_id") != sha256_bytes(
        "\0".join(COMMAND).encode("utf-8")
    ):
        failures.append("runtime.artifact_pressure.report_command")
    elif (
        verification.get("exit_code") != 0
        or not isinstance(verification.get("command_elapsed_ms"), int)
        or verification["command_elapsed_ms"] <= 0
        or verification["command_elapsed_ms"] > 360_000
    ):
        failures.append("runtime.artifact_pressure.report_result")
    raw_trace = report.get("raw_trace")
    if (
        not isinstance(raw_trace, dict)
        or raw_trace.get("path") != str(LOG_PATH.relative_to(ROOT))
        or raw_trace.get("redactions") != ["repository-root"]
    ):
        failures.append("runtime.artifact_pressure.report_trace")
    elif not LOG_PATH.is_file():
        failures.append("runtime.artifact_pressure.trace_missing")
    else:
        raw = LOG_PATH.read_bytes()
        if raw_trace.get("bytes") != len(raw) or raw_trace.get("sha256") != sha256_bytes(raw):
            failures.append("runtime.artifact_pressure.trace_drift")
        else:
            try:
                parse_metrics(raw.decode("utf-8"))
            except (UnicodeError, ArtifactPressureEvidenceError):
                failures.append("runtime.artifact_pressure.trace_invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(
        SOURCE_PATHS
    ):
        failures.append("runtime.artifact_pressure.report_sources")
    elif isinstance(revision, str) and REVISION.fullmatch(revision):
        for record in sources:
            content = git_blob(revision, record["path"])
            if (
                record.get("bytes") != len(content)
                or SHA256.fullmatch(str(record.get("sha256", ""))) is None
                or record["sha256"] != sha256_bytes(content)
            ):
                failures.append("runtime.artifact_pressure.source_drift")
                break
    if report.get("status") != "pass-current-linux-native-reference-host" or report.get(
        "task_ids"
    ) != ["22.2.3.5", "RV-16", "RV-17", "RV-18"]:
        failures.append("runtime.artifact_pressure.report_disposition")
    for field in ("external_network_used", "private_user_data_used"):
        if report.get(field) is not False:
            failures.append(f"runtime.artifact_pressure.{field}")
    limitations = report.get("limitations")
    if not isinstance(limitations, list) or len(limitations) != 6:
        failures.append("runtime.artifact_pressure.report_limitations")
    return failures


def read_report() -> Any:
    try:
        return json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise ArtifactPressureEvidenceError(
            "runtime.artifact_pressure.report_unavailable"
        ) from error


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    if arguments.write:
        revision = git_revision(arguments.source_revision)
        output, command_elapsed_ms = run_campaign()
        atomic_write(LOG_PATH, output.encode("utf-8"))
        atomic_write(
            REPORT_PATH,
            canonical_json_bytes(
                build_report(revision, output, command_elapsed_ms)
            ),
        )
    failures = validate_report(read_report())
    if failures:
        raise ArtifactPressureEvidenceError("; ".join(failures))
    print("Story 22.2 native artifact pressure evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
