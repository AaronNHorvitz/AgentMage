#!/usr/bin/env python3
"""Build the gate-owned Sprint 41 command-runner boundary review."""

from __future__ import annotations

import argparse, hashlib, json, subprocess, sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
REPORT: Final = ROOT / "artifacts/sprints/sprint-41/source-boundary-review.json"
SOURCES: Final = (
    "kernel/engine/src/command_runner.rs", "platforms/linux/src/command_runner.rs",
    "docs/verification/sprint-41-command-injection-corpus.json",
    "docs/verification/sprint-41-local-results.md",
    "artifacts/sprints/sprint-41/local-evidence-report.json", "scripts/sprint_41_evidence.py",
)
REQUIREMENTS: Final = [
    "SR-ACC-001", "SR-ACC-002", "SR-ACC-003", "SR-ACC-004", "SR-ACC-005",
    "SR-ACC-006", "SR-ACC-007", "SR-PLT-003", "SR-AI-005", "SR-AI-009",
    "SR-TST-004", "SR-TST-006",
]


def git_bytes(revision: str, path: str) -> bytes:
    result = subprocess.run(["git", "show", f"{revision}:{path}"], cwd=ROOT, capture_output=True, check=False, timeout=30)
    if result.returncode: raise ValueError(f"review source unavailable: {path}")
    return result.stdout


def expected(revision: str) -> dict[str, Any]:
    sources = {path: git_bytes(revision, path) for path in SOURCES}
    local = json.loads(sources[SOURCES[4]])
    impl, verify, summary = local["implemented_contracts"], local["verification_evidence"], local["summary"]
    checks = {
        "security_requirements_are_mapped": local.get("security_requirement_ids") == REQUIREMENTS,
        "template_grant_and_no_shell_are_bound": all(impl.get(key) is True for key in (
            "exact_command_spec_and_registry", "direct_process_without_shell",
            "exact_executable_argument_working_directory_environment_validation",
            "unrestricted_shell_and_hidden_expansion_absent", "exact_previews_and_terminal_receipts",
        )),
        "argv_environment_and_injection_are_bound": verify.get("planted_host_configuration_campaign") is True and verify.get("inherited_descriptor_confinement") is True,
        "process_cleanup_and_resources_are_bound": all(verify.get(key) is True for key in (
            "fedora_live_timeout_and_cancellation_cleanup", "peak_resource_accounting",
            "fedora_hostile_multiprocess_timeout", "parent_crash_recovery_campaign",
            "parent_crash_unit_ownership_proven",
        )),
        "sandbox_scope_is_bound": all(verify.get(key) is True for key in (
            "owned_worktree_descriptor_binding", "guest_environment_containment",
            "empty_scratch_residue_absent", "second_program_execution_unreachable",
            "guest_root_exposes_only_declared_mounts",
        )),
        "terminal_truth_is_bound": summary.get("local_sprint_41_contract_passed") is True and summary.get("generic_shell_present") is False and summary.get("network_access_enabled") is False,
        "missing_proof_remains_false": all(verify.get(key) is False for key in (
            "production_command_profile_registration", "hostile_descendant_crash_campaign",
            "native_cross_platform_command_acceptance", "trusted_package_launcher_environment",
            "independent_command_boundary_review", "manual_fuzzing",
        )),
    }
    return {
        "schema_version": 1, "record_type": "agentmage-sprint-41-source-boundary-review",
        "source_revision": revision, "reviewer_identity": "scripts/sprint_41_boundary_review.py",
        "review_class": "gate-owned-automated-command-runner-review",
        "independent_human_review_performed": False, "security_requirement_ids": REQUIREMENTS,
        "source_sha256": {p: hashlib.sha256(v).hexdigest() for p, v in sources.items()},
        "checks": checks, "status": "PASS_LOCAL_BOUNDARY_REVIEW" if all(checks.values()) else "FAIL",
        "limitations": [
            "This is gate-owned automated review, not a human-review claim.",
            "The root-owned helper, trusted launcher, production profile, and other native platforms remain blocked.",
            "Manual fuzzing remains deferred and no release claim is made.",
        ],
    }


def validate(value: Any) -> list[str]:
    if not isinstance(value, dict): return ["review is not an object"]
    failures=[]
    if value.get("reviewer_identity") != "scripts/sprint_41_boundary_review.py": failures.append("reviewer identity drift")
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
    except (OSError, json.JSONDecodeError) as error: print(f"cannot read Sprint 41 review: {error}", file=sys.stderr); return 1
    failures=validate(value)
    if failures: print("\n".join(failures), file=sys.stderr); return 1
    print("Sprint 41 gate-owned command-runner review passed"); return 0


if __name__ == "__main__": raise SystemExit(main())
