#!/usr/bin/env python3
"""Build and validate Story 1.3 AC3 pre-dispatch rejection evidence."""

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
RAW_PATH: Final = EVIDENCE_DIR / "story-ac3-predispatch-rejection-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "story-ac3-predispatch-rejection-report.json"
COMMANDS: Final = (
    (
        "cargo", "test", "-p", "agentmage-kernel-engine", "--test",
        "engineering_runtime_record_corpus",
        "every_declared_schema_failure_fails_before_canonical_publication", "--", "--exact",
    ),
    (
        "cargo", "test", "-p", "agentmage-kernel-engine", "--test",
        "engineering_runtime_record_corpus",
        "cyclic_and_stale_records_fail_at_their_exact_trusted_boundaries", "--", "--exact",
    ),
    (
        "cargo", "test", "-p", "agentmage-kernel-contracts", "--lib",
        "serialization::tests::malformed_missing_extra_duplicate_and_trailing_inputs_fail_closed",
        "--", "--exact",
    ),
    (
        "cargo", "test", "-p", "agentmage-kernel-contracts", "--lib",
        "serialization::tests::oversized_input_and_unsupported_versions_have_exact_error_paths",
        "--", "--exact",
    ),
    (
        "cargo", "test", "-p", "agentmage-kernel-contracts", "--lib",
        "engineering_records::execution_envelope_tests::unknown_and_missing_envelope_fields_are_rejected",
        "--", "--exact",
    ),
    ("python3", "scripts/engineering_runtime_record_evidence.py"),
)
MARKERS: Final = (
    "test every_declared_schema_failure_fails_before_canonical_publication ... ok",
    "test cyclic_and_stale_records_fail_at_their_exact_trusted_boundaries ... ok",
    "test serialization::tests::malformed_missing_extra_duplicate_and_trailing_inputs_fail_closed ... ok",
    "test serialization::tests::oversized_input_and_unsupported_versions_have_exact_error_paths ... ok",
    "test engineering_records::execution_envelope_tests::unknown_and_missing_envelope_fields_are_rejected ... ok",
    "Task 1.3.3.1 canonical record evidence validated",
)
RETAINED_PATHS: Final = (
    "kernel/contracts/src/serialization.rs",
    "kernel/contracts/src/engineering_records.rs",
    "kernel/engine/src/engineering_records.rs",
    "kernel/engine/tests/engineering_runtime_record_corpus.rs",
    "fixtures/engineering-runtime/v2/manifest.json",
    "artifacts/sprints/sprint-1/story-1.3/canonical-record-evidence-index.json",
)
TRUTH: Final = {
    "current_record_admission_scope_complete": True,
    "fixture_case_count": 56,
    "admitted_record_count": 9,
    "schema_rejection_count": 45,
    "trusted_boundary_rejection_count": 2,
    "malformed_records_reach_publication": False,
    "stale_records_reach_dispatch": False,
    "unsupported_versions_reach_dispatch": False,
    "cyclic_workflows_reach_dispatch": False,
    "model_dispatch_after_rejection": False,
    "tool_dispatch_after_rejection": False,
    "effect_dispatch_after_rejection": False,
    "diagnostics_are_stable_codes": True,
    "diagnostics_include_candidate_content": False,
    "terminal_diagnostics_admit_free_prose": False,
    "synthetic_data_only": True,
    "model_inference_executed": False,
    "tool_worker_executed": False,
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
        "criterion_id": "1.3.AC3",
        "generated_on": "2026-08-30",
        "status": "pass-local-current-record-admission-scope",
        "commands": [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "retained_evidence": [artifact(path) for path in RETAINED_PATHS],
        "artifacts": [
            artifact("docs/verification/story-1-3-ac3-predispatch-rejection.md"),
            artifact("scripts/story_1_3_ac3_evidence.py"),
            artifact("tests/test_story_1_3_ac3_evidence.py"),
            artifact(RAW_PATH.relative_to(ROOT).as_posix()),
        ],
        "acceptance_truth": dict(TRUTH),
        "limitations": [
            "the criterion closes the current local Rust and JSON record-admission scope",
            "rejection occurs in pure decoding and validation before canonical publication or dispatch",
            "installed-client, native-platform, real-model, and independent-review evidence remain later gates",
            "Story, Sprint, packaging, and release completion are not claimed",
        ],
    }


def validate_upstream() -> list[str]:
    failures: list[str] = []
    manifest = json.loads(
        (ROOT / "fixtures/engineering-runtime/v2/manifest.json").read_text(encoding="utf-8")
    )
    cases = manifest.get("cases", [])
    boundary_counts = {
        boundary: sum(case.get("expected_boundary") == boundary for case in cases)
        for boundary in (
            "admit", "reject-schema", "reject-semantic", "admit-individual-reject-record-set"
        )
    }
    if len(cases) != 56 or boundary_counts != {
        "admit": 9,
        "reject-schema": 45,
        "reject-semantic": 1,
        "admit-individual-reject-record-set": 1,
    }:
        failures.append("canonical rejection corpus boundary counts are incomplete")
    expected_codes = {
        case.get("expected_code")
        for case in cases
        if case.get("expected_boundary") in {"reject-semantic", "admit-individual-reject-record-set"}
    }
    if expected_codes != {
        "engineering.workflow.dependency_cycle", "engineering.record.binding_mismatch"
    }:
        failures.append("trusted-boundary diagnostic codes are incomplete")
    index = json.loads(
        (EVIDENCE_DIR / "canonical-record-evidence-index.json").read_text(encoding="utf-8")
    )
    truth = index.get("product_truth", {})
    if any(truth.get(key) is not True for key in (
        "schema_gate_passed", "serialization_gate_passed", "mutation_gate_passed"
    )):
        failures.append("upstream schema, serialization, or mutation evidence is incomplete")
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
        "Story 1.3 AC3 report is stale, incomplete, reordered, or widened"
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
        print(f"Story 1.3 AC3 evidence failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Story 1.3 AC3 evidence failed: {failure}", file=sys.stderr)
        return 1
    print("Story acceptance criterion 1.3.AC3 pre-dispatch rejection validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
