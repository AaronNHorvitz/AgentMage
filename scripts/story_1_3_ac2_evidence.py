#!/usr/bin/env python3
"""Build and validate Story 1.3 AC2 cross-client parity evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-1/story-1.3"
RAW_PATH: Final = EVIDENCE_DIR / "story-ac2-cross-client-parity-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "story-ac2-cross-client-parity-report.json"
COMMANDS: Final = (
    (
        "cargo", "test", "-p", "agentmage-kernel-engine", "--test",
        "engineering_runtime_record_corpus", "valid_records_round_trip_with_exact_cross_language_canonical_bytes",
        "--", "--exact",
    ),
    (
        "cargo", "test", "-p", "agentmage-kernel-engine", "--test",
        "engineering_runtime_record_corpus", "borrowed_record_boundary_matches_the_canonical_contract_encoder_for_every_family",
        "--", "--exact",
    ),
    ("python3", "scripts/engineering_runtime_record_evidence.py"),
)
MARKERS: Final = (
    "test valid_records_round_trip_with_exact_cross_language_canonical_bytes ... ok",
    "test borrowed_record_boundary_matches_the_canonical_contract_encoder_for_every_family ... ok",
    "Task 1.3.3.1 canonical record evidence validated",
)
RETAINED_PATHS: Final = (
    "kernel/contracts/src/engineering_records.rs",
    "kernel/engine/src/engineering_records.rs",
    "kernel/engine/tests/engineering_runtime_record_corpus.rs",
    "scripts/engineering_runtime_fixture_corpus.mjs",
    "fixtures/engineering-runtime/v2/manifest.json",
    "artifacts/sprints/sprint-1/story-1.3/canonical-record-evidence-index.json",
)
TRUTH: Final = {
    "current_rust_javascript_caller_parity_scope_complete": True,
    "canonical_record_family_count": 9,
    "independent_contract_caller_count": 2,
    "fixture_case_count": 56,
    "admitted_record_count": 9,
    "canonical_bytes_match_across_callers": True,
    "canonical_sha256_matches_across_callers": True,
    "content_and_identity_bindings_preserved": True,
    "serialized_enum_meaning_preserved": True,
    "borrowed_publication_matches_authoritative_encoder": True,
    "client_owns_runtime_state": False,
    "client_owns_persistence": False,
    "client_owns_lifecycle_transition": False,
    "client_receives_execution_authority": False,
    "synthetic_data_only": True,
    "model_inference_executed": False,
    "runtime_effect_executed": False,
    "network_calls": 0,
    "installed_client_parity_complete": False,
    "native_platform_complete": False,
    "independent_review_complete": False,
    "story_completion_claim": False,
    "sprint_completion_claim": False,
    "release_claim": "none",
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def artifact(path: str) -> dict[str, Any]:
    value = ROOT / path
    return {"path": path, "byte_length": value.stat().st_size, "sha256": sha256(value)}


def expected_report() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "agentmage-story-acceptance-evidence",
        "story_id": "1.3",
        "criterion_id": "1.3.AC2",
        "generated_on": "2026-08-30",
        "status": "pass-local-current-rust-javascript-caller-parity-scope",
        "commands": [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "retained_evidence": [artifact(path) for path in RETAINED_PATHS],
        "artifacts": [
            artifact("docs/verification/story-1-3-ac2-cross-client-parity.md"),
            artifact("scripts/story_1_3_ac2_evidence.py"),
            artifact("tests/test_story_1_3_ac2_evidence.py"),
            artifact(RAW_PATH.relative_to(ROOT).as_posix()),
        ],
        "acceptance_truth": dict(TRUTH),
        "limitations": [
            "the criterion closes the current deterministic Rust and JavaScript contract-caller scope",
            "callers validate or borrow canonical records and receive no state, persistence, transition, or execution authority",
            "installed-client, native-platform, real-model, and independent-review evidence remain later gates",
            "Story, Sprint, packaging, and release completion are not claimed",
        ],
    }


def validate_upstream() -> list[str]:
    failures: list[str] = []
    index = json.loads(
        (EVIDENCE_DIR / "canonical-record-evidence-index.json").read_text(encoding="utf-8")
    )
    counts = index.get("counts", {})
    if counts.get("canonical_schemas") != 9 or counts.get("fixture_cases") != 56:
        failures.append("canonical family or fixture-case closure is incomplete")
    manifest = json.loads(
        (ROOT / "fixtures/engineering-runtime/v2/manifest.json").read_text(encoding="utf-8")
    )
    cases = manifest.get("cases", [])
    admitted = [case for case in cases if case.get("expected_boundary") == "admit"]
    if len(cases) != 56 or len(admitted) != 9:
        failures.append("cross-client manifest does not contain nine admitted records in 56 cases")
    truth = index.get("product_truth", {})
    if truth.get("serialization_gate_passed") is not True or truth.get("schema_gate_passed") is not True:
        failures.append("upstream serialization or schema gate is incomplete")
    if truth.get("native_platform_execution_claimed") is not False:
        failures.append("upstream evidence widened platform truth")
    return failures


def validate_raw(value: str) -> list[str]:
    failures = [f"raw results missing marker: {marker}" for marker in MARKERS if marker not in value]
    for prohibited in ("Traceback", "test result: FAILED", "validation failed"):
        if prohibited in value:
            failures.append(f"raw results contain prohibited marker: {prohibited}")
    return failures


def validate_report(value: Any) -> list[str]:
    return [] if value == expected_report() else [
        "Story 1.3 AC2 report is stale, incomplete, reordered, or widened"
    ]


def capture() -> tuple[str, int]:
    chunks: list[str] = []
    for command in COMMANDS:
        result = subprocess.run(
            command, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, check=False
        )
        chunks.append(f"$ {' '.join(command)}\n{result.stdout.rstrip()}\n")
        if result.returncode != 0:
            return "".join(chunks), result.returncode
    return "".join(chunks), 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            raw, returncode = capture()
            if returncode != 0:
                sys.stderr.write(raw)
                return 1
            failures = validate_upstream() + validate_raw(raw)
            if failures:
                raise ValueError("; ".join(failures))
            EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
            RAW_PATH.write_text(raw, encoding="utf-8")
            REPORT_PATH.write_text(
                json.dumps(expected_report(), indent=2, sort_keys=True) + "\n", encoding="utf-8"
            )
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        failures = validate_upstream() + validate_raw(raw) + validate_report(report)
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as error:
        print(f"Story 1.3 AC2 evidence failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Story 1.3 AC2 evidence failed: {failure}", file=sys.stderr)
        return 1
    print("Story acceptance criterion 1.3.AC2 cross-client identity parity validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
