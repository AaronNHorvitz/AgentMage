#!/usr/bin/env python3
"""Build and validate Sub-task 11.2.4.1 crash-boundary evidence."""

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
RAW_PATH: Final = EVIDENCE_DIR / "storage-crash-boundary-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "storage-crash-boundary-report.json"
SOURCE_PATH: Final = ROOT / "kernel/engine/src/operational_store.rs"
COMMANDS: Final = (
    (
        "cargo", "test", "-p", "agentmage-kernel-engine",
        "seeded_crash_recovery_campaign_never_repeats_a_completed_transition",
        "--locked", "--", "--nocapture",
    ),
    (
        "cargo", "clippy", "-p", "agentmage-kernel-engine", "--all-targets",
        "--all-features", "--locked", "--", "-D", "warnings",
    ),
)
MARKERS: Final = (
    "seeded_crash_recovery_campaign_never_repeats_a_completed_transition ... ok",
    "test result: ok. 1 passed; 0 failed",
)
BOUNDARY_MARKERS: Final = tuple(
    f'Self::{name} => "{code}"'
    for name, code in (
        ("Manifest", "manifest"),
        ("Extraction", "extraction"),
        ("Index", "index"),
        ("Attempt", "attempt"),
        ("Receipt", "receipt"),
        ("Verification", "verification"),
        ("Recovery", "recovery"),
        ("Checkpoint", "checkpoint"),
        ("Expiry", "expiry"),
        ("Deletion", "deletion"),
    )
)
SOURCE_MARKERS: Final = (
    "const SEEDED_CRASH_RUNS: u64 = 224;",
    "assert_eq!(coverage.len(), 32);",
    "for position in SeededCrashPosition::ALL",
    "assert!(insert_seeded_new_family(&mut store, boundary).is_err());",
    "assert_eq!(final_count, 1);",
    "Some(SEEDED_CRASH_CHILD_EXIT)",
)
TRUTH: Final = {
    "synthetic_data_only": True,
    "forced_stop_case_count": 224,
    "boundary_position_cell_count": 32,
    "seeds_per_boundary_position": 7,
    "named_new_commit_boundaries_complete": True,
    "before_after_positions_complete": True,
    "duplicate_target_publication_refused": True,
    "cross_table_old_or_new_matrix_complete": False,
    "retained_trace_and_rv_mapping_complete": False,
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
        "record_type": "agentmage-storage-crash-boundary-evidence",
        "story_id": "11.2",
        "task_id": "11.2.4.1",
        "generated_on": "2026-08-30",
        "status": "pass-local-crash-boundaries",
        "commands": [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "artifacts": [
            artifact("kernel/engine/src/operational_store.rs"),
            artifact("docs/verification/story-11-2-storage-crash-boundary-evidence.md"),
            artifact("scripts/storage_crash_boundary_evidence.py"),
            artifact("tests/test_storage_crash_boundary_evidence.py"),
            artifact(RAW_PATH.relative_to(ROOT).as_posix()),
        ],
        "product_truth": dict(TRUTH),
    }


def render(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n"


def validate_source(value: str) -> list[str]:
    failures = [
        f"source missing crash marker: {marker}"
        for marker in (*BOUNDARY_MARKERS, *SOURCE_MARKERS)
        if marker not in value
    ]
    if "const ALL: [Self; 16]" not in value:
        failures.append("crash campaign does not declare exactly 16 boundaries")
    return failures


def validate_raw(value: str) -> list[str]:
    failures = [f"raw results missing marker: {marker}" for marker in MARKERS if marker not in value]
    for prohibited in ("test result: FAILED", "error: could not compile", "warning:"):
        if prohibited in value:
            failures.append(f"raw results contain prohibited marker: {prohibited}")
    return failures


def validate_report(value: Any) -> list[str]:
    if value != expected_report():
        return ["storage crash-boundary report is stale, incomplete, reordered, or widened"]
    if value.get("product_truth") != TRUTH:
        return ["storage crash-boundary product truth was widened"]
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
                print(f"storage crash-boundary evidence build failed: {failure}", file=sys.stderr)
            return 1
        EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
        RAW_PATH.write_text(raw, encoding="utf-8")
        REPORT_PATH.write_text(render(expected_report()), encoding="utf-8")

    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        source = SOURCE_PATH.read_text(encoding="utf-8")
    except (OSError, json.JSONDecodeError) as error:
        print(f"storage crash-boundary evidence validation failed: {error}", file=sys.stderr)
        return 1
    failures = validate_source(source) + validate_raw(raw) + validate_report(report)
    if failures:
        for failure in failures:
            print(f"storage crash-boundary evidence validation failed: {failure}", file=sys.stderr)
        return 1
    print("Sub-task 11.2.4.1 storage crash boundaries validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
