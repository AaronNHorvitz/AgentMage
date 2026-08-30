#!/usr/bin/env python3
"""Build and validate Sub-task 11.2.4.3 retained security evidence."""

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
RAW_PATH: Final = EVIDENCE_DIR / "storage-security-evidence-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "storage-security-evidence-report.json"
COMMANDS: Final = (
    ("python3", "scripts/storage_migration_compatibility_evidence.py"),
    ("python3", "scripts/storage_new_family_lifecycle_evidence.py"),
    ("python3", "scripts/storage_crash_boundary_evidence.py"),
    ("python3", "scripts/storage_crash_exact_state_evidence.py"),
)
MARKERS: Final = (
    "Sub-task 11.2.3.1 storage migration compatibility validated",
    "Sub-task 11.2.3.3 storage new-family lifecycle validated",
    "Sub-task 11.2.4.1 storage crash boundaries validated",
    "Sub-task 11.2.4.2 storage crash exact states validated",
)
RETAINED_PATHS: Final = (
    "artifacts/sprints/sprint-11/story-11.2/storage-migration-compatibility-report.json",
    "artifacts/sprints/sprint-11/story-11.2/storage-migration-compatibility-results.log",
    "artifacts/sprints/sprint-11/story-11.2/storage-downgrade-refusal-report.json",
    "artifacts/sprints/sprint-11/story-11.2/storage-new-family-lifecycle-report.json",
    "artifacts/sprints/sprint-11/story-11.2/storage-new-family-lifecycle-results.log",
    "artifacts/sprints/sprint-11/story-11.2/storage-crash-boundary-report.json",
    "artifacts/sprints/sprint-11/story-11.2/storage-crash-boundary-results.log",
    "artifacts/sprints/sprint-11/story-11.2/storage-crash-exact-state-report.json",
    "artifacts/sprints/sprint-11/story-11.2/storage-crash-exact-state-results.log",
    "artifacts/sprints/sprint-11/story-11.1/s-011-st01.json",
    "artifacts/sprints/sprint-11/story-11.1/crash-canary-results.json",
    "artifacts/sprints/sprint-11/story-11.1/s-011-rt01.json",
    "artifacts/sprints/sprint-11/story-11.1/store-lifecycle.json",
    "artifacts/sprints/sprint-11/story-11.1/security-evidence-map.json",
)
MAPPINGS: Final = (
    {
        "review_id": "RV-08",
        "story_status": "demonstrated-current-storage-scope",
        "evidence_categories": ["canary-scans", "content-free-export", "encrypted-page-scans"],
        "remaining": "Later active adapters, logging, model context, and OS telemetry.",
    },
    {
        "review_id": "RV-09",
        "story_status": "partial-current-storage-scope",
        "evidence_categories": ["migration-matrix", "keyed-backup-restore", "page-corruption", "erasure"],
        "remaining": "Live provider operations, rotation, other platforms, and independent cryptographic review.",
    },
    {
        "review_id": "RV-10",
        "story_status": "demonstrated-current-storage-scope",
        "evidence_categories": ["retention", "backup-restore", "deletion", "cleanup"],
        "remaining": "Complete product uninstall, separately managed copies, and installed-product residue scans.",
    },
    {
        "review_id": "RV-17",
        "story_status": "demonstrated-operational-store-scope",
        "evidence_categories": ["subprocess-crash-traces", "exact-old-new-state", "no-replay"],
        "remaining": "Physical power loss, torn sectors, later durable families, and full product transition coverage.",
    },
)
TRUTH: Final = {
    "synthetic_data_only": True,
    "private_user_data_used": False,
    "raw_canary_retained": False,
    "migration_matrices_retained": True,
    "transaction_traces_retained": True,
    "canary_scans_retained": True,
    "encrypted_page_scans_retained": True,
    "cleanup_evidence_retained": True,
    "required_review_mappings_retained": True,
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
        "record_type": "agentmage-storage-security-evidence",
        "story_id": "11.2",
        "task_id": "11.2.4.3",
        "generated_on": "2026-08-30",
        "status": "pass-local-retained-security-evidence",
        "commands": [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "review_mappings": list(MAPPINGS),
        "retained_evidence": [artifact(path) for path in RETAINED_PATHS],
        "artifacts": [
            artifact("docs/verification/story-11-2-storage-security-evidence.md"),
            artifact("scripts/storage_security_evidence.py"),
            artifact("tests/test_storage_security_evidence.py"),
            artifact(RAW_PATH.relative_to(ROOT).as_posix()),
        ],
        "product_truth": dict(TRUTH),
    }


def render(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n"


def validate_retained() -> list[str]:
    failures = [f"missing retained evidence: {path}" for path in RETAINED_PATHS if not (ROOT / path).is_file()]
    if [mapping["review_id"] for mapping in MAPPINGS] != ["RV-08", "RV-09", "RV-10", "RV-17"]:
        failures.append("required review mappings are incomplete or reordered")
    return failures


def validate_raw(value: str) -> list[str]:
    failures = [f"raw results missing marker: {marker}" for marker in MARKERS if marker not in value]
    for prohibited in ("validation failed", "Traceback", "raw canary"):
        if prohibited in value:
            failures.append(f"raw results contain prohibited marker: {prohibited}")
    return failures


def validate_report(value: Any) -> list[str]:
    if value != expected_report():
        return ["storage security evidence report is stale, incomplete, reordered, or widened"]
    if value.get("product_truth") != TRUTH:
        return ["storage security evidence product truth was widened"]
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
        failures = validate_retained() + validate_raw(raw)
        if failures:
            for failure in failures:
                print(f"storage security evidence build failed: {failure}", file=sys.stderr)
            return 1
        EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
        RAW_PATH.write_text(raw, encoding="utf-8")
        REPORT_PATH.write_text(render(expected_report()), encoding="utf-8")
    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(f"storage security evidence validation failed: {error}", file=sys.stderr)
        return 1
    failures = validate_retained() + validate_raw(raw) + validate_report(report)
    if failures:
        for failure in failures:
            print(f"storage security evidence validation failed: {failure}", file=sys.stderr)
        return 1
    print("Sub-task 11.2.4.3 retained storage security evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
