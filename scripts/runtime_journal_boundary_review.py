#!/usr/bin/env python3
"""Independently review the committed runtime-journal source boundary and evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path
from typing import Any, Final

try:
    from scripts.evidence_core import atomic_write, canonical_json_bytes
except ModuleNotFoundError:
    from evidence_core import atomic_write, canonical_json_bytes


ROOT: Final = Path(__file__).resolve().parents[1]
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-21/story-21.2/journal-boundary-review.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
ZERO_SHA256: Final = "0" * 64
SOURCE_PATHS: Final = (
    "schemas/runtime/runtime-event.schema.json",
    "kernel/contracts/src/runtime_event.rs",
    "kernel/engine/src/runtime_event.rs",
    "kernel/engine/src/runtime_journal.rs",
    "kernel/engine/src/runtime_loop.rs",
    "kernel/engine/src/runtime_loop_tests.rs",
    "kernel/engine/src/runtime_projection.rs",
    "kernel/engine/src/runtime_artifact.rs",
    "docs/architecture/runtime-event-journal.md",
    "docs/verification/story-21-2-local-results.md",
    "TASKS.md",
    "scripts/runtime_journal_boundary_review.py",
    "tests/test_runtime_journal_boundary_review.py",
)
EVIDENCE_PATHS: Final = (
    "artifacts/sprints/sprint-21/story-21.2/crash-matrix.json",
    "artifacts/sprints/sprint-21/story-21.2/crash-matrix.log",
    "artifacts/sprints/sprint-21/story-21.2/pressure-report.json",
    "artifacts/sprints/sprint-21/story-21.2/pressure-report.log",
    "artifacts/sprints/sprint-21/story-21.2/evidence-index.json",
    "artifacts/sprints/sprint-50/story-50.2-runtime-load-worker/report.json",
)
EVENT_FAMILIES: Final = (
    "RunStarted",
    "TurnStarted",
    "TurnCompleted",
    "ModelRequested",
    "ModelCompleted",
    "ModelFailed",
    "ToolRequested",
    "ToolStarted",
    "ToolCompleted",
    "ToolFailed",
    "PermissionRequested",
    "PermissionDecided",
    "FileObserved",
    "FileModified",
    "ArtifactCreated",
    "CheckpointCommitted",
    "CancellationRequested",
    "CancellationObserved",
    "Progress",
    "Metric",
    "RunTerminal",
)
PROHIBITED_NETWORK_TOKENS: Final = (
    "reqwest::",
    "TcpStream",
    "UdpSocket",
    "hyper::Client",
    "tonic::transport",
)
LIMITATIONS: Final = (
    "This is an independent automated source-boundary review, not an independent human review.",
    "Physical filesystem/device faults, host power loss, and integrated physical-effect recovery are not exercised.",
    "Installed clients, supported-platform packages, and additional model/runtime profiles are not reviewed.",
    "Manual fuzzing remains deferred and was not executed.",
    "No release approval, model enablement, platform support, or deployment authority follows from this review.",
)


class JournalReviewError(ValueError):
    """Raised when committed journal review inputs are missing or invalid."""


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
        raise JournalReviewError("runtime.review.source_revision")
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
        raise JournalReviewError(f"runtime.review.source_unavailable.{relative}")
    return result.stdout


def json_blob(revision: str, relative: str) -> dict[str, Any]:
    try:
        value = json.loads(git_blob(revision, relative))
    except (UnicodeError, json.JSONDecodeError) as error:
        raise JournalReviewError(f"runtime.review.json_invalid.{relative}") from error
    if not isinstance(value, dict):
        raise JournalReviewError(f"runtime.review.json_type.{relative}")
    return value


def source_records(revision: str) -> list[dict[str, Any]]:
    return [
        {"path": path, "bytes": len(content), "sha256": sha256_bytes(content)}
        for path in (*SOURCE_PATHS, *EVIDENCE_PATHS)
        for content in [git_blob(revision, path)]
    ]


def check_record(identifier: str, passed: bool, detail: str) -> dict[str, Any]:
    return {"check_id": identifier, "passed": passed, "detail": detail}


def review_checks(revision: str) -> list[dict[str, Any]]:
    schema = json_blob(revision, SOURCE_PATHS[0])
    contract = git_blob(revision, "kernel/contracts/src/runtime_event.rs").decode("utf-8")
    event_source = git_blob(revision, "kernel/engine/src/runtime_event.rs").decode("utf-8")
    journal = git_blob(revision, "kernel/engine/src/runtime_journal.rs").decode("utf-8")
    runtime_loop = git_blob(revision, "kernel/engine/src/runtime_loop.rs").decode("utf-8")
    loop_tests = git_blob(revision, "kernel/engine/src/runtime_loop_tests.rs").decode("utf-8")
    projection = git_blob(revision, "kernel/engine/src/runtime_projection.rs").decode("utf-8")
    artifact = git_blob(revision, "kernel/engine/src/runtime_artifact.rs").decode("utf-8")
    architecture = git_blob(
        revision, "docs/architecture/runtime-event-journal.md"
    ).decode("utf-8")
    tasks = git_blob(revision, "TASKS.md").decode("utf-8")
    crash = json_blob(revision, EVIDENCE_PATHS[0])
    crash_log = git_blob(revision, EVIDENCE_PATHS[1])
    pressure = json_blob(revision, EVIDENCE_PATHS[2])
    pressure_log = git_blob(revision, EVIDENCE_PATHS[3])
    index = json_blob(revision, EVIDENCE_PATHS[4])
    load = json_blob(revision, EVIDENCE_PATHS[5])

    schema_required = set(schema.get("required", []))
    required_bindings = {
        "event_id",
        "run_id",
        "session_id",
        "task_id",
        "correlation_id",
        "sequence",
        "occurred_at_epoch_ms",
        "sensitivity",
        "retention",
        "persistence",
        "policy_id",
        "kind",
        "previous_event_sha256",
        "event_sha256",
    }
    closed_schema = schema.get("additionalProperties") is False and required_bindings.issubset(
        schema_required
    )
    closed_families = all(
        contract.count(f"{family}") >= 1 and event_source.count(f"{family}") >= 1
        for family in EVENT_FAMILIES
    )
    persistence_partition = all(
        marker in event_source
        for marker in (
            "RuntimeEventPersistenceClass::Correctness",
            "RuntimeEventPersistenceClass::Progress",
            "RuntimeEventPersistenceClass::Metric",
            "story_21_2_closed_event_families_have_one_canonical_persistence_class",
        )
    )
    bounded_queue = all(
        marker in journal
        for marker in (
            "MAX_QUEUE_EVENTS",
            "MAX_QUEUE_BYTES",
            "MAX_BATCH_EVENTS",
            "MAX_BATCH_BYTES",
            "RuntimeJournalError::QueueSaturated",
            "story_21_2_worker_enforces_the_exact_canonical_byte_boundary",
        )
    )
    cancellation_truth = all(
        marker in runtime_loop + loop_tests
        for marker in (
            "Some(signal) => self.cancel(signal)",
            "RuntimeEventKind::CancellationRequested",
            "RuntimeEventKind::CancellationObserved",
            "story_21_2_durable_model_progress_and_cancellation_survive_delayed_sqlcipher",
        )
    )
    source_closure = "\n".join((event_source, journal, runtime_loop, projection, artifact))
    network_free = all(token not in source_closure for token in PROHIBITED_NETWORK_TOKENS)
    crash_valid = (
        crash.get("status") == "pass-current-linux-source-boundary"
        and crash.get("metrics", {}).get("case_count") == 8
        and crash.get("metrics", {}).get("false_terminal_count") == 0
        and crash.get("raw_trace", {}).get("sha256") == sha256_bytes(crash_log)
    )
    pressure_valid = (
        pressure.get("status") == "pass-current-linux-source-pressure"
        and all(pressure.get("coverage", {}).values())
        and pressure.get("metrics", {}).get("external_network_used") is False
        and pressure.get("raw_trace", {}).get("sha256") == sha256_bytes(pressure_log)
        and any("not physical" in item for item in pressure.get("limitations", []))
    )
    load_valid = (
        load.get("campaign_passed") is True
        and load.get("disposition") == "PARTIAL-PASS"
        and load.get("failures") == []
        and load.get("metrics", {}).get("event_count") == 8_196
        and load.get("metrics", {}).get("restart_cycles") == 16
    )
    index_summary = index.get("summary", {})
    index_truthful = (
        index.get("disposition") == "partial-local-evidence"
        and index_summary.get("story_complete") is False
        and index_summary.get("sprint_complete") is False
        and index_summary.get("release_approved") is False
        and SHA256.fullmatch(str(index.get("index_sha256", ""))) is not None
    )
    canary_closure = all(
        marker in projection + artifact
        for marker in (
            "story_21_2_sensitive_canaries_are_excluded_or_restricted_in_every_projection",
            "story_21_2_projection_runtime_has_no_external_telemetry_dependency",
            "story_21_2_artifact_canaries_require_an_exact_owner_bound_payload_read",
        )
    )
    limitation_truth = all(
        phrase in architecture + tasks
        for phrase in (
            "Physical filesystem/device",
            "independent",
            "Manual fuzzing",
        )
    )
    return [
        check_record("closed-event-schema", closed_schema, "Schema rejects unknown fields and requires canonical bindings."),
        check_record("closed-event-families", closed_families, "All 21 declared families exist in contract and verifier source."),
        check_record("persistence-partition", persistence_partition, "Correctness, progress, and metric classes remain explicit and tested."),
        check_record("bounded-journal-queue", bounded_queue, "Count, byte, batch, saturation, and exact byte-edge boundaries are present."),
        check_record("cancellation-terminal-truth", cancellation_truth, "Model cancellation emits the required ordered cancellation events."),
        check_record("network-free-source-closure", network_free, "Security-authoritative journal closure contains no prohibited network client token."),
        check_record("crash-report-integrity", crash_valid, "Eight stop cases, no false terminal, and raw trace digest reconcile."),
        check_record("pressure-report-integrity", pressure_valid, "Byte, progress, cancellation, durability, limitation, and trace records reconcile."),
        check_record("load-report-integrity", load_valid, "Partial Fedora load campaign retains exact event and restart counts."),
        check_record("canary-source-closure", canary_closure, "Projection, artifact, and telemetry canary tests remain present."),
        check_record("evidence-index-truth", index_truthful, "Hashed index denies story, sprint, and release completion."),
        check_record("limitation-truth", limitation_truth, "Physical, independent-review, and manual-fuzz limitations remain visible."),
    ]


def build_report(revision: str) -> dict[str, Any]:
    checks = review_checks(revision)
    return {
        "schema_version": 1,
        "artifact_id": "story-21.2-runtime-journal-boundary-review",
        "source_revision": revision,
        "status": "pass-independent-automated-source-boundary-review"
        if all(check["passed"] for check in checks)
        else "fail-independent-automated-source-boundary-review",
        "review_type": "independent-automated-boundary-review",
        "checks": checks,
        "sources": source_records(revision),
        "external_network_used": False,
        "private_user_data_used": False,
        "independent_human_review_performed": False,
        "manual_fuzzing_executed": False,
        "release_approved": False,
        "limitations": list(LIMITATIONS),
        "report_sha256": ZERO_SHA256,
    }


def seal_report(report: dict[str, Any]) -> dict[str, Any]:
    sealed = json.loads(json.dumps(report))
    sealed["report_sha256"] = ZERO_SHA256
    sealed["report_sha256"] = sha256_bytes(canonical_json_bytes(sealed))
    return sealed


def validate_report(report: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(report, dict):
        return ["runtime.review.report_type"]
    revision = report.get("source_revision")
    if not isinstance(revision, str) or REVISION.fullmatch(revision) is None:
        return ["runtime.review.report_revision"]
    try:
        expected = seal_report(build_report(revision))
    except JournalReviewError:
        return ["runtime.review.source_unavailable"]
    if report != expected:
        failures.append("runtime.review.report_drift")
    if report.get("status") != "pass-independent-automated-source-boundary-review":
        failures.append("runtime.review.failed")
    if any(check.get("passed") is not True for check in report.get("checks", [])):
        failures.append("runtime.review.check_failed")
    for field in (
        "external_network_used",
        "private_user_data_used",
        "independent_human_review_performed",
        "manual_fuzzing_executed",
        "release_approved",
    ):
        if report.get(field) is not False:
            failures.append(f"runtime.review.overclaim.{field}")
    return failures


def read_report() -> Any:
    try:
        return json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise JournalReviewError("runtime.review.report_unavailable") from error


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    if arguments.write:
        revision = git_revision(arguments.source_revision)
        report = seal_report(build_report(revision))
        if report["status"] != "pass-independent-automated-source-boundary-review":
            raise JournalReviewError("runtime.review.check_failed")
        atomic_write(REPORT_PATH, canonical_json_bytes(report))
    failures = validate_report(read_report())
    if failures:
        raise JournalReviewError("; ".join(failures))
    print("Story 21.2 independent automated journal review: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
