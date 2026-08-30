#!/usr/bin/env python3
"""Build and validate Sub-task 11.2.3.3 new-family lifecycle evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-11/story-11.2"
RAW_PATH: Final = EVIDENCE_DIR / "storage-new-family-lifecycle-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "storage-new-family-lifecycle-report.json"
SOURCE_PATH: Final = ROOT / "kernel/engine/src/operational_store.rs"
LIFECYCLE_PATH: Final = ROOT / "kernel/engine/src/source_lifecycle.rs"
COMMANDS: Final = (
    (
        "cargo", "test", "-p", "agentmage-kernel-engine",
        "every_source_and_workflow_family_has_a_content_free_derived_export", "--locked",
    ),
    (
        "cargo", "test", "-p", "agentmage-kernel-engine", "materializations_bind", "--locked",
    ),
    (
        "cargo", "test", "-p", "agentmage-kernel-engine",
        "refresh_hold_expiry_delete_and_collection_are_atomic_and_reference_safe", "--locked",
    ),
    (
        "cargo", "test", "-p", "agentmage-kernel-engine",
        "exclusive_writer_and_encrypted_backup_are_verified", "--locked",
    ),
    (
        "cargo", "test", "-p", "agentmage-kernel-engine",
        "encrypted_backup_restores_only_to_a_verified_fresh_candidate", "--locked",
    ),
    (
        "cargo", "test", "-p", "agentmage-kernel-engine",
        "whole_store_cryptographic_erasure_consumes_key_scope_without_overwrite_claim", "--locked",
    ),
    (
        "cargo", "test", "-p", "agentmage-kernel-engine",
        "synthetic_canary_is_absent_from_encrypted_and_derived_artifacts", "--locked",
    ),
    (
        "cargo", "clippy", "-p", "agentmage-kernel-engine", "--all-targets",
        "--all-features", "--locked", "--", "-D", "warnings",
    ),
)
MARKERS: Final = (
    "every_source_and_workflow_family_has_a_content_free_derived_export ... ok",
    "workflow_materializations_bind_existing_run_session_event_and_receipt_authorities ... ok",
    "source_materializations_bind_one_existing_encrypted_payload_without_new_byte_store ... ok",
    "refresh_hold_expiry_delete_and_collection_are_atomic_and_reference_safe ... ok",
    "exclusive_writer_and_encrypted_backup_are_verified ... ok",
    "encrypted_backup_restores_only_to_a_verified_fresh_candidate ... ok",
    "whole_store_cryptographic_erasure_consumes_key_scope_without_overwrite_claim ... ok",
    "synthetic_canary_is_absent_from_encrypted_and_derived_artifacts ... ok",
)
SOURCE_MARKERS: Final = (
    "const DERIVED_EXPORT_QUERIES: &[DerivedExportQuery]",
    'family: "source_released_payloads"',
    'family: "workflow_terminal_diagnostics"',
    "assert_eq!(persisted_families.len(), 31);",
    '"record_json"',
    '"payload_sha256"',
    "assert_artifacts_exclude_canary(&path, &backup, &export, canary);",
)
LIFECYCLE_MARKERS: Final = (
    "fn refresh_hold_expiry_delete_and_collection_are_atomic_and_reference_safe()",
    "verify_source_lifecycle(&store).expect(\"verified lifecycle\");",
)
TRUTH: Final = {
    "synthetic_data_only": True,
    "new_family_count": 31,
    "new_family_export_coverage_complete": True,
    "content_free_evidence_complete": True,
    "encrypted_backup_restore_complete": True,
    "source_retention_erasure_complete": True,
    "whole_store_cryptographic_erasure_complete": True,
    "synthetic_canary_scan_complete": True,
    "terminal_diagnostic_export_complete": True,
    "exhaustive_crash_campaign_complete": False,
    "story_completion_claim": False,
    "sprint_completion_claim": False,
    "release_claim": "none",
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def artifact(path: str) -> dict[str, Any]:
    absolute = ROOT / path
    return {"path": path, "byte_length": absolute.stat().st_size, "sha256": sha256(absolute)}


def expected_report() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "agentmage-storage-new-family-lifecycle-evidence",
        "story_id": "11.2",
        "task_id": "11.2.3.3",
        "generated_on": "2026-08-30",
        "status": "pass-local-new-family-lifecycle",
        "canonical_store": "operational-store",
        "coverage": [
            "schema-derived-family-inventory",
            "content-free-derived-export",
            "encrypted-whole-store-backup",
            "verified-fresh-candidate-restore",
            "source-retention-release-delete-collection",
            "whole-store-cryptographic-erasure",
            "encrypted-artifact-and-diagnostic-canary-scan",
        ],
        "commands": [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "artifacts": [
            artifact("kernel/engine/src/operational_store.rs"),
            artifact("kernel/engine/src/source_lifecycle.rs"),
            artifact("docs/verification/story-11-2-storage-new-family-lifecycle-evidence.md"),
            artifact("scripts/storage_new_family_lifecycle_evidence.py"),
            artifact("tests/test_storage_new_family_lifecycle_evidence.py"),
            artifact(RAW_PATH.relative_to(ROOT).as_posix()),
        ],
        "product_truth": dict(TRUTH),
    }


def render(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n"


def validate_source(source: str, lifecycle: str) -> list[str]:
    failures = [
        f"source missing lifecycle marker: {marker}"
        for marker in SOURCE_MARKERS
        if marker not in source
    ]
    failures.extend(
        f"source lifecycle missing marker: {marker}"
        for marker in LIFECYCLE_MARKERS
        if marker not in lifecycle
    )
    start = source.find("const DERIVED_EXPORT_QUERIES")
    end = source.find("fn derived_export_rows(", start)
    block = source[start:end]
    families = re.findall(r'family: "((?:source|workflow)_[^"]+)"', block)
    if len(families) != 31 or len(set(families)) != 31:
        failures.append("derived export does not cover exactly 31 unique new families")
    for prohibited in ("record_json FROM", "payload_sha256 FROM", "source_sha256 FROM"):
        if prohibited in block:
            failures.append(f"derived export selects prohibited content field: {prohibited}")
    return failures


def validate_raw(value: str) -> list[str]:
    failures = [f"raw results missing marker: {marker}" for marker in MARKERS if marker not in value]
    for prohibited in ("test result: FAILED", "error: could not compile", "warning:"):
        if prohibited in value:
            failures.append(f"raw results contain prohibited marker: {prohibited}")
    return failures


def validate_report(value: Any) -> list[str]:
    if value != expected_report():
        return ["storage new-family lifecycle report is stale, incomplete, reordered, or widened"]
    if value.get("product_truth") != TRUTH:
        return ["storage new-family lifecycle product truth was widened"]
    return []


def capture() -> tuple[str, int]:
    chunks: list[str] = []
    for command in COMMANDS:
        result = subprocess.run(
            command, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
            text=True, check=False,
        )
        chunks.append(f"$ {' '.join(command)}\n{result.stdout.rstrip(chr(10))}\n")
        if result.returncode != 0:
            return "".join(chunks), result.returncode
    return "".join(chunks), 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    if args.write:
        raw, returncode = capture()
        if returncode != 0:
            sys.stderr.write(raw)
            return 1
        failures = validate_source(
            SOURCE_PATH.read_text(encoding="utf-8"),
            LIFECYCLE_PATH.read_text(encoding="utf-8"),
        ) + validate_raw(raw)
        if failures:
            for failure in failures:
                print(f"storage new-family lifecycle evidence build failed: {failure}", file=sys.stderr)
            return 1
        EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
        RAW_PATH.write_text(raw, encoding="utf-8")
        REPORT_PATH.write_text(render(expected_report()), encoding="utf-8")

    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        source = SOURCE_PATH.read_text(encoding="utf-8")
        lifecycle = LIFECYCLE_PATH.read_text(encoding="utf-8")
    except (OSError, json.JSONDecodeError) as error:
        print(f"storage new-family lifecycle evidence validation failed: {error}", file=sys.stderr)
        return 1
    failures = validate_source(source, lifecycle) + validate_raw(raw) + validate_report(report)
    if failures:
        for failure in failures:
            print(f"storage new-family lifecycle evidence validation failed: {failure}", file=sys.stderr)
        return 1
    print("Sub-task 11.2.3.3 storage new-family lifecycle validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
