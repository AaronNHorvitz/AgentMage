#!/usr/bin/env python3
"""Build the gate-owned Sprint 46 validation-runner boundary review."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
REPORT: Final = ROOT / "artifacts/sprints/sprint-46/source-boundary-review.json"
REQUIREMENTS: Final = [
    "SR-OPS-003", "SR-SUP-003", "SR-TST-001", "SR-TST-002", "SR-TST-003",
    "SR-TST-004", "SR-TST-005", "SR-TST-006", "SR-TST-010",
]
SOURCES: Final = (
    "kernel/engine/src/command_runner.rs",
    "kernel/engine/src/validation_template.rs",
    "kernel/engine/src/validation_result.rs",
    "schemas/runtime/validation-receipt.schema.json",
    "docs/verification/sprint-46-failure-corpus.json",
    "docs/verification/sprint-46-local-results.md",
    "artifacts/sprints/sprint-46/local-evidence-report.json",
    "scripts/sprint_46_evidence.py",
)


def git_bytes(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, capture_output=True,
        check=False, timeout=30,
    )
    if result.returncode:
        raise ValueError(f"review source unavailable: {path}")
    return result.stdout


def expected(revision: str) -> dict[str, Any]:
    sources = {path: git_bytes(revision, path) for path in SOURCES}
    local = json.loads(sources["artifacts/sprints/sprint-46/local-evidence-report.json"])
    implemented = local["implemented_contracts"]
    verification = local["verification_evidence"]
    summary = local["summary"]
    checks = {
        "security_requirements_are_mapped": local.get("security_requirement_ids") == REQUIREMENTS,
        "trusted_template_and_scope_are_bound": all(
            implemented.get(key) is True for key in (
                "trusted_command_template_registry",
                "separate_template_and_scope_approval_contract",
                "focused_selection_without_argument_rewrite",
                "idempotent_separately_approved_rerun_plan",
            )
        ),
        "terminal_result_semantics_are_bound": all(
            implemented.get(key) is True for key in (
                "exact_terminal_command_receipt_verification",
                "strict_non_conflated_result_parser",
                "independent_artifact_and_affected_file_observation",
                "partial_and_unrun_validation_evidence",
                "deterministic_failure_classification",
                "secret_safe_detailed_validation_receipt",
            )
        ),
        "closed_result_inventory_is_bound": (
            verification.get("validation_kind_count") == 9
            and verification.get("normalized_result_state_count") == 14
            and verification.get("failure_classification_count") == 8
        ),
        "unauthorized_and_sensitive_results_are_absent": (
            verification.get("unauthorized_command_acceptance_count") == 0
            and verification.get("process_stream_value_field_count") == 0
            and verification.get("sprint_added_unsafe_or_ffi_file_count") == 0
        ),
        "local_contract_is_bound": summary.get("local_sprint_46_contract_passed") is True,
        "missing_native_proof_remains_false": all(
            verification.get(key) is False for key in (
                "production_validation_coordinator", "native_worker_campaign",
                "protected_raw_log_integration", "native_cross_platform_acceptance",
                "trusted_package_execution", "independent_review", "manual_fuzzing",
            )
        ),
    }
    return {
        "schema_version": 1,
        "record_type": "agentmage-sprint-46-source-boundary-review",
        "source_revision": revision,
        "reviewer_identity": "scripts/sprint_46_boundary_review.py",
        "review_class": "gate-owned-automated-validation-runner-boundary-review",
        "independent_human_review_performed": False,
        "security_requirement_ids": REQUIREMENTS,
        "source_sha256": {
            path: hashlib.sha256(value).hexdigest() for path, value in sources.items()
        },
        "checks": checks,
        "status": "PASS_LOCAL_BOUNDARY_REVIEW" if all(checks.values()) else "FAIL",
        "limitations": [
            "This is gate-owned automated review, not a human-review claim.",
            "Native worker/process-tree and protected raw-log integration remain blocked.",
            "Trusted launch, native platform proof, and manual fuzzing remain blocked.",
            "No production coordinator, platform support, or release claim is made.",
        ],
    }


def validate(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["review is not an object"]
    failures: list[str] = []
    if value.get("reviewer_identity") != "scripts/sprint_46_boundary_review.py":
        failures.append("reviewer identity drift")
    if value.get("independent_human_review_performed") is not False:
        failures.append("human review overclaim")
    if value.get("security_requirement_ids") != REQUIREMENTS:
        failures.append("security mapping drift")
    checks = value.get("checks")
    if not isinstance(checks, dict) or not checks or any(item is not True for item in checks.values()):
        failures.append("review check failed or suppressed")
    if value.get("status") != "PASS_LOCAL_BOUNDARY_REVIEW":
        failures.append("review status is not pass")
    revision = str(value.get("source_revision", ""))
    if len(revision) != 40 or any(character not in "0123456789abcdef" for character in revision):
        return failures + ["source revision invalid"]
    try:
        if value != expected(revision):
            failures.append("review is stale, incomplete, reordered, or widened")
    except (ValueError, json.JSONDecodeError) as error:
        failures.append(str(error))
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    revision = subprocess.run(
        ["git", "rev-parse", arguments.source_revision], cwd=ROOT, check=True,
        capture_output=True, text=True,
    ).stdout.strip()
    if arguments.write:
        REPORT.parent.mkdir(parents=True, exist_ok=True)
        REPORT.write_text(json.dumps(expected(revision), indent=2, sort_keys=True) + "\n")
    try:
        value = json.loads(REPORT.read_text())
    except (OSError, json.JSONDecodeError) as error:
        print(error, file=sys.stderr)
        return 1
    failures = validate(value)
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    print("Sprint 46 gate-owned boundary review passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
