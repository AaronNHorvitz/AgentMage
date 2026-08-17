#!/usr/bin/env python3
"""Build and validate the Story 21.2 product-security evidence map."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shlex
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
REPORT_PATH: Final = OUTPUT_DIRECTORY / "security-evidence-map.json"
LOG_PATH: Final = OUTPUT_DIRECTORY / "security-evidence.log"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
ZERO_SHA256: Final = "0" * 64
REQUIREMENT_IDS: Final = (
    "SR-DAT-001",
    "SR-DAT-002",
    "SR-DAT-003",
    "SR-DAT-004",
    "SR-DAT-005",
    "SR-DAT-006",
    "SR-DAT-007",
    "SR-DAT-008",
    "SR-DAT-009",
    "SR-DAT-010",
    "SR-DAT-011",
    "SR-DAT-012",
    "SR-AI-010",
    "SR-OPS-001",
    "SR-OPS-002",
    "SR-OPS-003",
    "SR-OPS-004",
    "SR-OPS-005",
    "SR-TST-005",
    "SR-TST-006",
    "SR-TST-010",
    "RV-18",
)
SCHEMA: Final = "schemas/runtime/runtime-event.schema.json"
CONTRACT: Final = "kernel/contracts/src/runtime_event.rs"
EVENT_SOURCE: Final = "kernel/engine/src/runtime_event.rs"
JOURNAL_SOURCE: Final = "kernel/engine/src/runtime_journal.rs"
LOOP_SOURCE: Final = "kernel/engine/src/runtime_loop.rs"
LOOP_TESTS: Final = "kernel/engine/src/runtime_loop_tests.rs"
PROJECTION_SOURCE: Final = "kernel/engine/src/runtime_projection.rs"
ARTIFACT_SOURCE: Final = "kernel/engine/src/runtime_artifact.rs"
ARCHITECTURE: Final = "docs/architecture/runtime-event-journal.md"
LOCAL_RESULTS: Final = "docs/verification/story-21-2-local-results.md"
SECURITY_MAP: Final = "docs/verification/task-21-2-3-6-product-security-evidence.md"
SECURITY_REVIEW: Final = "SECURITY-REVIEW.md"
CRASH_REPORT: Final = "artifacts/sprints/sprint-21/story-21.2/crash-matrix.json"
CRASH_LOG: Final = "artifacts/sprints/sprint-21/story-21.2/crash-matrix.log"
PRESSURE_REPORT: Final = "artifacts/sprints/sprint-21/story-21.2/pressure-report.json"
PRESSURE_LOG: Final = "artifacts/sprints/sprint-21/story-21.2/pressure-report.log"
BOUNDARY_REVIEW: Final = (
    "artifacts/sprints/sprint-21/story-21.2/journal-boundary-review.json"
)
EVIDENCE_INDEX: Final = "artifacts/sprints/sprint-21/story-21.2/evidence-index.json"
LOAD_REPORT: Final = (
    "artifacts/sprints/sprint-50/story-50.2-runtime-load-worker/report.json"
)
SOURCE_PATHS: Final = (
    SECURITY_MAP,
    SECURITY_REVIEW,
    ARCHITECTURE,
    LOCAL_RESULTS,
    SCHEMA,
    CONTRACT,
    EVENT_SOURCE,
    JOURNAL_SOURCE,
    LOOP_SOURCE,
    LOOP_TESTS,
    PROJECTION_SOURCE,
    ARTIFACT_SOURCE,
    "kernel/engine/Cargo.toml",
    "scripts/story_21_2_crash_evidence.py",
    "scripts/story_21_2_pressure_evidence.py",
    "scripts/runtime_journal_boundary_review.py",
    "scripts/story_21_2_evidence_index.py",
    "scripts/story_21_2_security_evidence.py",
    "tests/test_story_21_2_security_evidence.py",
    CRASH_REPORT,
    CRASH_LOG,
    PRESSURE_REPORT,
    PRESSURE_LOG,
    BOUNDARY_REVIEW,
    EVIDENCE_INDEX,
    LOAD_REPORT,
)


def evidence(*paths: str) -> list[str]:
    return list(paths)


MAPPINGS: Final = {
    "SR-DAT-001": {
        "contribution": "demonstrated-story-scope",
        "evidence": evidence(SCHEMA, EVENT_SOURCE, EVIDENCE_INDEX),
        "remaining": "Product-wide runtime inventory comparison.",
    },
    "SR-DAT-002": {
        "contribution": "demonstrated-story-scope",
        "evidence": evidence(SCHEMA, EVENT_SOURCE, PROJECTION_SOURCE),
        "remaining": "Installed-client storage observation.",
    },
    "SR-DAT-003": {
        "contribution": "demonstrated-story-scope",
        "evidence": evidence(PROJECTION_SOURCE, ARTIFACT_SOURCE, PRESSURE_LOG),
        "remaining": "Product-wide memory, crash, and OS telemetry scan.",
    },
    "SR-DAT-004": {
        "contribution": "demonstrated-story-scope",
        "evidence": evidence(JOURNAL_SOURCE, CRASH_REPORT),
        "remaining": "Supported-platform key-store failure campaign.",
    },
    "SR-DAT-005": {
        "contribution": "partial-story-evidence",
        "evidence": evidence("kernel/engine/Cargo.toml", JOURNAL_SOURCE, BOUNDARY_REVIEW),
        "remaining": "Release cryptographic-provider review and self-tests where available.",
    },
    "SR-DAT-006": {
        "contribution": "demonstrated-story-scope",
        "evidence": evidence(ARCHITECTURE, BOUNDARY_REVIEW),
        "remaining": "Release documentation and diagnostics claim scan.",
    },
    "SR-DAT-007": {
        "contribution": "partial-story-evidence",
        "evidence": evidence(JOURNAL_SOURCE, EVIDENCE_INDEX),
        "remaining": "Platform key-service integration and process instrumentation.",
    },
    "SR-DAT-008": {
        "contribution": "partial-inherited-evidence",
        "evidence": evidence("kernel/engine/Cargo.toml", BOUNDARY_REVIEW),
        "remaining": "Package and runtime cryptographic inventory reconciliation.",
    },
    "SR-DAT-009": {
        "contribution": "owned-by-later-gate",
        "evidence": evidence(SCHEMA, JOURNAL_SOURCE, ARCHITECTURE),
        "remaining": "Approved provider substitution and migration/rollback campaign.",
    },
    "SR-DAT-010": {
        "contribution": "partial-story-evidence",
        "evidence": evidence(SCHEMA, PROJECTION_SOURCE, ARCHITECTURE),
        "remaining": "Expiration, hold, export, backup, restore, and deletion lifecycle.",
    },
    "SR-DAT-011": {
        "contribution": "partial-inherited-evidence",
        "evidence": evidence(JOURNAL_SOURCE, CRASH_REPORT),
        "remaining": "Installed SSD, copy-on-write, cache, and backup verification.",
    },
    "SR-DAT-012": {
        "contribution": "owned-by-later-gate",
        "evidence": evidence(ARCHITECTURE, EVIDENCE_INDEX),
        "remaining": "Installed uninstall and residue campaign on every supported platform.",
    },
    "SR-AI-010": {
        "contribution": "partial-story-evidence",
        "evidence": evidence(SCHEMA, CONTRACT, EVENT_SOURCE, EVIDENCE_INDEX),
        "remaining": "Installed model/runtime manifest resolution for every inference.",
    },
    "SR-OPS-001": {
        "contribution": "demonstrated-story-scope",
        "evidence": evidence(SCHEMA, CONTRACT, EVENT_SOURCE),
        "remaining": "Product-wide security-event registry coverage.",
    },
    "SR-OPS-002": {
        "contribution": "partial-story-evidence",
        "evidence": evidence(EVENT_SOURCE, LOOP_SOURCE, EVIDENCE_INDEX),
        "remaining": "Configuration, installer, alert, and non-runtime product events.",
    },
    "SR-OPS-003": {
        "contribution": "demonstrated-story-scope",
        "evidence": evidence(PROJECTION_SOURCE, ARTIFACT_SOURCE, CRASH_LOG, PRESSURE_LOG),
        "remaining": "Product-wide memory, crash, export, and OS telemetry scan.",
    },
    "SR-OPS-004": {
        "contribution": "partial-story-evidence",
        "evidence": evidence(JOURNAL_SOURCE, CRASH_REPORT, BOUNDARY_REVIEW),
        "remaining": "Signed checkpoint/export and external collector integration.",
    },
    "SR-OPS-005": {
        "contribution": "partial-story-evidence",
        "evidence": evidence(SCHEMA, EVENT_SOURCE, JOURNAL_SOURCE),
        "remaining": "Monotonic-time field and explicit clock-anomaly event campaign.",
    },
    "SR-TST-005": {
        "contribution": "partial-story-evidence",
        "evidence": evidence(CRASH_REPORT, CRASH_LOG, JOURNAL_SOURCE),
        "remaining": "One hundred integrated resumes plus power and device faults.",
    },
    "SR-TST-006": {
        "contribution": "partial-story-evidence",
        "evidence": evidence(PRESSURE_REPORT, PRESSURE_LOG, LOAD_REPORT),
        "remaining": "Physical device latency and installed-interface campaigns.",
    },
    "SR-TST-010": {
        "contribution": "demonstrated-story-scope",
        "evidence": evidence(
            CRASH_REPORT,
            PRESSURE_REPORT,
            BOUNDARY_REVIEW,
            EVIDENCE_INDEX,
            LOAD_REPORT,
        ),
        "remaining": "Release evidence recomputation by independent reviewers.",
    },
    "RV-18": {
        "contribution": "demonstrated-local-story-scope",
        "evidence": evidence(
            SCHEMA,
            EVENT_SOURCE,
            JOURNAL_SOURCE,
            PROJECTION_SOURCE,
            ARTIFACT_SOURCE,
            CRASH_REPORT,
            PRESSURE_REPORT,
            BOUNDARY_REVIEW,
            EVIDENCE_INDEX,
        ),
        "remaining": "Product-wide event coverage and independent human review.",
    },
}
DOCUMENT_FRAGMENTS: Final = (
    "Mapped controls and review protocols: **22 of 22**.",
    "This result completes the local evidence-organization obligation for Sub-task",
    "independent automated source-boundary review",
    "Physical filesystem/device faults",
    "Manual fuzzing remains deliberately deferred",
    "No result in this document enables a model, interface, supported platform,",
)
COMMAND_SPECS: Final = (
    (
        "projection-canaries",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine", "--lib", "--locked",
            "runtime_projection::tests::story_21_2_sensitive_canaries_are_excluded_or_restricted_in_every_projection",
            "--", "--exact", "--nocapture",
        ),
        "1 passed; 0 failed",
    ),
    (
        "artifact-canaries",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine", "--lib", "--locked",
            "runtime_artifact::tests::story_21_2_artifact_canaries_require_an_exact_owner_bound_payload_read",
            "--", "--exact", "--nocapture",
        ),
        "1 passed; 0 failed",
    ),
    (
        "network-source-closure",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine", "--lib", "--locked",
            "runtime_projection::tests::story_21_2_projection_runtime_has_no_external_telemetry_dependency",
            "--", "--exact", "--nocapture",
        ),
        "1 passed; 0 failed",
    ),
    (
        "crash-evidence", ("python3", "scripts/story_21_2_crash_evidence.py"),
        "Story 21.2 journal crash evidence: pass",
    ),
    (
        "pressure-evidence", ("python3", "scripts/story_21_2_pressure_evidence.py"),
        "Story 21.2 journal pressure evidence: pass",
    ),
    (
        "independent-boundary-review",
        ("python3", "scripts/runtime_journal_boundary_review.py"),
        "Story 21.2 independent automated journal review: pass",
    ),
    (
        "evidence-index", ("python3", "scripts/story_21_2_evidence_index.py", "--check"),
        None,
    ),
    ("schema-suite", ("npm", "run", "schemas:check"), None),
)
LIMITATIONS: Final = (
    "This closes local Story 21.2 security-evidence organization, not any product or release security requirement.",
    "The independent reviewer is an automated implementation with separate rules, not an independent human review.",
    "Physical filesystem/device faults, host power loss, installed clients, and additional supported-platform profiles are not exercised.",
    "Manual fuzzing remains deferred and was not executed.",
    "No model, interface, platform, package, deployment, or release is enabled or approved by this evidence.",
    "Raw command output replaces the absolute checkout root with the literal <repository-root>.",
)


class SecurityEvidenceError(ValueError):
    """Raised when Story 21.2 security evidence is missing or invalid."""


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
        raise SecurityEvidenceError("runtime.security.source_revision")
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
        raise SecurityEvidenceError(f"runtime.security.source_unavailable.{relative}")
    return result.stdout


def source_records(revision: str) -> list[dict[str, Any]]:
    values: dict[str, str] = {}
    records: list[dict[str, Any]] = []
    for relative in SOURCE_PATHS:
        content = git_blob(revision, relative)
        values[relative] = content.decode("utf-8")
        records.append(
            {"path": relative, "bytes": len(content), "sha256": sha256_bytes(content)}
        )
    if failures := validate_source_values(values):
        raise SecurityEvidenceError("; ".join(failures))
    return records


def validate_source_values(values: dict[str, str]) -> list[str]:
    failures: list[str] = []
    if set(values) != set(SOURCE_PATHS):
        return ["runtime.security.source_set"]
    document = values[SECURITY_MAP]
    security = values[SECURITY_REVIEW]
    for fragment in DOCUMENT_FRAGMENTS:
        if document.count(fragment) != 1:
            failures.append("runtime.security.document_fragment")
    for requirement_id in REQUIREMENT_IDS:
        marker = f"`{requirement_id}`"
        if marker not in security:
            failures.append(f"runtime.security.registry.{requirement_id}")
        if document.count(marker) != 1:
            failures.append(f"runtime.security.mapping.{requirement_id}")
    if tuple(MAPPINGS) != REQUIREMENT_IDS:
        failures.append("runtime.security.mapping_order")
    for requirement_id, mapping in MAPPINGS.items():
        mapped_paths = mapping.get("evidence")
        if not isinstance(mapped_paths, list) or not mapped_paths:
            failures.append(f"runtime.security.mapping_empty.{requirement_id}")
        elif any(path not in SOURCE_PATHS for path in mapped_paths):
            failures.append(f"runtime.security.mapping_path.{requirement_id}")
    expected_statuses = {
        CRASH_REPORT: "pass-current-linux-source-boundary",
        PRESSURE_REPORT: "pass-current-linux-source-pressure",
        BOUNDARY_REVIEW: "pass-independent-automated-source-boundary-review",
    }
    for path, expected_status in expected_statuses.items():
        try:
            artifact = json.loads(values[path])
        except json.JSONDecodeError:
            failures.append(f"runtime.security.artifact_json.{path}")
            continue
        if artifact.get("status") != expected_status:
            failures.append(f"runtime.security.artifact_status.{path}")
        if artifact.get("external_network_used") is not False:
            failures.append(f"runtime.security.artifact_network.{path}")
        if artifact.get("private_user_data_used") is not False:
            failures.append(f"runtime.security.artifact_private_data.{path}")
    index = json.loads(values[EVIDENCE_INDEX])
    if (
        index.get("disposition") != "partial-local-evidence"
        or index.get("summary", {}).get("release_approved") is not False
    ):
        failures.append("runtime.security.index_disposition")
    load = json.loads(values[LOAD_REPORT])
    if load.get("disposition") != "PARTIAL-PASS" or load.get("failures") != []:
        failures.append("runtime.security.load_disposition")
    root_marker = str(ROOT)
    for path in (CRASH_LOG, PRESSURE_LOG):
        if root_marker in values[path]:
            failures.append(f"runtime.security.unredacted_root.{path}")
    return failures


def command_sha256(arguments: tuple[str, ...]) -> str:
    return sha256_bytes("\0".join(arguments).encode("utf-8"))


def expected_command_identity() -> list[dict[str, Any]]:
    return [
        {
            "command_id": command_id,
            "argv_sha256": command_sha256(arguments),
            "expected_marker_sha256": (
                sha256_bytes(marker.encode("utf-8")) if marker else None
            ),
        }
        for command_id, arguments, marker in COMMAND_SPECS
    ]


def run_commands() -> tuple[bytes, list[dict[str, Any]]]:
    traces: list[str] = []
    records: list[dict[str, Any]] = []
    for command_id, arguments, marker in COMMAND_SPECS:
        started = time.monotonic()
        result = subprocess.run(
            list(arguments),
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
            },
        )
        output = (result.stdout + result.stderr).replace(str(ROOT), "<repository-root>")
        if result.returncode or (marker is not None and marker not in output):
            raise SecurityEvidenceError(f"runtime.security.command_failed.{command_id}")
        elapsed_ms = max(1, int((time.monotonic() - started) * 1_000))
        traces.append(f"=== {command_id} ===\n$ {shlex.join(arguments)}\n{output}")
        identity = next(
            item for item in expected_command_identity() if item["command_id"] == command_id
        )
        records.append(
            {
                **identity,
                "elapsed_ms": elapsed_ms,
                "exit_code": 0,
                "status": "pass",
            }
        )
    return "\n".join(traces).encode("utf-8"), records


def build_report(
    revision: str, raw_log: bytes, commands: list[dict[str, Any]]
) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "artifact_id": "story-21.2-product-security-evidence-map",
        "source_revision": revision,
        "status": "pass-local-story-security-evidence",
        "task_ids": ["21.2.3.6", "RV-18"],
        "requirement_ids": list(REQUIREMENT_IDS),
        "mappings": [
            {"requirement_id": requirement_id, **MAPPINGS[requirement_id]}
            for requirement_id in REQUIREMENT_IDS
        ],
        "verification_commands": commands,
        "raw_trace": {
            "path": str(LOG_PATH.relative_to(ROOT)),
            "bytes": len(raw_log),
            "sha256": sha256_bytes(raw_log),
            "redactions": ["repository-root"],
        },
        "sources": source_records(revision),
        "external_network_used": False,
        "private_user_data_used": False,
        "independent_automated_review_performed": True,
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


def validate_report(report: Any, raw_log: bytes | None = None) -> list[str]:
    if not isinstance(report, dict):
        return ["runtime.security.report_type"]
    failures: list[str] = []
    exact = {
        "schema_version": 1,
        "artifact_id": "story-21.2-product-security-evidence-map",
        "status": "pass-local-story-security-evidence",
        "task_ids": ["21.2.3.6", "RV-18"],
        "requirement_ids": list(REQUIREMENT_IDS),
        "mappings": [
            {"requirement_id": requirement_id, **MAPPINGS[requirement_id]}
            for requirement_id in REQUIREMENT_IDS
        ],
        "external_network_used": False,
        "private_user_data_used": False,
        "independent_automated_review_performed": True,
        "independent_human_review_performed": False,
        "manual_fuzzing_executed": False,
        "release_approved": False,
        "limitations": list(LIMITATIONS),
    }
    failures.extend(
        f"runtime.security.report_field.{field}"
        for field, expected in exact.items()
        if report.get(field) != expected
    )
    revision = report.get("source_revision")
    if not isinstance(revision, str) or REVISION.fullmatch(revision) is None:
        failures.append("runtime.security.report_revision")
    commands = report.get("verification_commands")
    identities = expected_command_identity()
    if not isinstance(commands, list) or len(commands) != len(identities):
        failures.append("runtime.security.report_commands")
    else:
        for record, identity in zip(commands, identities, strict=True):
            if (
                any(record.get(field) != value for field, value in identity.items())
                or record.get("exit_code") != 0
                or record.get("status") != "pass"
                or not isinstance(record.get("elapsed_ms"), int)
                or isinstance(record.get("elapsed_ms"), bool)
                or record["elapsed_ms"] <= 0
            ):
                failures.append("runtime.security.report_command")
                break
    trace = report.get("raw_trace")
    if (
        not isinstance(trace, dict)
        or trace.get("path") != str(LOG_PATH.relative_to(ROOT))
        or trace.get("redactions") != ["repository-root"]
        or not isinstance(trace.get("bytes"), int)
        or SHA256.fullmatch(str(trace.get("sha256", ""))) is None
    ):
        failures.append("runtime.security.report_trace")
    elif raw_log is not None and (
        trace["bytes"] != len(raw_log) or trace["sha256"] != sha256_bytes(raw_log)
    ):
        failures.append("runtime.security.trace_drift")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(
        SOURCE_PATHS
    ):
        failures.append("runtime.security.report_sources")
    elif any(
        not isinstance(item.get("bytes"), int)
        or item["bytes"] <= 0
        or SHA256.fullmatch(str(item.get("sha256", ""))) is None
        for item in sources
    ):
        failures.append("runtime.security.report_source_identity")
    digest = report.get("report_sha256")
    if not isinstance(digest, str) or SHA256.fullmatch(digest) is None:
        failures.append("runtime.security.report_digest")
    else:
        candidate = json.loads(json.dumps(report))
        candidate["report_sha256"] = ZERO_SHA256
        if digest != sha256_bytes(canonical_json_bytes(candidate)):
            failures.append("runtime.security.report_digest_drift")
    return failures


def validate_current(report: Any, raw_log: bytes) -> list[str]:
    failures = validate_report(report, raw_log)
    if not isinstance(report, dict):
        return failures
    revision = report.get("source_revision")
    if not isinstance(revision, str) or REVISION.fullmatch(revision) is None:
        return failures
    try:
        expected_sources = source_records(revision)
    except SecurityEvidenceError as error:
        failures.append(str(error))
    else:
        if report.get("sources") != expected_sources:
            failures.append("runtime.security.source_drift")
    return failures


def read_report() -> Any:
    try:
        return json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise SecurityEvidenceError("runtime.security.report_unavailable") from error


def read_log() -> bytes:
    try:
        return LOG_PATH.read_bytes()
    except OSError as error:
        raise SecurityEvidenceError("runtime.security.log_unavailable") from error


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    if arguments.write:
        revision = git_revision(arguments.source_revision)
        raw_log, commands = run_commands()
        report = seal_report(build_report(revision, raw_log, commands))
        atomic_write(LOG_PATH, raw_log)
        atomic_write(REPORT_PATH, canonical_json_bytes(report))
    failures = validate_current(read_report(), read_log())
    if failures:
        raise SecurityEvidenceError("; ".join(failures))
    print("Story 21.2 product-security evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
