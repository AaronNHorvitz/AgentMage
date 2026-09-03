#!/usr/bin/env python3
"""Build gate-owned Sprint 43 comprehension and Sprint 44 planning reviews."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
REQUIREMENTS: Final = {
    43: ["SR-ACC-008", "SR-AI-003", "SR-AI-007", "SR-AI-010", "SR-AI-011", "SR-DAT-003", "SR-TST-004", "SR-TST-006"],
    44: ["SR-GOV-005", "SR-GOV-010", "SR-ACC-007", "SR-ACC-008", "SR-AI-003", "SR-AI-007", "SR-AI-011", "SR-TST-001"],
}
SOURCES: Final = {
    43: (
        "capabilities/repository-map/src/deep_analysis.rs",
        "capabilities/repository-map/src/deep_views.rs",
        "docs/verification/sprint-43-hostile-repository-corpus.json",
        "docs/verification/sprint-43-local-results.md",
        "artifacts/sprints/sprint-43/local-evidence-report.json",
        "scripts/sprint_43_evidence.py",
    ),
    44: (
        "capabilities/repository-map/src/change_intent.rs",
        "capabilities/repository-map/src/change_plan.rs",
        "docs/verification/sprint-44-planning-corpus.json",
        "docs/verification/sprint-44-local-results.md",
        "artifacts/sprints/sprint-44/local-evidence-report.json",
        "scripts/sprint_44_evidence.py",
    ),
}


def report_path(sprint: int) -> Path:
    return ROOT / f"artifacts/sprints/sprint-{sprint}/source-boundary-review.json"


def git_bytes(revision: str, path: str) -> bytes:
    result = subprocess.run(["git", "show", f"{revision}:{path}"], cwd=ROOT, capture_output=True, check=False, timeout=30)
    if result.returncode: raise ValueError(f"review source unavailable: {path}")
    return result.stdout


def checks_43(local: dict[str, Any]) -> dict[str, bool]:
    impl, verify, summary = local["implemented_contracts"], local["verification_evidence"], local["summary"]
    return {
        "security_requirements_are_mapped": local.get("security_requirement_ids") == REQUIREMENTS[43],
        "cited_maps_and_coverage_are_bound": all(impl.get(k) is True for k in ("deterministic_repository_profiles", "source_resolvable_evidence_states", "typed_repository_trace_set", "quantified_coverage_and_blind_spots", "whole_repository_overclaim_prohibited")),
        "history_slicing_and_exports_are_bound": all(impl.get(k) is True for k in ("branch_aware_history_contract", "separated_cross_repository_portfolio", "bounded_large_repository_slices", "structured_learning_export_suite")),
        "hostile_and_mutation_results_are_bound": verify.get("hostile_repository_case_count") == 12 and verify.get("deep_index_mutation_count") == 10000 and verify.get("unauthorized_mutation_acceptance_count") == 0 and verify.get("secret_canary_export_count") == 0,
        "unsupported_claims_remain_false": summary.get("complete_repository_comprehension_claim") is False and verify.get("whole_repository_claim_permitted") is False,
        "missing_proof_remains_false": all(verify.get(k) is False for k in ("production_repository_projection", "live_language_native_or_server_adapter", "complete_semantic_relationship_coverage", "native_cross_platform_acceptance", "trusted_package_execution", "independent_review", "manual_fuzzing")),
    }


def checks_44(local: dict[str, Any]) -> dict[str, bool]:
    impl, verify, summary = local["implemented_contracts"], local["verification_evidence"], local["summary"]
    return {
        "security_requirements_are_mapped": local.get("security_requirement_ids") == REQUIREMENTS[44],
        "intent_reproduction_and_scope_are_bound": all(impl.get(k) is True for k in ("source_bound_change_intent", "complete_impact_surface_assessment", "truthful_reproduction_outcomes", "competing_hypothesis_record", "regression_test_gate")),
        "minimal_plan_and_review_controls_are_bound": all(impl.get(k) is True for k in ("five_dimension_alternative_record", "deterministic_review_selection", "separately_grantable_validation_plan", "golden_minimal_scope_comparison")),
        "hostile_and_mutation_results_are_bound": verify.get("hostile_repository_instruction_case_count") == 5 and verify.get("change_plan_mutation_count") == 10000 and verify.get("unauthorized_mutation_acceptance_count") == 0 and verify.get("secret_canary_export_count") == 0,
        "local_planning_truth_is_bound": summary.get("local_sprint_44_contract_passed") is True,
        "missing_proof_remains_false": all(verify.get(k) is False for k in ("production_planning_coordinator", "live_reproduction_and_validation_runner", "native_cross_platform_acceptance", "trusted_package_execution", "independent_review", "manual_fuzzing")),
    }


def expected(sprint: int, revision: str) -> dict[str, Any]:
    sources = {path: git_bytes(revision, path) for path in SOURCES[sprint]}
    local = json.loads(sources[SOURCES[sprint][4]])
    checks = checks_43(local) if sprint == 43 else checks_44(local)
    noun = "repository-comprehension" if sprint == 43 else "change-planning"
    return {
        "schema_version": 1,
        "record_type": f"agentmage-sprint-{sprint}-source-boundary-review",
        "source_revision": revision,
        "reviewer_identity": "scripts/sprint_43_44_boundary_review.py",
        "review_class": f"gate-owned-automated-{noun}-review",
        "independent_human_review_performed": False,
        "security_requirement_ids": REQUIREMENTS[sprint],
        "source_sha256": {p: hashlib.sha256(v).hexdigest() for p, v in sources.items()},
        "checks": checks,
        "status": "PASS_LOCAL_BOUNDARY_REVIEW" if all(checks.values()) else "FAIL",
        "limitations": [
            "This is gate-owned automated review, not a human-review claim.",
            "Production coordination/adapters, trusted launch, and native platform proof remain blocked.",
            "Manual fuzzing remains deferred and no release claim is made.",
        ],
    }


def validate(sprint: int, value: Any) -> list[str]:
    if not isinstance(value, dict): return ["review is not an object"]
    failures: list[str] = []
    if value.get("reviewer_identity") != "scripts/sprint_43_44_boundary_review.py": failures.append("reviewer identity drift")
    if value.get("independent_human_review_performed") is not False: failures.append("human review overclaim")
    if value.get("security_requirement_ids") != REQUIREMENTS[sprint]: failures.append("security mapping drift")
    checks = value.get("checks")
    if not isinstance(checks, dict) or not checks or any(v is not True for v in checks.values()): failures.append("review check failed or suppressed")
    if value.get("status") != "PASS_LOCAL_BOUNDARY_REVIEW": failures.append("review status is not pass")
    revision = str(value.get("source_revision", ""))
    if len(revision) != 40 or any(c not in "0123456789abcdef" for c in revision): return failures + ["source revision invalid"]
    try:
        if value != expected(sprint, revision): failures.append("review is stale, incomplete, reordered, or widened")
    except (ValueError, json.JSONDecodeError) as error: failures.append(str(error))
    return failures


def main() -> int:
    parser = argparse.ArgumentParser(); parser.add_argument("--write", action="store_true"); parser.add_argument("--source-revision", default="HEAD"); args = parser.parse_args()
    revision = subprocess.run(["git", "rev-parse", args.source_revision], cwd=ROOT, check=True, capture_output=True, text=True).stdout.strip()
    failures: list[str] = []
    for sprint in (43, 44):
        path = report_path(sprint)
        if args.write: path.parent.mkdir(parents=True, exist_ok=True); path.write_text(json.dumps(expected(sprint, revision), indent=2, sort_keys=True) + "\n")
        try: value = json.loads(path.read_text())
        except (OSError, json.JSONDecodeError) as error: failures.append(f"Sprint {sprint}: {error}"); continue
        failures.extend(f"Sprint {sprint}: {item}" for item in validate(sprint, value))
    if failures: print("\n".join(failures), file=sys.stderr); return 1
    print("Sprint 43/44 gate-owned boundary reviews passed"); return 0


if __name__ == "__main__": raise SystemExit(main())
