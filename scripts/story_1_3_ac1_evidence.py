#!/usr/bin/env python3
"""Build and validate Story 1.3 AC1 Rust-owned boundary evidence."""

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
RAW_PATH: Final = EVIDENCE_DIR / "story-ac1-rust-owned-boundary-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "story-ac1-rust-owned-boundary-report.json"
COMMANDS: Final = (
    (
        "cargo", "test", "-p", "agentmage-kernel-contracts", "--test",
        "engineering_runtime_record_types", "--locked",
    ),
    (
        "cargo", "test", "-p", "agentmage-kernel-engine", "--test",
        "engineering_runtime_record_corpus", "every_declared_schema_failure_fails_before_canonical_publication",
        "--", "--exact",
    ),
    ("python3", "scripts/engineering_runtime_record_evidence.py"),
)
MARKERS: Final = (
    "test all_nine_runtime_record_families_match_their_admitted_schema_shape ... ok",
    "test every_runtime_record_family_rejects_unsupported_versions ... ok",
    "test schema_required_nullable_fields_cannot_be_omitted ... ok",
    "test every_declared_schema_failure_fails_before_canonical_publication ... ok",
    "Task 1.3.3.1 canonical record evidence validated",
)
RETAINED_PATHS: Final = (
    "kernel/contracts/src/engineering_records.rs",
    "kernel/engine/src/engineering_records.rs",
    "kernel/contracts/tests/engineering_runtime_record_types.rs",
    "kernel/engine/tests/engineering_runtime_record_corpus.rs",
    "scripts/engineering_runtime_schemas.mjs",
    "scripts/engineering_runtime_fixture_corpus.mjs",
    "artifacts/sprints/sprint-1/story-1.3/canonical-record-evidence-index.json",
)
TRUTH: Final = {
    "current_canonical_record_boundary_scope_complete": True,
    "canonical_record_family_count": 9,
    "canonical_schema_version": 2,
    "generated_record_schema_count": 9,
    "rust_owned_meaning": True,
    "rust_and_json_field_sets_match": True,
    "required_nullable_fields_may_be_omitted": False,
    "unknown_fields_admitted": False,
    "missing_fields_admitted": False,
    "malformed_records_admitted": False,
    "oversized_records_admitted": False,
    "unsupported_versions_admitted": False,
    "invalid_records_reach_canonical_publication": False,
    "synthetic_data_only": True,
    "model_inference_executed": False,
    "runtime_effect_executed": False,
    "network_calls": 0,
    "installed_product_complete": False,
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
        "criterion_id": "1.3.AC1",
        "generated_on": "2026-08-30",
        "status": "pass-local-current-canonical-record-boundary-scope",
        "commands": [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "retained_evidence": [artifact(path) for path in RETAINED_PATHS],
        "artifacts": [
            artifact("docs/verification/story-1-3-ac1-rust-owned-boundary-meaning.md"),
            artifact("scripts/story_1_3_ac1_evidence.py"),
            artifact("tests/test_story_1_3_ac1_evidence.py"),
            artifact(RAW_PATH.relative_to(ROOT).as_posix()),
        ],
        "acceptance_truth": dict(TRUTH),
        "limitations": [
            "the criterion closes the current canonical Rust and JSON record-boundary scope",
            "all fixture inputs are public synthetic data and no model, tool, network, or product effect executes",
            "installed-product, native-platform, real-model, and independent-review evidence remain later gates",
            "Story, Sprint, packaging, and release completion are not claimed",
        ],
    }


def validate_upstream() -> list[str]:
    failures: list[str] = []
    path = EVIDENCE_DIR / "canonical-record-evidence-index.json"
    report = json.loads(path.read_text(encoding="utf-8"))
    counts = report.get("counts", {})
    if counts != {
        "canonical_schemas": 9,
        "fixture_files": 57,
        "fixture_cases": 56,
        "commands": 7,
    }:
        failures.append("canonical schema and fixture closure is incomplete")
    groups = {group.get("id"): group for group in report.get("groups", [])}
    if len(groups.get("schemas", {}).get("artifacts", [])) != 9:
        failures.append("nine canonical generated schemas are not retained")
    if len(groups.get("fixtures", {}).get("artifacts", [])) != 57:
        failures.append("canonical fixture file set is incomplete")
    truth = report.get("product_truth", {})
    if any(truth.get(key) is not True for key in (
        "schema_gate_passed", "serialization_gate_passed", "mutation_gate_passed"
    )):
        failures.append("upstream schema, serialization, or mutation gate is incomplete")
    if any(truth.get(key) is not False for key in (
        "native_platform_execution_claimed", "external_review_claimed", "release_readiness_claimed"
    )):
        failures.append("upstream evidence widened product truth")
    return failures


def validate_raw(value: str) -> list[str]:
    failures = [f"raw results missing marker: {marker}" for marker in MARKERS if marker not in value]
    for prohibited in ("Traceback", "test result: FAILED", "validation failed"):
        if prohibited in value:
            failures.append(f"raw results contain prohibited marker: {prohibited}")
    return failures


def validate_report(value: Any) -> list[str]:
    return [] if value == expected_report() else [
        "Story 1.3 AC1 report is stale, incomplete, reordered, or widened"
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
        print(f"Story 1.3 AC1 evidence failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Story 1.3 AC1 evidence failed: {failure}", file=sys.stderr)
        return 1
    print("Story acceptance criterion 1.3.AC1 Rust-owned boundary meaning validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
