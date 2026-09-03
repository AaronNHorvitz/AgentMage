#!/usr/bin/env python3
"""Build and validate the Story 23.4 source-security evidence map."""

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
REPORT_PATH: Final = OUTPUT_DIRECTORY / "security-evidence-map.json"
LOG_PATH: Final = OUTPUT_DIRECTORY / "security-evidence.log"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
TEST_RESULT: Final = re.compile(
    r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; "
    r"(\d+) measured; (\d+) filtered out"
)
MAXIMUM_COMMAND_ELAPSED_MS: Final = 120_000

SECURITY_REVIEW: Final = "SECURITY-REVIEW.md"
ARCHITECTURE: Final = "docs/architecture/reusable-runtime-coordinator.md"
LOCAL_RESULTS: Final = "docs/verification/story-23-4-local-results.md"
RUNTIME_REPORT: Final = (
    "artifacts/sprints/sprint-23/story-23.4/runtime-evidence.json"
)
RUNTIME_LOG: Final = "artifacts/sprints/sprint-23/story-23.4/runtime-evidence.log"
BOUNDARY_REVIEW: Final = (
    "artifacts/sprints/sprint-23/story-23.4/coordinator-boundary-review.json"
)

SOURCE_PATHS: Final = (
    SECURITY_REVIEW,
    ARCHITECTURE,
    LOCAL_RESULTS,
    RUNTIME_REPORT,
    RUNTIME_LOG,
    BOUNDARY_REVIEW,
    "kernel/contracts/src/runtime_event.rs",
    "kernel/contracts/src/runtime_run.rs",
    "kernel/engine/src/runtime_coordinator.rs",
    "kernel/engine/src/runtime_event.rs",
    "kernel/engine/src/runtime_loop.rs",
    "kernel/engine/src/runtime_loop_tests.rs",
    "shells/host/src/coding_client.rs",
    "shells/host/src/cli_runtime.rs",
    "shells/host/src/native_chat_runtime.rs",
    "shells/host/src/runtime_parity_tests.rs",
    "shells/host/src/runtime_read_tests.rs",
    "scripts/dependency_rules.py",
    "scripts/effect_boundary.py",
    "scripts/runtime_coordinator_boundary_review.py",
    "scripts/story_23_4_runtime_evidence.py",
    "scripts/story_23_4_security_evidence.py",
    "tests/test_runtime_coordinator_boundary_review.py",
    "tests/test_story_23_4_runtime_evidence.py",
    "tests/test_story_23_4_security_evidence.py",
)

COMMANDS: Final = (
    (
        "retained-runtime-evidence",
        ("python3", "scripts/story_23_4_runtime_evidence.py"),
        0,
    ),
    (
        "coordinator-boundary-review",
        ("python3", "scripts/runtime_coordinator_boundary_review.py"),
        0,
    ),
    (
        "engine-security-matrix",
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
        "native-read-security-matrix",
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
        "engine-strict-clippy",
        (
            "cargo",
            "clippy",
            "-p",
            "agentmage-kernel-engine",
            "--all-targets",
            "--locked",
            "--",
            "-D",
            "warnings",
        ),
        0,
    ),
    (
        "host-strict-clippy",
        (
            "cargo",
            "clippy",
            "-p",
            "agentmage-host",
            "--all-targets",
            "--locked",
            "--",
            "-D",
            "warnings",
        ),
        0,
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


def mapping(status: str, evidence: list[str], remaining: str) -> dict[str, Any]:
    return {"status": status, "evidence": evidence, "remaining": remaining}


DEMONSTRATED: Final = "demonstrated-story-scope"
PARTIAL: Final = "partial-story-evidence"
NOT_APPLICABLE: Final = "not-applicable-ephemeral-scope"

MAPPINGS: Final = {
    "SR-ACC-001": mapping(
        DEMONSTRATED,
        ["kernel/engine/src/runtime_loop.rs", RUNTIME_REPORT],
        "Product-wide authority review remains separately gated.",
    ),
    "SR-ACC-002": mapping(
        PARTIAL,
        ["kernel/contracts/src/runtime_run.rs", "kernel/engine/src/runtime_loop_tests.rs"],
        "The product-wide 500-case grant-field mutation corpus remains outside this story.",
    ),
    "SR-ACC-003": mapping(
        DEMONSTRATED,
        ["kernel/engine/src/runtime_loop.rs", RUNTIME_REPORT],
        "Persistent effect replay remains owned by Stories 21.2, 22, and 50.2.",
    ),
    "SR-ACC-006": mapping(
        PARTIAL,
        ["shells/host/src/runtime_read_tests.rs", RUNTIME_REPORT, BOUNDARY_REVIEW],
        "Installed-process ambient-home and credential canary inspection remains open.",
    ),
    "SR-ACC-007": mapping(
        DEMONSTRATED,
        ["shells/host/src/runtime_parity_tests.rs", RUNTIME_REPORT],
        "Installed-client human-flow observation remains open.",
    ),
    "SR-ACC-008": mapping(
        PARTIAL,
        ["shells/host/src/cli_runtime.rs", RUNTIME_REPORT],
        "The complete repository-content prompt-injection corpus remains a later campaign.",
    ),
    "SR-AI-003": mapping(
        DEMONSTRATED,
        ["kernel/engine/src/runtime_loop.rs", "kernel/engine/src/runtime_loop_tests.rs"],
        "Installed-model presentation evidence remains open.",
    ),
    "SR-AI-004": mapping(
        DEMONSTRATED,
        ["kernel/contracts/src/runtime_run.rs", "kernel/engine/src/runtime_loop_tests.rs"],
        "Controlled-write modes retain their own later approval gates.",
    ),
    "SR-AI-005": mapping(
        PARTIAL,
        ["shells/host/src/cli_runtime.rs", RUNTIME_REPORT, BOUNDARY_REVIEW],
        "The complete labeled direct and indirect injection corpus is not claimed.",
    ),
    "SR-AI-006": mapping(
        PARTIAL,
        ["shells/host/src/runtime_read_tests.rs", RUNTIME_REPORT],
        "Admitted-model repeated-trial quality evidence is unavailable.",
    ),
    "SR-AI-007": mapping(
        DEMONSTRATED,
        ["kernel/engine/src/runtime_loop.rs", "kernel/engine/src/runtime_loop_tests.rs"],
        "Installed-client accessibility review of these states remains open.",
    ),
    "SR-AI-008": mapping(
        PARTIAL,
        ["kernel/engine/src/runtime_loop_tests.rs", RUNTIME_REPORT],
        "Raw event canaries pass; installed ambient-file and model-context canaries remain open.",
    ),
    "SR-AI-009": mapping(
        DEMONSTRATED,
        ["kernel/contracts/src/runtime_run.rs", RUNTIME_REPORT],
        "Installed runtime CPU, GPU, and model-memory ceilings remain open.",
    ),
    "SR-AI-010": mapping(
        DEMONSTRATED,
        ["kernel/contracts/src/runtime_run.rs", "kernel/contracts/src/runtime_event.rs"],
        "Release-wide provenance reconciliation remains separately gated.",
    ),
    "SR-AI-013": mapping(
        DEMONSTRATED,
        ["shells/host/src/native_chat_runtime.rs", "shells/host/src/cli_runtime.rs"],
        "Installed model artifact substitution remains open.",
    ),
    "SR-AI-015": mapping(
        PARTIAL,
        ["kernel/engine/src/runtime_loop_tests.rs", RUNTIME_REPORT],
        "The full malformed, stale, oversized, and family-codec corpus remains separately owned.",
    ),
    "SR-AI-016": mapping(
        PARTIAL,
        [RUNTIME_REPORT],
        "Fixture repeatability is measured; cross-device model determinism is not claimed.",
    ),
    "SR-AI-017": mapping(
        PARTIAL,
        ["kernel/engine/src/runtime_loop.rs", "shells/host/src/runtime_parity_tests.rs"],
        "The required full policy and learned-classifier fixture counts remain open.",
    ),
    "SR-AI-018": mapping(
        DEMONSTRATED,
        ["kernel/engine/src/runtime_loop.rs", RUNTIME_REPORT],
        "Persistent restart reconciliation remains owned by later durable campaigns.",
    ),
    "SR-DAT-001": mapping(
        DEMONSTRATED,
        ["kernel/contracts/src/runtime_run.rs", "kernel/contracts/src/runtime_event.rs", ARCHITECTURE],
        "The product-wide data dictionary remains separately gated.",
    ),
    "SR-DAT-002": mapping(
        NOT_APPLICABLE,
        ["kernel/contracts/src/runtime_run.rs", ARCHITECTURE],
        "This campaign is ephemeral; durable pre-persistence policy belongs to optional ports.",
    ),
    "SR-DAT-003": mapping(
        DEMONSTRATED,
        ["kernel/engine/src/runtime_loop_tests.rs", RUNTIME_LOG],
        "Installed crash, telemetry, and export scans remain open.",
    ),
    "SR-DAT-004": mapping(
        NOT_APPLICABLE,
        ["kernel/contracts/src/runtime_run.rs", ARCHITECTURE],
        "No durable store is admitted in this ephemeral slice.",
    ),
    "SR-DAT-006": mapping(
        DEMONSTRATED,
        [LOCAL_RESULTS, RUNTIME_REPORT],
        "Release-wide claim scanning remains separately gated.",
    ),
    "SR-OPS-001": mapping(
        DEMONSTRATED,
        ["kernel/contracts/src/runtime_event.rs", "kernel/engine/src/runtime_event.rs"],
        "Product-wide event-family coverage remains separate.",
    ),
    "SR-OPS-002": mapping(
        PARTIAL,
        ["kernel/engine/src/runtime_loop_tests.rs", RUNTIME_REPORT],
        "Only Story 23.4 coordinator event families are demonstrated.",
    ),
    "SR-OPS-003": mapping(
        DEMONSTRATED,
        ["kernel/engine/src/runtime_loop_tests.rs", RUNTIME_LOG],
        "Installed logs and crash outputs remain open.",
    ),
    "SR-OPS-004": mapping(
        DEMONSTRATED,
        ["kernel/contracts/src/runtime_event.rs", "kernel/engine/src/runtime_event.rs"],
        "External collector export is outside this source-local story.",
    ),
    "SR-OPS-005": mapping(
        PARTIAL,
        ["kernel/engine/src/runtime_event.rs", RUNTIME_REPORT],
        "Installed wall-clock anomaly and sleep-resume observation remains open.",
    ),
    "SR-TST-001": mapping(
        PARTIAL,
        [RUNTIME_REPORT, "tests/test_story_23_4_runtime_evidence.py"],
        "Installed end-to-end, platform, and deferred fuzz layers remain open.",
    ),
    "SR-TST-003": mapping(
        PARTIAL,
        [
            RUNTIME_REPORT,
            BOUNDARY_REVIEW,
            "scripts/dependency_rules.py",
            "scripts/effect_boundary.py",
        ],
        "Release SAST, dependency, secret, and platform-security scans remain separate.",
    ),
    "SR-TST-004": mapping(
        DEMONSTRATED,
        ["kernel/engine/src/runtime_loop_tests.rs", "shells/host/src/cli_runtime.rs"],
        "Product-wide hostile-input closure remains separately gated.",
    ),
    "SR-TST-005": mapping(
        PARTIAL,
        ["kernel/engine/src/runtime_loop_tests.rs", RUNTIME_REPORT],
        "Persistent crash consistency and 100 restart cycles are intentionally deferred.",
    ),
    "SR-TST-006": mapping(
        DEMONSTRATED,
        [RUNTIME_REPORT],
        "Installed model and accelerator resource governance remains open.",
    ),
    "SR-TST-010": mapping(
        DEMONSTRATED,
        [RUNTIME_REPORT, RUNTIME_LOG],
        "Release-wide evidence aggregation remains separately gated.",
    ),
    "SR-TST-011": mapping(
        DEMONSTRATED,
        [ARCHITECTURE, LOCAL_RESULTS, BOUNDARY_REVIEW],
        "The gate-owned independent boundary review is complete for this source scope; product-wide review remains separately gated.",
    ),
    "RV-05": mapping(
        PARTIAL,
        ["shells/host/src/native_chat_runtime.rs", RUNTIME_REPORT, BOUNDARY_REVIEW],
        "Source replay and identity tests pass; installed IPC observation remains open.",
    ),
    "RV-17": mapping(
        PARTIAL,
        ["kernel/engine/src/runtime_loop_tests.rs", RUNTIME_REPORT],
        "Ephemeral safe stops pass; durable crash/restart evidence remains elsewhere.",
    ),
    "RV-18": mapping(
        DEMONSTRATED,
        ["kernel/engine/src/runtime_loop_tests.rs", RUNTIME_LOG],
        "Story-local event closure and redaction pass; product-wide audit remains separate.",
    ),
    "RV-20": mapping(
        PARTIAL,
        [LOCAL_RESULTS, "shells/host/src/coding_client.rs"],
        "Installed assistive-technology and independent human review evidence is absent.",
    ),
}

LIMITATIONS: Final = (
    "This map reports Story 23.4 source evidence only and is not an installed-product, supported-platform, release, or approval result.",
    "The installed host has no production runtime factory or admitted local model, so native end-to-end evidence remains blocked.",
    "Gate-owned independent coordinator review is complete; installed accessibility review evidence is absent.",
    "Persistent crash/restart, complete pressure, and optional-port evidence remain owned by Stories 21.2, 22, and 50.2.",
    "Product-wide injection counts, static analysis, secret scanning, and platform security evidence remain separate gates.",
    "Manual fuzzing remains deferred and was not executed.",
)


class SecurityEvidenceError(ValueError):
    """Raised when Story 23.4 security evidence is unavailable or stale."""


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
        raise SecurityEvidenceError("runtime.story23.security.source_revision")
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
        raise SecurityEvidenceError("runtime.story23.security.source_unavailable")
    return result.stdout


def source_records(revision: str) -> list[dict[str, Any]]:
    return [
        {"path": relative, "bytes": len(content), "sha256": sha256_bytes(content)}
        for relative in SOURCE_PATHS
        for content in [git_blob(revision, relative)]
    ]


def parse_test_result(output: str, minimum_passed: int) -> dict[str, int]:
    matches = TEST_RESULT.findall(output)
    if not matches:
        raise SecurityEvidenceError("runtime.story23.security.test_result_missing")
    labels = ("passed", "failed", "ignored", "measured", "filtered_out")
    totals = {label: 0 for label in labels}
    for match in matches:
        for label, raw in zip(labels, match, strict=True):
            totals[label] += int(raw)
    if totals["passed"] < minimum_passed or totals["failed"] != 0:
        raise SecurityEvidenceError("runtime.story23.security.test_result_failed")
    return totals


def run_checks() -> tuple[str, list[dict[str, Any]]]:
    traces: list[str] = []
    results: list[dict[str, Any]] = []
    for command_id, command, minimum_passed in COMMANDS:
        started = time.monotonic()
        result = subprocess.run(
            list(command),
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
            raise SecurityEvidenceError(
                f"runtime.story23.security.command_failed.{command_id}"
            )
        test_result = (
            parse_test_result(output, minimum_passed) if minimum_passed else None
        )
        traces.append(f"=== {command_id} ===\n{output}")
        results.append(
            {
                "command_id": command_id,
                "argv_sha256": command_sha256(command),
                "elapsed_ms": elapsed_ms,
                "maximum_elapsed_ms": MAXIMUM_COMMAND_ELAPSED_MS,
                "exit_code": 0,
                "test_result": test_result,
            }
        )
    return "\n".join(traces), results


def requirement_ids_in_review(revision: str) -> set[str]:
    text = git_blob(revision, SECURITY_REVIEW).decode("utf-8")
    return set(re.findall(r"`(SR-[A-Z]+-\d{3}|RV-\d{2})`", text))


def validate_mapping_shape(revision: str | None = None) -> list[str]:
    failures: list[str] = []
    if len(MAPPINGS) != len(set(MAPPINGS)):
        failures.append("runtime.story23.security.mapping_duplicate")
    allowed_statuses = {DEMONSTRATED, PARTIAL, NOT_APPLICABLE}
    available_paths = set(SOURCE_PATHS)
    for requirement_id, record in MAPPINGS.items():
        if (
            set(record) != {"status", "evidence", "remaining"}
            or record["status"] not in allowed_statuses
            or not isinstance(record["evidence"], list)
            or not record["evidence"]
            or any(path not in available_paths for path in record["evidence"])
            or not isinstance(record["remaining"], str)
            or not record["remaining"]
        ):
            failures.append(f"runtime.story23.security.mapping.{requirement_id}")
    if revision is not None:
        missing = set(MAPPINGS) - requirement_ids_in_review(revision)
        if missing:
            failures.append("runtime.story23.security.requirement_missing")
    return failures


def build_report(
    revision: str,
    output: str,
    commands: list[dict[str, Any]],
) -> dict[str, Any]:
    mapping_failures = validate_mapping_shape(revision)
    if mapping_failures:
        raise SecurityEvidenceError("; ".join(mapping_failures))
    raw = output.encode("utf-8")
    summary = {
        "demonstrated_count": sum(
            record["status"] == DEMONSTRATED for record in MAPPINGS.values()
        ),
        "partial_count": sum(record["status"] == PARTIAL for record in MAPPINGS.values()),
        "not_applicable_count": sum(
            record["status"] == NOT_APPLICABLE for record in MAPPINGS.values()
        ),
        "complete_story_security_evidence": True,
    }
    return {
        "schema_version": 1,
        "artifact_id": "story-23.4-source-security-evidence-map",
        "source_revision": revision,
        "status": "pass-current-linux-source-security-map",
        "task_ids": ["23.4.3.2", "23.4.3.3", "23.4.3.6"],
        "mappings": [
            {"requirement_id": requirement_id, **record}
            for requirement_id, record in MAPPINGS.items()
        ],
        "summary": summary,
        "commands": commands,
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
        "independent_review_complete": True,
        "installed_runtime_complete": False,
        "limitations": list(LIMITATIONS),
    }


def validate_report(report: Any) -> list[str]:
    failures = validate_mapping_shape()
    if not isinstance(report, dict):
        return ["runtime.story23.security.report_type", *failures]
    if report.get("schema_version") != 1 or report.get("artifact_id") != (
        "story-23.4-source-security-evidence-map"
    ):
        failures.append("runtime.story23.security.report_identity")
    revision = report.get("source_revision")
    if not isinstance(revision, str) or REVISION.fullmatch(revision) is None:
        failures.append("runtime.story23.security.report_revision")
    else:
        failures.extend(validate_mapping_shape(revision))
    expected_mappings = [
        {"requirement_id": requirement_id, **record}
        for requirement_id, record in MAPPINGS.items()
    ]
    if report.get("mappings") != expected_mappings:
        failures.append("runtime.story23.security.report_mappings")
    statuses = [record["status"] for record in MAPPINGS.values()]
    expected_summary = {
        "demonstrated_count": statuses.count(DEMONSTRATED),
        "partial_count": statuses.count(PARTIAL),
        "not_applicable_count": statuses.count(NOT_APPLICABLE),
        "complete_story_security_evidence": True,
    }
    if report.get("summary") != expected_summary:
        failures.append("runtime.story23.security.report_summary")
    commands = report.get("commands")
    if not isinstance(commands, list) or len(commands) != len(COMMANDS):
        failures.append("runtime.story23.security.report_commands")
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
                or not isinstance(record.get("elapsed_ms"), int)
                or isinstance(record.get("elapsed_ms"), bool)
                or not 0 < record["elapsed_ms"] <= MAXIMUM_COMMAND_ELAPSED_MS
                or not valid_test_result
            ):
                failures.append("runtime.story23.security.report_command")
                break
    raw_trace = report.get("raw_trace")
    if (
        not isinstance(raw_trace, dict)
        or raw_trace.get("path") != str(LOG_PATH.relative_to(ROOT))
        or raw_trace.get("redactions") != ["repository-root"]
    ):
        failures.append("runtime.story23.security.report_trace")
    elif not LOG_PATH.is_file():
        failures.append("runtime.story23.security.trace_missing")
    else:
        raw = LOG_PATH.read_bytes()
        if raw_trace.get("bytes") != len(raw) or raw_trace.get("sha256") != sha256_bytes(raw):
            failures.append("runtime.story23.security.trace_drift")
        elif str(ROOT).encode("utf-8") in raw:
            failures.append("runtime.story23.security.trace_root_disclosure")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(
        SOURCE_PATHS
    ):
        failures.append("runtime.story23.security.report_sources")
    elif isinstance(revision, str) and REVISION.fullmatch(revision):
        for record in sources:
            content = git_blob(revision, record["path"])
            if (
                record.get("bytes") != len(content)
                or SHA256.fullmatch(str(record.get("sha256", ""))) is None
                or record["sha256"] != sha256_bytes(content)
            ):
                failures.append("runtime.story23.security.source_drift")
                break
    if report.get("status") != "pass-current-linux-source-security-map":
        failures.append("runtime.story23.security.report_status")
    for field in (
        "external_network_used",
        "private_user_data_used",
        "manual_fuzzing_executed",
        "installed_runtime_complete",
    ):
        if report.get(field) is not False:
            failures.append(f"runtime.story23.security.{field}")
    if report.get("independent_review_complete") is not True:
        failures.append("runtime.story23.security.independent_review_complete")
    if report.get("limitations") != list(LIMITATIONS):
        failures.append("runtime.story23.security.report_limitations")
    return failures


def read_report() -> Any:
    try:
        return json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise SecurityEvidenceError(
            "runtime.story23.security.report_unavailable"
        ) from error


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    if arguments.write:
        revision = git_revision(arguments.source_revision)
        output, commands = run_checks()
        atomic_write(LOG_PATH, output.encode("utf-8"))
        atomic_write(
            REPORT_PATH,
            canonical_json_bytes(build_report(revision, output, commands)),
        )
    report = read_report()
    failures = validate_report(report)
    if failures:
        raise SecurityEvidenceError("; ".join(failures))
    print("Story 23.4 source security evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
