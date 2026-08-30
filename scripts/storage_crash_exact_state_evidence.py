#!/usr/bin/env python3
"""Build and validate Sub-task 11.2.4.2 exact crash-state evidence."""

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
RAW_PATH: Final = EVIDENCE_DIR / "storage-crash-exact-state-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "storage-crash-exact-state-report.json"
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
SOURCE_MARKERS: Final = (
    "fn assert_seeded_new_family_state(",
    'assert!(matches!(target_count, 0 | 1), "old or new state only");',
    '"manifest and provenance publish together"',
    '"manifest and lifecycle head publish together"',
    '"extraction crash cannot partially publish an index"',
    '"attempt identity cannot duplicate"',
    '"receipt crash cannot duplicate its attempt"',
    'assert_eq!(stale_current, 0, "stale source cannot appear current");',
    '!directory.join("effect-driver-launched").exists()',
    '"recovery never launches an effect driver"',
)
TRUTH: Final = {
    "synthetic_data_only": True,
    "forced_stop_case_count": 224,
    "exact_old_or_new_state_complete": True,
    "partial_publication_refused": True,
    "duplicate_attempt_refused": True,
    "stale_current_projection_refused": True,
    "replay_driver_launch_refused": True,
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
        "record_type": "agentmage-storage-crash-exact-state-evidence",
        "story_id": "11.2",
        "task_id": "11.2.4.2",
        "generated_on": "2026-08-30",
        "status": "pass-local-crash-exact-state",
        "commands": [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "artifacts": [
            artifact("kernel/engine/src/operational_store.rs"),
            artifact("docs/verification/story-11-2-storage-crash-exact-state-evidence.md"),
            artifact("scripts/storage_crash_exact_state_evidence.py"),
            artifact("tests/test_storage_crash_exact_state_evidence.py"),
            artifact(RAW_PATH.relative_to(ROOT).as_posix()),
        ],
        "product_truth": dict(TRUTH),
    }


def render(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n"


def validate_source(value: str) -> list[str]:
    return [
        f"source missing exact-state marker: {marker}"
        for marker in SOURCE_MARKERS
        if marker not in value
    ]


def validate_raw(value: str) -> list[str]:
    failures = [f"raw results missing marker: {marker}" for marker in MARKERS if marker not in value]
    for prohibited in ("test result: FAILED", "error: could not compile", "warning:"):
        if prohibited in value:
            failures.append(f"raw results contain prohibited marker: {prohibited}")
    return failures


def validate_report(value: Any) -> list[str]:
    if value != expected_report():
        return ["storage crash exact-state report is stale, incomplete, reordered, or widened"]
    if value.get("product_truth") != TRUTH:
        return ["storage crash exact-state product truth was widened"]
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
                print(f"storage crash exact-state evidence build failed: {failure}", file=sys.stderr)
            return 1
        EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
        RAW_PATH.write_text(raw, encoding="utf-8")
        REPORT_PATH.write_text(render(expected_report()), encoding="utf-8")

    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        source = SOURCE_PATH.read_text(encoding="utf-8")
    except (OSError, json.JSONDecodeError) as error:
        print(f"storage crash exact-state evidence validation failed: {error}", file=sys.stderr)
        return 1
    failures = validate_source(source) + validate_raw(raw) + validate_report(report)
    if failures:
        for failure in failures:
            print(f"storage crash exact-state evidence validation failed: {failure}", file=sys.stderr)
        return 1
    print("Sub-task 11.2.4.2 storage crash exact states validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
