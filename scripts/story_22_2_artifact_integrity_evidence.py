#!/usr/bin/env python3
"""Run and validate the Story 22.2 artifact-integrity campaign."""

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
REPORT_PATH: Final = OUTPUT_DIRECTORY / "native-artifact-integrity.json"
LOG_PATH: Final = OUTPUT_DIRECTORY / "native-artifact-integrity.log"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
SOURCE_PATHS: Final = (
    "kernel/engine/src/runtime_artifact.rs",
    "platforms/linux/src/runtime_artifact_store.rs",
    "shells/host/src/linux_coding_runtime.rs",
    "tests/test_planning_schemas.mjs",
    "scripts/story_22_2_artifact_integrity_evidence.py",
    "tests/test_story_22_2_artifact_integrity_evidence.py",
)
COMMANDS: Final = (
    {
        "id": "kernel-contracts",
        "argv": (
            "cargo",
            "test",
            "-p",
            "agentmage-kernel-engine",
            "--lib",
            "--locked",
            "runtime_artifact::tests::",
            "--",
            "--nocapture",
        ),
        "summary": "22 passed; 0 failed; 0 ignored",
        "tests": (
            "manifest_digest_or_reference_drift_fails_closed",
            "size_media_retention_and_preview_bounds_are_closed",
            "unknown_versions_and_expired_artifacts_fail_closed",
            "partial_and_identifier_colliding_publications_preserve_canonical_state",
            "publication_rejects_mismatched_producer_authority_before_staging",
            "story_21_2_artifact_canaries_require_an_exact_owner_bound_payload_read",
            "current_checkpoint_reference_prevents_release_and_collection",
            "missing_and_corrupt_payloads_quarantine_every_active_reference",
            "operator_view_is_path_free_complete_and_tracks_cleanup_state",
            "publication_deduplicates_without_broadening_owner_or_reference_state",
            "checkpoint_cursor_and_artifact_set_publish_atomically_and_reopen",
        ),
    },
    {
        "id": "linux-native-store",
        "argv": (
            "cargo",
            "test",
            "-p",
            "agentmage-platform-linux",
            "--lib",
            "--locked",
            "runtime_artifact_store::tests::",
            "--",
            "--nocapture",
        ),
        "summary": "9 passed; 0 failed; 2 ignored",
        "tests": (
            "stage_place_read_inventory_and_dedup_are_content_addressed",
            "staging_and_objects_are_encrypted_randomized_and_key_bound",
            "hard_links_and_open_handle_collection_do_not_disclose_plaintext",
            "staging_is_bounded_and_interrupted_objects_are_cleaned",
            "invalid_names_symlinks_modes_and_root_drift_fail_closed",
            "corruption_quarantine_and_verified_delete_leave_no_active_object",
            "fixed_namespace_substitution_fails_before_the_next_store_effect",
            "repository_evidence_namespace_is_never_private_artifact_authority",
            "story_22_2_native_crash_matrix_reconciles_every_artifact_boundary",
        ),
    },
    {
        "id": "generated-file-route",
        "argv": (
            "cargo",
            "test",
            "-p",
            "agentmage-host",
            "--lib",
            "--locked",
            "story_22_2_linux_generated_file_is_published_and_checkpoint_bound",
            "--",
            "--nocapture",
        ),
        "summary": "1 passed; 0 failed; 0 ignored",
        "tests": (
            "story_22_2_linux_generated_file_is_published_and_checkpoint_bound",
        ),
    },
    {
        "id": "public-schema-contracts",
        "argv": (
            "node",
            "--test",
            "--test-name-pattern=runtime artifact schemas reject path authority and lifecycle drift",
            "tests/test_planning_schemas.mjs",
        ),
        "summary": "pass 1",
        "tests": (
            "runtime artifact schemas reject path authority and lifecycle drift",
        ),
    },
)
COVERAGE: Final = {
    "digest-size-media-preview-retention": [
        "kernel-contracts:size_media_retention_and_preview_bounds_are_closed",
        "kernel-contracts:manifest_digest_or_reference_drift_fails_closed",
    ],
    "identity-reference-owner-checkpoint": [
        "kernel-contracts:story_21_2_artifact_canaries_require_an_exact_owner_bound_payload_read",
        "kernel-contracts:publication_rejects_mismatched_producer_authority_before_staging",
        "kernel-contracts:checkpoint_cursor_and_artifact_set_publish_atomically_and_reopen",
    ],
    "encryption-key-binding-and-plaintext-exclusion": [
        "linux-native-store:staging_and_objects_are_encrypted_randomized_and_key_bound",
        "linux-native-store:hard_links_and_open_handle_collection_do_not_disclose_plaintext",
    ],
    "missing-corrupt-quarantine-and-cleanup": [
        "kernel-contracts:missing_and_corrupt_payloads_quarantine_every_active_reference",
        "linux-native-store:corruption_quarantine_and_verified_delete_leave_no_active_object",
    ],
    "duplicate-collision-partial-and-oversized": [
        "kernel-contracts:partial_and_identifier_colliding_publications_preserve_canonical_state",
        "linux-native-store:stage_place_read_inventory_and_dedup_are_content_addressed",
        "linux-native-store:staging_is_bounded_and_interrupted_objects_are_cleaned",
    ],
    "expired-stale-and-unknown-version": [
        "kernel-contracts:unknown_versions_and_expired_artifacts_fail_closed",
        "kernel-contracts:current_checkpoint_reference_prevents_release_and_collection",
    ],
    "path-link-namespace-and-public-evidence-separation": [
        "linux-native-store:invalid_names_symlinks_modes_and_root_drift_fail_closed",
        "linux-native-store:fixed_namespace_substitution_fails_before_the_next_store_effect",
        "linux-native-store:repository_evidence_namespace_is_never_private_artifact_authority",
    ],
    "operator-projection-and-generated-file-route": [
        "kernel-contracts:operator_view_is_path_free_complete_and_tracks_cleanup_state",
        "generated-file-route:story_22_2_linux_generated_file_is_published_and_checkpoint_bound",
    ],
    "schema-lifecycle-and-path-mutations": [
        "public-schema-contracts:runtime artifact schemas reject path authority and lifecycle drift",
    ],
}


class ArtifactIntegrityEvidenceError(ValueError):
    """Raised when artifact-integrity evidence is unavailable, malformed, or stale."""


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def command_digest(argv: tuple[str, ...]) -> str:
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
        raise ArtifactIntegrityEvidenceError("runtime.artifact_integrity.source_revision")
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
        raise ArtifactIntegrityEvidenceError("runtime.artifact_integrity.source_unavailable")
    return result.stdout


def source_records(revision: str) -> list[dict[str, Any]]:
    return [
        {"path": relative, "bytes": len(content), "sha256": sha256_bytes(content)}
        for relative in SOURCE_PATHS
        for content in [git_blob(revision, relative)]
    ]


def run_campaign() -> tuple[str, list[dict[str, Any]]]:
    if platform.system() != "Linux":
        raise ArtifactIntegrityEvidenceError("runtime.artifact_integrity.platform")
    traces: list[str] = []
    results: list[dict[str, Any]] = []
    environment = {
        **os.environ,
        "CARGO_TERM_COLOR": "never",
        "LANG": "C",
        "LC_ALL": "C",
        "NO_COLOR": "1",
    }
    for case in COMMANDS:
        started = time.monotonic()
        result = subprocess.run(
            list(case["argv"]),
            cwd=ROOT,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=360,
            check=False,
            env=environment,
        )
        output = (result.stdout + result.stderr).replace(str(ROOT), "<repository-root>")
        if result.returncode or case["summary"] not in output:
            raise ArtifactIntegrityEvidenceError(
                f"runtime.artifact_integrity.command_failed.{case['id']}"
            )
        for test_name in case["tests"]:
            if test_name not in output:
                raise ArtifactIntegrityEvidenceError(
                    f"runtime.artifact_integrity.test_missing.{case['id']}"
                )
        elapsed_ms = max(1, int((time.monotonic() - started) * 1_000))
        results.append(
            {
                "id": case["id"],
                "command_id": command_digest(case["argv"]),
                "elapsed_ms": elapsed_ms,
                "exit_code": 0,
                "expected_summary": case["summary"],
                "required_tests": list(case["tests"]),
            }
        )
        traces.append(f"===== {case['id']} =====\n{output.rstrip()}\n")
    return "\n".join(traces), results


def build_report(
    revision: str, output: str, command_results: list[dict[str, Any]]
) -> dict[str, Any]:
    raw = output.encode("utf-8")
    return {
        "schema_version": 1,
        "artifact_id": "story-22.2-native-runtime-artifact-integrity",
        "source_revision": revision,
        "status": "pass-current-linux-source-boundary",
        "task_ids": ["22.2.1.3", "22.2.3.1", "RV-17", "RV-18"],
        "host": {"system": platform.system(), "machine": platform.machine()},
        "commands": command_results,
        "coverage": COVERAGE,
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
            "This campaign proves the named unit, native Linux source, generated-file route, and public-schema contracts; it is not installed-package or cross-platform evidence.",
            "The namespace-substitution tests are deterministic boundary attacks, not a concurrent file-level race campaign.",
            "Physical power loss, disk-full, device latency, controller failure, and filesystem corruption are not claimed.",
            "Windows and macOS native artifact-store evidence and independent cryptographic review remain open.",
            "Large command, test, and model artifact session reconstruction remains separate resume-campaign work.",
            "Manual fuzzing remains deferred and was not executed by this campaign.",
        ],
    }


def validate_report(report: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(report, dict):
        return ["runtime.artifact_integrity.report_type"]
    if report.get("schema_version") != 1 or report.get("artifact_id") != (
        "story-22.2-native-runtime-artifact-integrity"
    ):
        failures.append("runtime.artifact_integrity.report_identity")
    revision = report.get("source_revision")
    if not isinstance(revision, str) or REVISION.fullmatch(revision) is None:
        failures.append("runtime.artifact_integrity.report_revision")
    if report.get("status") != "pass-current-linux-source-boundary" or report.get(
        "task_ids"
    ) != ["22.2.1.3", "22.2.3.1", "RV-17", "RV-18"]:
        failures.append("runtime.artifact_integrity.report_disposition")
    if report.get("coverage") != COVERAGE:
        failures.append("runtime.artifact_integrity.report_coverage")
    commands = report.get("commands")
    if not isinstance(commands, list) or len(commands) != len(COMMANDS):
        failures.append("runtime.artifact_integrity.report_commands")
    else:
        for record, expected in zip(commands, COMMANDS, strict=True):
            if (
                record.get("id") != expected["id"]
                or record.get("command_id") != command_digest(expected["argv"])
                or record.get("expected_summary") != expected["summary"]
                or record.get("required_tests") != list(expected["tests"])
                or record.get("exit_code") != 0
                or not isinstance(record.get("elapsed_ms"), int)
                or record["elapsed_ms"] <= 0
            ):
                failures.append(f"runtime.artifact_integrity.command.{expected['id']}")
    raw_trace = report.get("raw_trace")
    if (
        not isinstance(raw_trace, dict)
        or raw_trace.get("path") != str(LOG_PATH.relative_to(ROOT))
        or raw_trace.get("redactions") != ["repository-root"]
    ):
        failures.append("runtime.artifact_integrity.report_trace")
    elif not LOG_PATH.is_file():
        failures.append("runtime.artifact_integrity.trace_missing")
    else:
        raw = LOG_PATH.read_bytes()
        if raw_trace.get("bytes") != len(raw) or raw_trace.get("sha256") != sha256_bytes(raw):
            failures.append("runtime.artifact_integrity.trace_drift")
        elif str(ROOT).encode("utf-8") in raw:
            failures.append("runtime.artifact_integrity.trace_unredacted")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(
        SOURCE_PATHS
    ):
        failures.append("runtime.artifact_integrity.report_sources")
    elif isinstance(revision, str) and REVISION.fullmatch(revision):
        for record in sources:
            content = git_blob(revision, record["path"])
            if (
                record.get("bytes") != len(content)
                or SHA256.fullmatch(str(record.get("sha256", ""))) is None
                or record["sha256"] != sha256_bytes(content)
            ):
                failures.append("runtime.artifact_integrity.source_drift")
                break
    for field in ("external_network_used", "private_user_data_used"):
        if report.get(field) is not False:
            failures.append(f"runtime.artifact_integrity.{field}")
    limitations = report.get("limitations")
    if not isinstance(limitations, list) or len(limitations) != 6:
        failures.append("runtime.artifact_integrity.report_limitations")
    return failures


def read_report() -> Any:
    try:
        return json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise ArtifactIntegrityEvidenceError(
            "runtime.artifact_integrity.report_unavailable"
        ) from error


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    if arguments.write:
        revision = git_revision(arguments.source_revision)
        output, command_results = run_campaign()
        atomic_write(LOG_PATH, output.encode("utf-8"))
        atomic_write(
            REPORT_PATH,
            canonical_json_bytes(build_report(revision, output, command_results)),
        )
    failures = validate_report(read_report())
    if failures:
        raise ArtifactIntegrityEvidenceError("; ".join(failures))
    print("Story 22.2 native artifact integrity evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
