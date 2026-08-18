#!/usr/bin/env python3
"""Build and validate the Story 22.2 product-security evidence map."""

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
OUTPUT_DIRECTORY: Final = ROOT / "artifacts/sprints/sprint-22/story-22.2"
REPORT_PATH: Final = OUTPUT_DIRECTORY / "security-evidence-map.json"
LOG_PATH: Final = OUTPUT_DIRECTORY / "security-evidence.log"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
ZERO_SHA256: Final = "0" * 64
REQUIREMENT_IDS: Final = (
    "SR-DAT-001", "SR-DAT-002", "SR-DAT-003", "SR-DAT-004",
    "SR-DAT-005", "SR-DAT-006", "SR-DAT-007", "SR-DAT-008",
    "SR-DAT-009", "SR-DAT-010", "SR-DAT-011", "SR-DAT-012",
    "SR-ACC-001", "SR-ACC-002", "SR-ACC-003", "SR-ACC-004",
    "SR-ACC-005", "SR-ACC-006", "SR-ACC-007", "SR-ACC-008",
    "SR-OPS-003", "SR-TST-005", "SR-TST-006", "RV-17", "RV-18",
)
SECURITY_MAP: Final = "docs/verification/task-22-2-3-6-product-security-evidence.md"
SECURITY_REVIEW: Final = "SECURITY-REVIEW.md"
ARCHITECTURE: Final = "docs/architecture/runtime-artifact-lifecycle.md"
CONTRACT: Final = "kernel/contracts/src/runtime_artifact.rs"
ENGINE: Final = "kernel/engine/src/runtime_artifact.rs"
MIGRATION: Final = "kernel/engine/migrations/operational-store/0007-runtime-artifacts.sql"
LINUX_STORE: Final = "platforms/linux/src/runtime_artifact_store.rs"
SCHEMAS: Final = (
    "schemas/runtime/runtime-artifact-reference.schema.json",
    "schemas/runtime/runtime-artifact-manifest.schema.json",
    "schemas/runtime/runtime-artifact-operator-view.schema.json",
    "schemas/runtime/runtime-resume-binding.schema.json",
)
CRASH_REPORT: Final = "artifacts/sprints/sprint-22/story-22.2/native-artifact-crash-matrix.json"
CRASH_LOG: Final = "artifacts/sprints/sprint-22/story-22.2/native-artifact-crash-matrix.log"
INTEGRITY_REPORT: Final = "artifacts/sprints/sprint-22/story-22.2/native-artifact-integrity.json"
INTEGRITY_LOG: Final = "artifacts/sprints/sprint-22/story-22.2/native-artifact-integrity.log"
PRESSURE_REPORT: Final = "artifacts/sprints/sprint-22/story-22.2/native-artifact-pressure.json"
PRESSURE_LOG: Final = "artifacts/sprints/sprint-22/story-22.2/native-artifact-pressure.log"
RESUME_REPORT: Final = "artifacts/sprints/sprint-22/story-22.2/native-artifact-resume.json"
RESUME_LOG: Final = "artifacts/sprints/sprint-22/story-22.2/native-artifact-resume.log"
BOUNDARY_REVIEW: Final = "artifacts/sprints/sprint-22/story-22.2/artifact-boundary-review.json"
SOURCE_PATHS: Final = (
    SECURITY_MAP, SECURITY_REVIEW, ARCHITECTURE, "TASKS.md", CONTRACT, ENGINE,
    MIGRATION, LINUX_STORE, *SCHEMAS,
    "scripts/runtime_artifact_boundary_review.py",
    "tests/test_runtime_artifact_boundary_review.py",
    "scripts/story_22_2_artifact_crash_evidence.py",
    "scripts/story_22_2_artifact_integrity_evidence.py",
    "scripts/story_22_2_artifact_pressure_evidence.py",
    "scripts/story_22_2_artifact_resume_evidence.py",
    "scripts/story_22_2_security_evidence.py",
    "tests/test_story_22_2_security_evidence.py",
    CRASH_REPORT, CRASH_LOG, INTEGRITY_REPORT, INTEGRITY_LOG,
    PRESSURE_REPORT, PRESSURE_LOG, RESUME_REPORT, RESUME_LOG, BOUNDARY_REVIEW,
)


def evidence(*paths: str) -> list[str]:
    return list(paths)


MAPPINGS: Final = {
    "SR-DAT-001": {"contribution": "demonstrated-story-scope", "evidence": evidence(*SCHEMAS, MIGRATION), "remaining": "Product-wide runtime inventory comparison."},
    "SR-DAT-002": {"contribution": "demonstrated-story-scope", "evidence": evidence(CONTRACT, ENGINE, INTEGRITY_REPORT), "remaining": "Installed-client observation and complete policy composition."},
    "SR-DAT-003": {"contribution": "partial-story-evidence", "evidence": evidence(CONTRACT, LINUX_STORE, INTEGRITY_REPORT), "remaining": "Product-wide memory, swap, crash, export, and OS telemetry scan."},
    "SR-DAT-004": {"contribution": "demonstrated-linux-story-scope", "evidence": evidence(ENGINE, LINUX_STORE, INTEGRITY_REPORT), "remaining": "Supported-platform key-store failure campaigns."},
    "SR-DAT-005": {"contribution": "partial-story-evidence", "evidence": evidence(ARCHITECTURE, LINUX_STORE, BOUNDARY_REVIEW), "remaining": "Independent cryptographic provider and construction review."},
    "SR-DAT-006": {"contribution": "demonstrated-story-scope", "evidence": evidence(ARCHITECTURE, SECURITY_MAP, BOUNDARY_REVIEW), "remaining": "Release documentation and diagnostics claim scan."},
    "SR-DAT-007": {"contribution": "partial-story-evidence", "evidence": evidence(CONTRACT, ENGINE, LINUX_STORE), "remaining": "Live platform key-service and process instrumentation evidence."},
    "SR-DAT-008": {"contribution": "partial-inherited-evidence", "evidence": evidence(ARCHITECTURE, BOUNDARY_REVIEW), "remaining": "Release package and runtime CBOM reconciliation."},
    "SR-DAT-009": {"contribution": "partial-story-evidence", "evidence": evidence(ARCHITECTURE, LINUX_STORE), "remaining": "Provider substitution, rotation, migration, and rollback campaign."},
    "SR-DAT-010": {"contribution": "partial-story-evidence", "evidence": evidence(CONTRACT, ENGINE, PRESSURE_REPORT), "remaining": "Export, backup, restore, and installed retention lifecycle."},
    "SR-DAT-011": {"contribution": "partial-story-evidence", "evidence": evidence(ARCHITECTURE, LINUX_STORE, INTEGRITY_REPORT), "remaining": "Live key destruction and physical-media verification."},
    "SR-DAT-012": {"contribution": "owned-by-later-gate", "evidence": evidence(ARCHITECTURE, LINUX_STORE), "remaining": "Installed uninstall and residue campaign on every supported platform."},
    "SR-ACC-001": {"contribution": "partial-story-evidence", "evidence": evidence(CONTRACT, ENGINE, RESUME_REPORT), "remaining": "CapabilityGrant-mediated product operation composition."},
    "SR-ACC-002": {"contribution": "partial-story-evidence", "evidence": evidence(CONTRACT, ENGINE, INTEGRITY_REPORT), "remaining": "Complete signed grant field mutation campaign."},
    "SR-ACC-003": {"contribution": "partial-story-evidence", "evidence": evidence(ENGINE, CRASH_REPORT, RESUME_REPORT), "remaining": "Atomic one-use grant consumption at every artifact effect."},
    "SR-ACC-004": {"contribution": "demonstrated-story-scope", "evidence": evidence(*SCHEMAS, CONTRACT, INTEGRITY_REPORT), "remaining": "Product-wide typed-path integration."},
    "SR-ACC-005": {"contribution": "demonstrated-linux-story-scope", "evidence": evidence(LINUX_STORE, INTEGRITY_REPORT, BOUNDARY_REVIEW), "remaining": "Concurrent rename, link, and mount campaign on installed platforms."},
    "SR-ACC-006": {"contribution": "partial-story-evidence", "evidence": evidence(LINUX_STORE, INTEGRITY_REPORT, BOUNDARY_REVIEW), "remaining": "Complete ambient and adjacent-root canary campaign."},
    "SR-ACC-007": {"contribution": "partial-story-evidence", "evidence": evidence(ENGINE, RESUME_REPORT), "remaining": "Installed user approval, cancellation, diagnostics, and recovery interaction."},
    "SR-ACC-008": {"contribution": "demonstrated-story-scope", "evidence": evidence(ARCHITECTURE, LINUX_STORE, INTEGRITY_REPORT), "remaining": "Product-wide prompt-injection corpus and interface composition."},
    "SR-OPS-003": {"contribution": "demonstrated-story-scope", "evidence": evidence(CRASH_LOG, INTEGRITY_LOG, PRESSURE_LOG, RESUME_LOG), "remaining": "Product-wide memory, crash, export, and OS telemetry scan."},
    "SR-TST-005": {"contribution": "partial-story-evidence", "evidence": evidence(CRASH_REPORT, CRASH_LOG, RESUME_REPORT), "remaining": "One hundred integrated artifact resumes plus power and device faults."},
    "SR-TST-006": {"contribution": "partial-story-evidence", "evidence": evidence(PRESSURE_REPORT, PRESSURE_LOG, INTEGRITY_REPORT), "remaining": "Physical latency, larger populations, installed interfaces, and manual fuzzing."},
    "RV-17": {"contribution": "demonstrated-local-story-scope", "evidence": evidence(CRASH_REPORT, RESUME_REPORT, PRESSURE_REPORT), "remaining": "Full product transition set and physical-fault campaign."},
    "RV-18": {"contribution": "demonstrated-local-story-scope", "evidence": evidence(CRASH_REPORT, INTEGRITY_REPORT, PRESSURE_REPORT, RESUME_REPORT, BOUNDARY_REVIEW), "remaining": "Product-wide event coverage and independent human review."},
}
DOCUMENT_FRAGMENTS: Final = (
    "Mapped controls and review protocols: **25 of 25**.",
    "This result completes the local evidence-organization obligation for Sub-task",
    "independent automated artifact-boundary review",
    "Physical filesystem or SQLite syscall interruption",
    "Manual fuzzing remains deliberately deferred",
    "No result in this document enables a model, interface, supported platform,",
)
COMMAND_SPECS: Final = (
    ("crash-evidence", ("python3", "scripts/story_22_2_artifact_crash_evidence.py"), "Story 22.2 native artifact crash evidence: pass"),
    ("integrity-evidence", ("python3", "scripts/story_22_2_artifact_integrity_evidence.py"), "Story 22.2 native artifact integrity evidence: pass"),
    ("resume-evidence", ("python3", "scripts/story_22_2_artifact_resume_evidence.py"), "Story 22.2 native artifact resume evidence: pass"),
    ("pressure-evidence", ("python3", "scripts/story_22_2_artifact_pressure_evidence.py"), "Story 22.2 native artifact pressure evidence: pass"),
    ("independent-boundary-review", ("python3", "scripts/runtime_artifact_boundary_review.py"), "Story 22.2 independent automated artifact review: pass"),
    ("security-evidence-tests", ("python3", "-m", "unittest", "tests.test_story_22_2_security_evidence", "tests.test_runtime_artifact_boundary_review"), "OK"),
    ("artifact-canaries", ("cargo", "test", "-p", "agentmage-kernel-engine", "--lib", "--locked", "runtime_artifact::tests::story_21_2_artifact_canaries_require_an_exact_owner_bound_payload_read", "--", "--exact", "--nocapture"), "1 passed; 0 failed"),
)
LIMITATIONS: Final = (
    "This closes local Story 22.2 security-evidence organization, not any product or release security requirement.",
    "The independent reviewer is an automated implementation with separate rules, not an independent human or cryptographic review.",
    "Physical storage faults, syscall interruption, installed clients, Windows storage, and real-model sessions are not exercised.",
    "Quarantined-payload operator recovery, concurrent collection, and larger unique-object populations remain open.",
    "Manual fuzzing remains deferred and was not executed.",
    "No model, interface, platform, package, deployment, or release is enabled or approved by this evidence.",
    "Raw command output replaces the absolute checkout root with the literal <repository-root>.",
)


class SecurityEvidenceError(ValueError):
    """Raised when Story 22.2 security evidence is missing or invalid."""


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def git_revision(candidate: str) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"], cwd=ROOT,
        capture_output=True, text=True, timeout=30, check=False,
    )
    revision = result.stdout.strip()
    if result.returncode or REVISION.fullmatch(revision) is None:
        raise SecurityEvidenceError("runtime.artifact_security.source_revision")
    return revision


def git_blob(revision: str, relative: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{relative}"], cwd=ROOT,
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=60, check=False,
    )
    if result.returncode or not result.stdout:
        raise SecurityEvidenceError(f"runtime.artifact_security.source_unavailable.{relative}")
    return result.stdout


def source_records(revision: str) -> list[dict[str, Any]]:
    values = {path: git_blob(revision, path) for path in SOURCE_PATHS}
    failures = validate_source_values({path: value.decode("utf-8") for path, value in values.items()})
    if failures:
        raise SecurityEvidenceError("; ".join(failures))
    return [
        {"path": path, "bytes": len(values[path]), "sha256": sha256_bytes(values[path])}
        for path in SOURCE_PATHS
    ]


def validate_source_values(values: dict[str, str]) -> list[str]:
    if set(values) != set(SOURCE_PATHS):
        return ["runtime.artifact_security.source_set"]
    failures: list[str] = []
    document = values[SECURITY_MAP]
    registry = values[SECURITY_REVIEW]
    for fragment in DOCUMENT_FRAGMENTS:
        if document.count(fragment) != 1:
            failures.append("runtime.artifact_security.document_fragment")
    if tuple(MAPPINGS) != REQUIREMENT_IDS:
        failures.append("runtime.artifact_security.mapping_order")
    for requirement_id, mapping in MAPPINGS.items():
        marker = f"`{requirement_id}`"
        if marker not in registry:
            failures.append(f"runtime.artifact_security.registry.{requirement_id}")
        if document.count(marker) != 1:
            failures.append(f"runtime.artifact_security.mapping.{requirement_id}")
        paths = mapping.get("evidence")
        if not isinstance(paths, list) or not paths:
            failures.append(f"runtime.artifact_security.mapping_empty.{requirement_id}")
        elif any(path not in SOURCE_PATHS for path in paths):
            failures.append(f"runtime.artifact_security.mapping_path.{requirement_id}")
    expected_reports = {
        CRASH_REPORT: (CRASH_LOG, "pass-current-linux-native-source-boundary"),
        INTEGRITY_REPORT: (INTEGRITY_LOG, "pass-current-linux-source-boundary"),
        PRESSURE_REPORT: (PRESSURE_LOG, "pass-current-linux-native-reference-host"),
        RESUME_REPORT: (RESUME_LOG, "partial-pass-current-linux-native-source-boundary"),
    }
    for path, (log_path, expected_status) in expected_reports.items():
        try:
            report = json.loads(values[path])
        except json.JSONDecodeError:
            failures.append(f"runtime.artifact_security.artifact_json.{path}")
            continue
        trace = report.get("raw_trace", {})
        if report.get("status") != expected_status:
            failures.append(f"runtime.artifact_security.artifact_status.{path}")
        if report.get("external_network_used") is not False or report.get("private_user_data_used") is not False:
            failures.append(f"runtime.artifact_security.artifact_scope.{path}")
        if trace.get("sha256") != sha256_bytes(values[log_path].encode("utf-8")):
            failures.append(f"runtime.artifact_security.artifact_trace.{path}")
    try:
        review = json.loads(values[BOUNDARY_REVIEW])
    except json.JSONDecodeError:
        failures.append("runtime.artifact_security.review_json")
    else:
        if (
            review.get("status") != "pass-independent-automated-source-boundary-review"
            or review.get("independent_human_review_performed") is not False
            or review.get("independent_cryptographic_review_performed") is not False
            or review.get("release_approved") is not False
        ):
            failures.append("runtime.artifact_security.review_scope")
    if "- [x] **Sub-task 22.2.3.6 - Product security evidence:**" not in values["TASKS.md"]:
        failures.append("runtime.artifact_security.task_status")
    root_marker = str(ROOT)
    for path in (CRASH_LOG, INTEGRITY_LOG, PRESSURE_LOG, RESUME_LOG):
        if root_marker in values[path]:
            failures.append(f"runtime.artifact_security.unredacted_root.{path}")
    return failures


def command_sha256(arguments: tuple[str, ...]) -> str:
    return sha256_bytes("\0".join(arguments).encode("utf-8"))


def expected_command_identity() -> list[dict[str, Any]]:
    return [
        {
            "command_id": command_id,
            "argv_sha256": command_sha256(arguments),
            "expected_marker_sha256": sha256_bytes(marker.encode("utf-8")),
        }
        for command_id, arguments, marker in COMMAND_SPECS
    ]


def run_commands() -> tuple[bytes, list[dict[str, Any]]]:
    traces: list[str] = []
    records: list[dict[str, Any]] = []
    for command_id, arguments, marker in COMMAND_SPECS:
        started = time.monotonic()
        result = subprocess.run(
            list(arguments), cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            text=True, timeout=900, check=False,
            env={**os.environ, "CARGO_TERM_COLOR": "never", "LANG": "C", "LC_ALL": "C"},
        )
        output = (result.stdout + result.stderr).replace(str(ROOT), "<repository-root>")
        if result.returncode or marker not in output:
            raise SecurityEvidenceError(f"runtime.artifact_security.command_failed.{command_id}")
        elapsed_ms = max(1, int((time.monotonic() - started) * 1000))
        traces.append(f"=== {command_id} ===\n$ {shlex.join(arguments)}\n{output}")
        identity = next(item for item in expected_command_identity() if item["command_id"] == command_id)
        records.append({**identity, "elapsed_ms": elapsed_ms, "exit_code": 0, "status": "pass"})
    return "\n".join(traces).encode("utf-8"), records


def build_report(revision: str, raw_log: bytes, commands: list[dict[str, Any]]) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "artifact_id": "story-22.2-product-security-evidence-map",
        "source_revision": revision,
        "status": "pass-local-story-security-evidence",
        "task_ids": ["22.2.3.6", "RV-17", "RV-18"],
        "requirement_ids": list(REQUIREMENT_IDS),
        "mappings": [
            {"requirement_id": requirement_id, **MAPPINGS[requirement_id]}
            for requirement_id in REQUIREMENT_IDS
        ],
        "verification_commands": commands,
        "raw_trace": {
            "path": str(LOG_PATH.relative_to(ROOT)), "bytes": len(raw_log),
            "sha256": sha256_bytes(raw_log), "redactions": ["repository-root"],
        },
        "sources": source_records(revision),
        "external_network_used": False,
        "private_user_data_used": False,
        "independent_automated_review_performed": True,
        "independent_human_review_performed": False,
        "independent_cryptographic_review_performed": False,
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
        return ["runtime.artifact_security.report_type"]
    exact = {
        "schema_version": 1,
        "artifact_id": "story-22.2-product-security-evidence-map",
        "status": "pass-local-story-security-evidence",
        "task_ids": ["22.2.3.6", "RV-17", "RV-18"],
        "requirement_ids": list(REQUIREMENT_IDS),
        "mappings": [{"requirement_id": identifier, **MAPPINGS[identifier]} for identifier in REQUIREMENT_IDS],
        "external_network_used": False,
        "private_user_data_used": False,
        "independent_automated_review_performed": True,
        "independent_human_review_performed": False,
        "independent_cryptographic_review_performed": False,
        "manual_fuzzing_executed": False,
        "release_approved": False,
        "limitations": list(LIMITATIONS),
    }
    failures = [
        f"runtime.artifact_security.report_field.{field}"
        for field, expected in exact.items() if report.get(field) != expected
    ]
    revision = report.get("source_revision")
    if not isinstance(revision, str) or REVISION.fullmatch(revision) is None:
        failures.append("runtime.artifact_security.report_revision")
    commands = report.get("verification_commands")
    identities = expected_command_identity()
    if not isinstance(commands, list) or len(commands) != len(identities):
        failures.append("runtime.artifact_security.report_commands")
    else:
        for record, identity in zip(commands, identities, strict=True):
            if (
                any(record.get(field) != value for field, value in identity.items())
                or record.get("exit_code") != 0 or record.get("status") != "pass"
                or not isinstance(record.get("elapsed_ms"), int)
                or isinstance(record.get("elapsed_ms"), bool) or record["elapsed_ms"] <= 0
            ):
                failures.append("runtime.artifact_security.report_command")
                break
    trace = report.get("raw_trace")
    if (
        not isinstance(trace, dict)
        or trace.get("path") != str(LOG_PATH.relative_to(ROOT))
        or trace.get("redactions") != ["repository-root"]
        or not isinstance(trace.get("bytes"), int)
        or SHA256.fullmatch(str(trace.get("sha256", ""))) is None
    ):
        failures.append("runtime.artifact_security.report_trace")
    elif raw_log is not None and (
        trace["bytes"] != len(raw_log) or trace["sha256"] != sha256_bytes(raw_log)
    ):
        failures.append("runtime.artifact_security.trace_drift")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(SOURCE_PATHS):
        failures.append("runtime.artifact_security.report_sources")
    elif any(
        not isinstance(item.get("bytes"), int) or item["bytes"] <= 0
        or SHA256.fullmatch(str(item.get("sha256", ""))) is None for item in sources
    ):
        failures.append("runtime.artifact_security.report_source_identity")
    digest = report.get("report_sha256")
    if not isinstance(digest, str) or SHA256.fullmatch(digest) is None:
        failures.append("runtime.artifact_security.report_digest")
    else:
        candidate = json.loads(json.dumps(report))
        candidate["report_sha256"] = ZERO_SHA256
        if digest != sha256_bytes(canonical_json_bytes(candidate)):
            failures.append("runtime.artifact_security.report_digest_drift")
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
            failures.append("runtime.artifact_security.source_drift")
    return failures


def read_report() -> Any:
    try:
        return json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise SecurityEvidenceError("runtime.artifact_security.report_unavailable") from error


def read_log() -> bytes:
    try:
        return LOG_PATH.read_bytes()
    except OSError as error:
        raise SecurityEvidenceError("runtime.artifact_security.log_unavailable") from error


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    if arguments.write:
        revision = git_revision(arguments.source_revision)
        raw_log, commands = run_commands()
        report = seal_report(build_report(revision, raw_log, commands))
        if failures := validate_report(report, raw_log):
            raise SecurityEvidenceError("; ".join(failures))
        atomic_write(LOG_PATH, raw_log)
        atomic_write(REPORT_PATH, canonical_json_bytes(report))
    failures = validate_current(read_report(), read_log())
    if failures:
        raise SecurityEvidenceError("; ".join(failures))
    print("Story 22.2 product security evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
