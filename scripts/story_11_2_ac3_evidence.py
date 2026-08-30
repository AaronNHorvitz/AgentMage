#!/usr/bin/env python3
"""Build and validate the Story 11.2 AC3 lifecycle-acceptance record."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-11/story-11.2"
STORY_11_1_DIR: Final = ROOT / "artifacts/sprints/sprint-11/story-11.1"
RAW_PATH: Final = EVIDENCE_DIR / "story-ac3-lifecycle-acceptance-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "story-ac3-lifecycle-acceptance-report.json"
COMMANDS: Final = (
    ("python3", "scripts/storage_new_family_lifecycle_evidence.py"),
    ("python3", "scripts/source_lifecycle_transactions_evidence.py"),
    ("python3", "scripts/storage_security_evidence.py"),
)
MARKERS: Final = (
    "Sub-task 11.2.3.3 storage new-family lifecycle validated",
    "Sub-task 11.2.1.3 atomic source lifecycle transactions validated",
    "Sub-task 11.2.4.3 retained storage security evidence validated",
)
SURFACES: Final = (
    "sqlcipher-main",
    "sqlcipher-wal",
    "sqlcipher-shm",
    "encrypted-backup",
    "content-free-export",
    "receipt-and-diagnostic-strings",
)
RETAINED_PATHS: Final = (
    "kernel/engine/src/operational_store.rs",
    "kernel/engine/src/source_lifecycle.rs",
    "kernel/engine/src/runtime_artifact.rs",
    "artifacts/sprints/sprint-11/story-11.2/storage-new-family-lifecycle-report.json",
    "artifacts/sprints/sprint-11/story-11.2/storage-new-family-lifecycle-results.log",
    "artifacts/sprints/sprint-11/story-11.2/source-lifecycle-transactions-report.json",
    "artifacts/sprints/sprint-11/story-11.2/source-lifecycle-transactions-results.log",
    "artifacts/sprints/sprint-11/story-11.2/storage-security-evidence-report.json",
    "artifacts/sprints/sprint-11/story-11.2/storage-security-evidence-results.log",
    "artifacts/sprints/sprint-11/story-11.1/s-011-st01.json",
    "artifacts/sprints/sprint-11/story-11.1/crash-canary-results.json",
)
TRUTH: Final = {
    "current_encrypted_operational_store_scope_complete": True,
    "source_workflow_family_count": 31,
    "whole_store_backup_complete": True,
    "fresh_candidate_restore_complete": True,
    "source_and_payload_reference_reconciliation_complete": True,
    "shared_payload_preservation_complete": True,
    "retention_hold_expiry_release_delete_complete": True,
    "content_free_export_coverage_complete": True,
    "unauthorized_raw_restricted_data_matches": 0,
    "raw_canary_retained": False,
    "private_user_data_used": False,
    "synthetic_data_only": True,
    "complete_product_uninstall_and_external_copy_cleanup": False,
    "physical_remanence_complete": False,
    "all_later_active_surfaces_complete": False,
    "cross_platform_complete": False,
    "independent_review_complete": False,
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
        "record_type": "agentmage-story-acceptance-evidence",
        "story_id": "11.2",
        "criterion_id": "11.2.AC3",
        "generated_on": "2026-08-30",
        "status": "pass-local-current-encrypted-operational-store-scope",
        "commands": [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "scanned_surface_classes": list(SURFACES),
        "retained_evidence": [artifact(path) for path in RETAINED_PATHS],
        "artifacts": [
            artifact("docs/verification/story-11-2-ac3-lifecycle-acceptance.md"),
            artifact("scripts/story_11_2_ac3_evidence.py"),
            artifact("tests/test_story_11_2_ac3_evidence.py"),
            artifact(RAW_PATH.relative_to(ROOT).as_posix()),
        ],
        "acceptance_truth": dict(TRUTH),
        "limitations": [
            "complete product uninstall and separately managed copies remain later gates",
            "physical remanence and later active logging, model-context, and adapter surfaces are not represented",
            "native cross-platform, installed-package, independent-review, Story, Sprint, packaging, and release completion are not claimed",
        ],
    }


def render(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n"


def validate_retained() -> list[str]:
    return [f"missing retained evidence: {path}" for path in RETAINED_PATHS if not (ROOT / path).is_file()]


def validate_upstream_reports() -> list[str]:
    failures: list[str] = []
    expectations = (
        (
            EVIDENCE_DIR / "storage-new-family-lifecycle-report.json",
            "product_truth",
            {
                "new_family_count": 31,
                "new_family_export_coverage_complete": True,
                "encrypted_backup_restore_complete": True,
                "source_retention_erasure_complete": True,
                "synthetic_canary_scan_complete": True,
                "content_free_evidence_complete": True,
            },
        ),
        (
            EVIDENCE_DIR / "source-lifecycle-transactions-report.json",
            "transaction_contract",
            {
                "release_updates_source_and_runtime_references_together": True,
                "shared_live_payloads_survive_collection": True,
                "deletion_precedes_existing_payload_reconciliation": True,
                "held_expiry_is_preserved": True,
            },
        ),
        (
            EVIDENCE_DIR / "storage-security-evidence-report.json",
            "product_truth",
            {
                "canary_scans_retained": True,
                "cleanup_evidence_retained": True,
                "encrypted_page_scans_retained": True,
                "raw_canary_retained": False,
                "private_user_data_used": False,
            },
        ),
        (
            STORY_11_1_DIR / "s-011-st01.json",
            "claims",
            {
                "reachable_surfaces_closed": 12,
                "plaintext_fallback_paths_observed": 0,
                "raw_canary_values_retained_in_evidence": 0,
                "unauthorized_model_context_paths_observed": 0,
            },
        ),
    )
    for path, section, required in expectations:
        try:
            value = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            failures.append(f"cannot read upstream report {path.relative_to(ROOT)}: {error}")
            continue
        actual = value.get(section, {})
        for field, expected in required.items():
            if actual.get(field) != expected:
                failures.append(f"upstream report {path.name} has invalid {section}.{field}")
    try:
        canary = json.loads((STORY_11_1_DIR / "s-011-st01.json").read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return failures
    if canary.get("private_user_data_used") is not False or canary.get("external_network_used") is not False:
        failures.append("secret-canary evidence used private data or external network")
    return failures


def validate_raw(value: str) -> list[str]:
    failures = [f"raw results missing marker: {marker}" for marker in MARKERS if marker not in value]
    for prohibited in ("validation failed", "Traceback", "FAILED", "raw canary"):
        if prohibited in value:
            failures.append(f"raw results contain prohibited marker: {prohibited}")
    return failures


def validate_report(value: Any) -> list[str]:
    if value != expected_report():
        return ["Story 11.2 AC3 report is stale, incomplete, reordered, or widened"]
    return []


def capture() -> tuple[str, int]:
    chunks: list[str] = []
    for command in COMMANDS:
        result = subprocess.run(command, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, check=False)
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
        failures = validate_retained() + validate_upstream_reports() + validate_raw(raw)
        if failures:
            for failure in failures:
                print(f"Story 11.2 AC3 evidence build failed: {failure}", file=sys.stderr)
            return 1
        EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
        RAW_PATH.write_text(raw, encoding="utf-8")
        REPORT_PATH.write_text(render(expected_report()), encoding="utf-8")
    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(f"Story 11.2 AC3 evidence validation failed: {error}", file=sys.stderr)
        return 1
    failures = validate_retained() + validate_upstream_reports() + validate_raw(raw) + validate_report(report)
    if failures:
        for failure in failures:
            print(f"Story 11.2 AC3 evidence validation failed: {failure}", file=sys.stderr)
        return 1
    print("Story acceptance criterion 11.2.AC3 lifecycle and restricted-data reconciliation validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
