#!/usr/bin/env python3
"""Build and validate the Story 11.2 AC1 crash-acceptance record."""

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
RAW_PATH: Final = EVIDENCE_DIR / "story-ac1-crash-acceptance-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "story-ac1-crash-acceptance-report.json"
SOURCE_PATH: Final = ROOT / "kernel/engine/src/operational_store.rs"
COMMANDS: Final = (
    ("python3", "scripts/storage_crash_boundary_evidence.py"),
    ("python3", "scripts/storage_crash_exact_state_evidence.py"),
)
MARKERS: Final = (
    "Sub-task 11.2.4.1 storage crash boundaries validated",
    "Sub-task 11.2.4.2 storage crash exact states validated",
)
BOUNDARIES: Final = (
    "transaction",
    "manifest",
    "extraction",
    "index",
    "attempt",
    "receipt",
    "verification",
    "recovery",
    "checkpoint",
    "session-checkpoint",
    "migration",
    "key-retrieval",
    "backup",
    "restore",
    "expiry",
    "deletion",
)
SOURCE_MARKERS: Final = (
    "const ALL: [Self; 16]",
    "const SEEDED_CRASH_RUNS: u64 = 224;",
    'assert!(matches!(target_count, 0 | 1), "old or new state only");',
    '"manifest and provenance publish together"',
    '"manifest and lifecycle head publish together"',
    '"extraction crash cannot partially publish an index"',
    '"attempt identity cannot duplicate"',
    '"stale source cannot appear current"',
    "assert!(insert_seeded_new_family(&mut store, boundary).is_err());",
    '"recovery never launches an effect driver"',
    "assert_eq!(coverage.len(), 32);",
    "assert_eq!(coverage.get(&(boundary, position)), Some(&7));",
)
RETAINED_PATHS: Final = (
    "kernel/engine/src/operational_store.rs",
    "artifacts/sprints/sprint-11/story-11.2/storage-crash-boundary-report.json",
    "artifacts/sprints/sprint-11/story-11.2/storage-crash-boundary-results.log",
    "artifacts/sprints/sprint-11/story-11.2/storage-crash-exact-state-report.json",
    "artifacts/sprints/sprint-11/story-11.2/storage-crash-exact-state-results.log",
    "artifacts/sprints/sprint-11/story-11.2/storage-security-evidence-report.json",
)
TRUTH: Final = {
    "current_operational_store_scope_complete": True,
    "forced_stop_case_count": 224,
    "boundary_count": 16,
    "position_count": 2,
    "boundary_position_cell_count": 32,
    "seeds_per_boundary_position": 7,
    "exact_pre_or_post_state": True,
    "orphan_state_refused": True,
    "duplicate_state_refused": True,
    "authority_bearing_partial_state_refused": True,
    "stale_current_state_refused": True,
    "replay_driver_launch_refused": True,
    "synthetic_data_only": True,
    "physical_power_loss_complete": False,
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
        "criterion_id": "11.2.AC1",
        "generated_on": "2026-08-30",
        "status": "pass-local-current-operational-store-scope",
        "commands": [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "covered_boundaries": list(BOUNDARIES),
        "retained_evidence": [artifact(path) for path in RETAINED_PATHS],
        "artifacts": [
            artifact("docs/verification/story-11-2-ac1-crash-acceptance.md"),
            artifact("scripts/story_11_2_ac1_evidence.py"),
            artifact("tests/test_story_11_2_ac1_evidence.py"),
            artifact(RAW_PATH.relative_to(ROOT).as_posix()),
        ],
        "acceptance_truth": dict(TRUTH),
        "limitations": [
            "physical power loss and torn sectors are not represented",
            "native cross-platform and installed-package execution remain separate gates",
            "independent review, Story, Sprint, packaging, and release completion are not claimed",
        ],
    }


def render(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n"


def validate_sources() -> list[str]:
    failures = [f"missing retained evidence: {path}" for path in RETAINED_PATHS if not (ROOT / path).is_file()]
    try:
        source = SOURCE_PATH.read_text(encoding="utf-8")
    except OSError as error:
        return failures + [f"cannot read operational-store source: {error}"]
    failures.extend(f"operational-store source missing marker: {marker}" for marker in SOURCE_MARKERS if marker not in source)
    for name in BOUNDARIES:
        if f'"{name}"' not in source:
            failures.append(f"operational-store source missing boundary: {name}")
    return failures


def validate_upstream_reports() -> list[str]:
    failures: list[str] = []
    expectations = (
        (
            "storage-crash-boundary-report.json",
            {
                "forced_stop_case_count": 224,
                "boundary_position_cell_count": 32,
                "seeds_per_boundary_position": 7,
                "before_after_positions_complete": True,
                "named_new_commit_boundaries_complete": True,
                "duplicate_target_publication_refused": True,
            },
        ),
        (
            "storage-crash-exact-state-report.json",
            {
                "forced_stop_case_count": 224,
                "exact_old_or_new_state_complete": True,
                "partial_publication_refused": True,
                "duplicate_attempt_refused": True,
                "stale_current_projection_refused": True,
                "replay_driver_launch_refused": True,
            },
        ),
    )
    for name, required in expectations:
        try:
            value = json.loads((EVIDENCE_DIR / name).read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            failures.append(f"cannot read upstream report {name}: {error}")
            continue
        truth = value.get("product_truth", {})
        for field, expected in required.items():
            if truth.get(field) != expected:
                failures.append(f"upstream report {name} has invalid {field}")
    return failures


def validate_raw(value: str) -> list[str]:
    failures = [f"raw results missing marker: {marker}" for marker in MARKERS if marker not in value]
    for prohibited in ("validation failed", "Traceback", "FAILED"):
        if prohibited in value:
            failures.append(f"raw results contain prohibited marker: {prohibited}")
    return failures


def validate_report(value: Any) -> list[str]:
    if value != expected_report():
        return ["Story 11.2 AC1 report is stale, incomplete, reordered, or widened"]
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
        failures = validate_sources() + validate_upstream_reports() + validate_raw(raw)
        if failures:
            for failure in failures:
                print(f"Story 11.2 AC1 evidence build failed: {failure}", file=sys.stderr)
            return 1
        EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
        RAW_PATH.write_text(raw, encoding="utf-8")
        REPORT_PATH.write_text(render(expected_report()), encoding="utf-8")
    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(f"Story 11.2 AC1 evidence validation failed: {error}", file=sys.stderr)
        return 1
    failures = validate_sources() + validate_upstream_reports() + validate_raw(raw) + validate_report(report)
    if failures:
        for failure in failures:
            print(f"Story 11.2 AC1 evidence validation failed: {failure}", file=sys.stderr)
        return 1
    print("Story acceptance criterion 11.2.AC1 crash recovery validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
