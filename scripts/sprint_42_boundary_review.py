#!/usr/bin/env python3
"""Build the gate-owned Sprint 42 repository-safety boundary review."""

from __future__ import annotations

import argparse, hashlib, json, subprocess, sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
REPORT: Final = ROOT / "artifacts/sprints/sprint-42/source-boundary-review.json"
SOURCES: Final = (
    "kernel/engine/src/repository_safety.rs", "platforms/linux/src/repository_safety.rs",
    "docs/verification/sprint-42-local-results.md",
    "artifacts/sprints/sprint-42/local-evidence-report.json", "scripts/sprint_42_evidence.py",
)
REQUIREMENTS: Final = [
    "SR-ACC-006", "SR-ACC-007", "SR-ACC-008", "SR-NET-005", "SR-NET-006",
    "SR-NET-007", "SR-OPS-001", "SR-TST-004", "SR-TST-005", "SR-GIT-001",
    "SR-GIT-002", "SR-GIT-003", "SR-GIT-004", "SR-GIT-005", "SR-GIT-006",
]


def git_bytes(revision: str, path: str) -> bytes:
    result=subprocess.run(["git", "show", f"{revision}:{path}"], cwd=ROOT, capture_output=True, check=False, timeout=30)
    if result.returncode: raise ValueError(f"review source unavailable: {path}")
    return result.stdout


def expected(revision: str) -> dict[str, Any]:
    sources={path: git_bytes(revision, path) for path in SOURCES}
    local=json.loads(sources[SOURCES[3]])
    impl, verify, summary=local["implemented_contracts"], local["verification_evidence"], local["summary"]
    checks={
        "security_requirements_are_mapped": local.get("security_requirement_ids") == REQUIREMENTS,
        "owned_worktree_preservation_is_bound": all(impl.get(key) is True for key in (
            "owned_task_worktree_lifecycle", "worktree_lifecycle_state_matrix",
            "content_minimized_ownership_registry", "preservation_manifest_reconciliation",
        )),
        "active_checkout_and_unrelated_state_are_preserved": verify.get("fedora_local_worktree_lifecycle") is True and summary.get("active_checkout_preservation_locally_exercised") is True,
        "hostile_repository_controls_are_bound": verify.get("fixed_hostile_case_count") == 44 and verify.get("protected_manifest_mutation_count") == 10000 and verify.get("unauthorized_manifest_mutation_acceptance_count") == 0,
        "local_recovery_and_collision_results_are_bound": all(verify.get(key) is True for key in (
            "local_interruption_and_recovery_campaign", "fedora_local_agentmage_branch_cas",
            "fedora_live_repository_fixtures_executed",
        )) and impl.get("stale_preimage_and_collision_denial") is True,
        "remote_authority_remains_disabled": summary.get("repository_profile_active") is False and summary.get("network_git_enabled") is False,
        "missing_proof_remains_false": all(verify.get(key) is False for key in (
            "network_clone_or_fetch_execution", "clone_success_reconciliation",
            "complete_descendant_containment", "peak_resource_accounting",
            "native_cross_platform_acceptance", "trusted_package_execution",
            "independent_review", "manual_fuzzing",
        )),
    }
    return {
        "schema_version": 1, "record_type": "agentmage-sprint-42-source-boundary-review",
        "source_revision": revision, "reviewer_identity": "scripts/sprint_42_boundary_review.py",
        "review_class": "gate-owned-automated-repository-safety-review",
        "independent_human_review_performed": False, "security_requirement_ids": REQUIREMENTS,
        "source_sha256": {p: hashlib.sha256(v).hexdigest() for p,v in sources.items()},
        "checks": checks, "status": "PASS_LOCAL_BOUNDARY_REVIEW" if all(checks.values()) else "FAIL",
        "limitations": [
            "This is gate-owned automated local review, not a human-review claim or complete RV-49.",
            "Authenticated network Git, trusted launcher, containment resources, and other platforms remain blocked.",
            "Manual fuzzing remains deferred and no release claim is made.",
        ],
    }


def validate(value: Any) -> list[str]:
    if not isinstance(value, dict): return ["review is not an object"]
    failures=[]
    if value.get("reviewer_identity") != "scripts/sprint_42_boundary_review.py": failures.append("reviewer identity drift")
    if value.get("independent_human_review_performed") is not False: failures.append("human review overclaim")
    if value.get("security_requirement_ids") != REQUIREMENTS: failures.append("security mapping drift")
    if not isinstance(value.get("checks"), dict) or not value["checks"] or any(v is not True for v in value["checks"].values()): failures.append("review check failed or suppressed")
    if value.get("status") != "PASS_LOCAL_BOUNDARY_REVIEW": failures.append("review status is not pass")
    revision=str(value.get("source_revision", ""))
    if len(revision) != 40 or any(c not in "0123456789abcdef" for c in revision): return failures+["source revision invalid"]
    try:
        if value != expected(revision): failures.append("review is stale, incomplete, reordered, or widened")
    except (ValueError, json.JSONDecodeError) as error: failures.append(str(error))
    return failures


def main() -> int:
    parser=argparse.ArgumentParser(); parser.add_argument("--write", action="store_true"); parser.add_argument("--source-revision", default="HEAD"); args=parser.parse_args()
    if args.write:
        revision=subprocess.run(["git", "rev-parse", args.source_revision], cwd=ROOT, check=True, capture_output=True, text=True).stdout.strip()
        REPORT.parent.mkdir(parents=True, exist_ok=True); REPORT.write_text(json.dumps(expected(revision), indent=2, sort_keys=True)+"\n")
    try: value=json.loads(REPORT.read_text())
    except (OSError, json.JSONDecodeError) as error: print(f"cannot read Sprint 42 review: {error}", file=sys.stderr); return 1
    failures=validate(value)
    if failures: print("\n".join(failures), file=sys.stderr); return 1
    print("Sprint 42 gate-owned repository-safety review passed"); return 0


if __name__ == "__main__": raise SystemExit(main())
