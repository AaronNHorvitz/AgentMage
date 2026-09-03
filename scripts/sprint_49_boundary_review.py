#!/usr/bin/env python3
"""Build the gate-owned Sprint 49 measured-routing boundary review."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
REPORT: Final = ROOT / "artifacts/sprints/sprint-49/source-boundary-review.json"
REQUIREMENTS: Final = [
    "SR-SUP-006", "SR-SUP-007", "SR-SUP-008", "SR-AI-001", "SR-AI-006",
    "SR-AI-010", "SR-AI-011", "SR-AI-012", "SR-AI-013", "SR-AI-014",
    "SR-TST-006",
]
SOURCES: Final = (
    "kernel/engine/src/model_routing.rs",
    "model-profiles/routing/historical-later-candidates.json",
    "model-profiles/routing/role-benchmark-corpus-v1.json",
    "model-profiles/routing/measured-routing-decision-table-v1.json",
    "docs/verification/sprint-49-routing-corpus.json",
    "docs/verification/sprint-49-local-results.md",
    "artifacts/sprints/sprint-49/local-evidence-report.json",
    "scripts/sprint_49_evidence.py",
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
    local = json.loads(sources["artifacts/sprints/sprint-49/local-evidence-report.json"])
    implemented = local["implemented_contracts"]
    verification = local["verification_evidence"]
    summary = local["summary"]
    checks = {
        "security_requirements_are_mapped": local.get("security_requirement_ids") == REQUIREMENTS,
        "exact_profile_and_role_boundaries_are_bound": all(
            implemented.get(key) is True for key in (
                "historical_gemma_4_26b_preserved", "historical_devstral_small_2_preserved",
                "exact_profile_boundary", "twelve_independent_roles",
                "role_to_profile_allowlists", "four_visible_local_budgets",
            )
        ),
        "routing_authority_boundaries_are_bound": all(
            implemented.get(key) is True for key in (
                "deterministic_measured_router", "manual_selection_preserved",
                "visible_disagreements", "measured_high_risk_second_verifier",
                "hidden_fallback_disabled", "frontier_transfer_disabled",
                "model_confidence_unused",
            )
        ),
        "corpus_and_zero_activation_results_are_bound": (
            verification.get("historical_candidate_count") == 2
            and verification.get("enabled_profile_count") == 0
            and verification.get("independent_role_count") == 12
            and verification.get("visible_budget_count") == 4
            and verification.get("routing_rule_count") == 16
            and verification.get("adversarial_corpus_case_count") == 48
            and verification.get("unauthorized_or_remote_selection_count") == 0
        ),
        "local_contract_is_bound": summary.get("local_sprint_49_contract_passed") is True,
        "missing_product_proof_remains_false": all(
            verification.get(key) is False for key in (
                "live_role_benchmark_campaign", "product_router_integration",
                "native_cross_platform_acceptance", "trusted_package_execution",
                "independent_review", "manual_fuzzing",
            )
        ),
    }
    return {
        "schema_version": 1,
        "record_type": "agentmage-sprint-49-source-boundary-review",
        "source_revision": revision,
        "reviewer_identity": "scripts/sprint_49_boundary_review.py",
        "review_class": "gate-owned-automated-measured-routing-boundary-review",
        "independent_human_review_performed": False,
        "security_requirement_ids": REQUIREMENTS,
        "source_sha256": {
            path: hashlib.sha256(value).hexdigest() for path, value in sources.items()
        },
        "checks": checks,
        "status": "PASS_LOCAL_BOUNDARY_REVIEW" if all(checks.values()) else "FAIL",
        "limitations": [
            "This is gate-owned automated review, not a human-review claim.",
            "No approved later-profile manifest or live role benchmark exists.",
            "Product routing and native audit-view integration remain absent.",
            "No model, adapter, platform support, or release is enabled.",
        ],
    }


def validate(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["review is not an object"]
    failures: list[str] = []
    if value.get("reviewer_identity") != "scripts/sprint_49_boundary_review.py":
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
    print("Sprint 49 gate-owned boundary review passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
