#!/usr/bin/env python3
"""Build the gate-owned Sprint 47 local-commit boundary review."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
REPORT: Final = ROOT / "artifacts/sprints/sprint-47/source-boundary-review.json"
REQUIREMENTS: Final = [
    "SR-GOV-005", "SR-GOV-010", "SR-ACC-002", "SR-ACC-007",
    "SR-SUP-002", "SR-SUP-005", "SR-TST-010", "SR-TST-011",
    "SR-GIT-001", "SR-GIT-002", "SR-GIT-003", "SR-GIT-004", "SR-GIT-007",
]
SOURCES: Final = (
    "kernel/engine/src/review_packet.rs",
    "kernel/engine/src/local_commit.rs",
    "kernel/engine/src/repository_safety.rs",
    "platforms/linux/src/local_commit.rs",
    "platforms/linux/src/repository_safety.rs",
    "docs/verification/sprint-47-security-corpus.json",
    "docs/verification/sprint-47-local-results.md",
    "artifacts/sprints/sprint-47/local-evidence-report.json",
    "scripts/sprint_47_evidence.py",
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
    local = json.loads(sources["artifacts/sprints/sprint-47/local-evidence-report.json"])
    implemented = local["implemented_contracts"]
    verification = local["verification_evidence"]
    summary = local["summary"]
    checks = {
        "security_requirements_are_mapped": local.get("security_requirement_ids") == REQUIREMENTS,
        "review_packet_and_commit_plan_are_bound": all(
            implemented.get(key) is True for key in (
                "complete_review_packet_contract", "nine_review_modes",
                "duplicate_and_low_confidence_evidence", "six_logical_commit_purposes",
                "unrelated_change_exclusion",
            )
        ),
        "approval_signer_and_commit_boundary_are_bound": all(
            implemented.get(key) is True for key in (
                "external_pinned_signer_report", "agent_owned_temporary_index_contract",
                "exact_manual_commit_approval", "one_shot_git_commit_mediation",
                "exact_commit_and_signature_verification",
                "compare_and_swap_owned_task_branch",
                "automatic_publication_and_destructive_git_denied",
            )
        ),
        "native_linux_commit_portion_of_rv49_is_bound": (
            implemented.get("native_linux_candidate_fixture") is True
            and implemented.get("native_linux_openpgp_signed_commit_fixture") is True
            and verification.get("native_candidate_tree_fixture_passed") is True
            and verification.get("native_signed_commit_fixture_passed") is True
        ),
        "closed_inventory_and_adversarial_results_are_bound": (
            verification.get("review_mode_count") == 9
            and verification.get("commit_purpose_count") == 6
            and verification.get("runtime_record_count") == 8
            and verification.get("adversarial_corpus_case_count") == 28
            and verification.get("unauthorized_effect_acceptance_count") == 0
        ),
        "local_contract_is_bound": summary.get("local_sprint_47_contract_passed") is True,
        "missing_product_proof_remains_false": all(
            verification.get(key) is False for key in (
                "production_review_commit_coordinator", "production_approved_signer",
                "protected_manual_approval_channel", "native_signer_process_tree_campaign",
                "native_cross_platform_acceptance", "trusted_package_execution",
                "independent_review", "manual_fuzzing",
            )
        ),
    }
    return {
        "schema_version": 1,
        "record_type": "agentmage-sprint-47-source-boundary-review",
        "source_revision": revision,
        "reviewer_identity": "scripts/sprint_47_boundary_review.py",
        "review_class": "gate-owned-automated-local-commit-boundary-review",
        "independent_human_review_performed": False,
        "security_requirement_ids": REQUIREMENTS,
        "source_sha256": {
            path: hashlib.sha256(value).hexdigest() for path, value in sources.items()
        },
        "checks": checks,
        "status": "PASS_LOCAL_BOUNDARY_REVIEW" if all(checks.values()) else "FAIL",
        "limitations": [
            "This is gate-owned automated review, not a human-review claim.",
            "The signer fixture is disposable and is not a production signing identity.",
            "No protected approval channel or product commit coordinator is active.",
            "Native platform parity, trusted launch, and manual fuzzing remain blocked.",
        ],
    }


def validate(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["review is not an object"]
    failures: list[str] = []
    if value.get("reviewer_identity") != "scripts/sprint_47_boundary_review.py":
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
    print("Sprint 47 gate-owned boundary review passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
