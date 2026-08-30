#!/usr/bin/env python3
"""Build and validate Sub-task 11.2.3.2 downgrade-refusal evidence."""

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
RAW_PATH: Final = EVIDENCE_DIR / "storage-downgrade-refusal-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "storage-downgrade-refusal-report.json"
SOURCE_PATH: Final = ROOT / "kernel/engine/src/operational_store.rs"
CONTRACT_TEST_PATH: Final = ROOT / "kernel/contracts/tests/engineering_runtime_record_types.rs"
COMMANDS: Final = (
    (
        "cargo", "test", "-p", "agentmage-kernel-engine",
        "operational_store::tests::older_client_refuses_current_store_before_writer_claim_and_preserves_records",
        "--locked",
    ),
    (
        "cargo", "test", "-p", "agentmage-kernel-engine",
        "operational_store::tests::future_schema_and_page_corruption_are_refused", "--locked",
    ),
    (
        "cargo", "test", "-p", "agentmage-kernel-contracts",
        "every_runtime_record_family_rejects_unsupported_versions", "--locked",
    ),
    (
        "cargo", "clippy", "-p", "agentmage-kernel-engine", "--all-targets",
        "--all-features", "--locked", "--", "-D", "warnings",
    ),
)
MARKERS: Final = (
    "older_client_refuses_current_store_before_writer_claim_and_preserves_records ... ok",
    "future_schema_and_page_corruption_are_refused ... ok",
    "every_runtime_record_family_rejects_unsupported_versions ... ok",
)
SOURCE_MARKERS: Final = (
    "fn open_connection_for_supported_schema(",
    "if retained_schema_version > supported_schema_version",
    "fn older_client_refuses_current_store_before_writer_claim_and_preserves_records()",
    "fs::read(&path).expect(\"encrypted postimage\")",
    "current client reopens preserved store",
    "preserved newer record",
)
TRUTH: Final = {
    "synthetic_data_only": True,
    "schema_refusal_before_writer_claim_complete": True,
    "encrypted_store_byte_preservation_complete": True,
    "unsupported_record_family_refusal_complete": True,
    "automatic_reverse_migration_allowed": False,
    "partial_interpretation_allowed": False,
    "record_deletion_or_rewrite_allowed": False,
    "new_family_lifecycle_coverage_complete": False,
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
        "record_type": "agentmage-storage-downgrade-refusal-evidence",
        "story_id": "11.2",
        "task_id": "11.2.3.2",
        "generated_on": "2026-08-30",
        "status": "pass-local-downgrade-refusal",
        "canonical_store": "operational-store",
        "refusal_order": [
            "open-keyed-connection",
            "read-user-version",
            "refuse-unsupported-version",
            "claim-exclusive-writer",
            "migrate-supported-store",
            "interpret-records",
        ],
        "commands": [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "artifacts": [
            artifact("kernel/engine/src/operational_store.rs"),
            artifact("kernel/contracts/tests/engineering_runtime_record_types.rs"),
            artifact("docs/verification/story-11-2-storage-downgrade-refusal-evidence.md"),
            artifact("scripts/storage_downgrade_refusal_evidence.py"),
            artifact("tests/test_storage_downgrade_refusal_evidence.py"),
            artifact(RAW_PATH.relative_to(ROOT).as_posix()),
        ],
        "product_truth": dict(TRUTH),
    }


def render(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n"


def validate_source(value: str) -> list[str]:
    failures = [f"source missing downgrade-refusal marker: {marker}" for marker in SOURCE_MARKERS if marker not in value]
    start = value.find("fn open_keyed(")
    end = value.find("fn open_current_keyed(", start)
    open_keyed = value[start:end]
    compatibility = open_keyed.find("open_connection_for_supported_schema")
    writer_claim = open_keyed.find("claim_exclusive_writer")
    migration = open_keyed.find("migrate(&connection)")
    record_load = open_keyed.find("load_authority()")
    if min(start, end, compatibility, writer_claim, migration, record_load) < 0:
        failures.append("canonical store-open sequence is incomplete")
    elif not compatibility < writer_claim < migration < record_load:
        failures.append("unsupported schema is not refused before writer claim, migration, and record load")
    return failures


def validate_raw(value: str) -> list[str]:
    failures = [f"raw results missing marker: {marker}" for marker in MARKERS if marker not in value]
    for prohibited in ("test result: FAILED", "error: could not compile", "warning:"):
        if prohibited in value:
            failures.append(f"raw results contain prohibited marker: {prohibited}")
    return failures


def validate_report(value: Any) -> list[str]:
    if value != expected_report():
        return ["storage downgrade-refusal report is stale, incomplete, reordered, or widened"]
    if value.get("product_truth") != TRUTH:
        return ["storage downgrade-refusal product truth was widened"]
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
        failures = validate_source(SOURCE_PATH.read_text(encoding="utf-8")) + validate_raw(raw)
        if failures:
            for failure in failures:
                print(f"storage downgrade-refusal evidence build failed: {failure}", file=sys.stderr)
            return 1
        EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
        RAW_PATH.write_text(raw, encoding="utf-8")
        REPORT_PATH.write_text(render(expected_report()), encoding="utf-8")

    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        source = SOURCE_PATH.read_text(encoding="utf-8")
    except (OSError, json.JSONDecodeError) as error:
        print(f"storage downgrade-refusal evidence validation failed: {error}", file=sys.stderr)
        return 1
    failures = validate_source(source) + validate_raw(raw) + validate_report(report)
    if failures:
        for failure in failures:
            print(f"storage downgrade-refusal evidence validation failed: {failure}", file=sys.stderr)
        return 1
    print("Sub-task 11.2.3.2 storage downgrade refusal validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
