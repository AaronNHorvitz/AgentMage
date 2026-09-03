#!/usr/bin/env python3
"""Build the gate-owned Sprint 45 structured-coding boundary review."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
REPORT: Final = ROOT / "artifacts/sprints/sprint-45/source-boundary-review.json"
REQUIREMENTS: Final = [
    "SR-ACC-002", "SR-ACC-003", "SR-ACC-004", "SR-ACC-005",
    "SR-ACC-006", "SR-ACC-007", "SR-ACC-008", "SR-SUP-003",
    "SR-SUP-009", "SR-AI-005", "SR-TST-002", "SR-TST-005", "SR-TST-011",
]
SOURCES: Final = (
    "capabilities/repository-map/src/structured_edit.rs",
    "capabilities/repository-map/src/language_service.rs",
    "capabilities/repository-map/src/package_scaffold.rs",
    "capabilities/repository-map/src/test_generation.rs",
    "shells/host/src/code_change.rs",
    "kernel/engine/src/write_transaction.rs",
    "kernel/engine/src/write_recovery.rs",
    "kernel/engine/tests/write_recovery_matrix.rs",
    "docs/security/sprint-45-ffi-unsafe-inventory.md",
    "docs/verification/sprint-45-language-service-confinement.md",
    "docs/verification/sprint-45-coding-corpus.json",
    "docs/verification/sprint-45-local-results.md",
    "artifacts/sprints/sprint-45/local-evidence-report.json",
    "scripts/sprint_45_evidence.py",
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
    local = json.loads(sources["artifacts/sprints/sprint-45/local-evidence-report.json"])
    implemented = local["implemented_contracts"]
    verification = local["verification_evidence"]
    summary = local["summary"]
    checks = {
        "security_requirements_are_mapped": local.get("security_requirement_ids") == REQUIREMENTS,
        "structured_edits_and_safe_fallback_are_bound": all(
            implemented.get(key) is True for key in (
                "six_artifact_classes", "parser_backed_structured_edits",
                "unique_exact_text_fallback", "unrelated_byte_preservation_evidence",
            )
        ),
        "authority_free_service_boundary_is_bound": (
            implemented.get("confined_language_service_contract") is True
            and verification.get("language_service_capability_count") == 5
            and verification.get("forbidden_service_power_count") == 7
        ),
        "atomic_transaction_and_recovery_are_bound": (
            implemented.get("ordered_atomic_shadow_composition") is True
            and verification.get("unauthorized_mutation_acceptance_count") == 0
        ),
        "approved_scaffold_application_is_bound": (
            implemented.get("approved_package_scaffold_plans") is True
            and implemented.get("controlled_package_scaffold_application") is True
            and verification.get("approved_package_convention_count") == 5
            and verification.get("controlled_package_scaffold_application_count") == 5
        ),
        "ffi_and_unsafe_inventory_is_bound": (
            verification.get("sprint_added_unsafe_or_ffi_file_count") == 0
        ),
        "local_contract_is_bound": summary.get("local_sprint_45_contract_passed") is True,
        "missing_native_proof_remains_false": all(
            verification.get(key) is False for key in (
                "production_language_service_sandbox", "native_cross_platform_acceptance",
                "trusted_package_execution", "independent_review", "manual_fuzzing",
            )
        ),
    }
    return {
        "schema_version": 1,
        "record_type": "agentmage-sprint-45-source-boundary-review",
        "source_revision": revision,
        "reviewer_identity": "scripts/sprint_45_boundary_review.py",
        "review_class": "gate-owned-automated-structured-coding-boundary-review",
        "independent_human_review_performed": False,
        "security_requirement_ids": REQUIREMENTS,
        "source_sha256": {
            path: hashlib.sha256(value).hexdigest() for path, value in sources.items()
        },
        "checks": checks,
        "status": "PASS_LOCAL_BOUNDARY_REVIEW" if all(checks.values()) else "FAIL",
        "limitations": [
            "This is gate-owned automated review, not a human-review claim.",
            "The scaffold application is inert and grants no filesystem or command authority.",
            "Real language-service confinement, trusted launch, and native platform proof remain blocked.",
            "Manual fuzzing remains deferred and no release claim is made.",
        ],
    }


def validate(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["review is not an object"]
    failures: list[str] = []
    if value.get("reviewer_identity") != "scripts/sprint_45_boundary_review.py":
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
    print("Sprint 45 gate-owned boundary review passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
