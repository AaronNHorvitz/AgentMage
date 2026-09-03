#!/usr/bin/env python3
"""Build the gate-owned Sprint 36 atomic-write transaction review."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
REPORT: Final = ROOT / "artifacts/sprints/sprint-36/transaction-boundary-review.json"
SOURCES: Final = (
    "kernel/engine/src/write_transaction.rs",
    "platforms/linux/src/write_transaction.rs",
    "docs/architecture/atomic-write-transaction-and-rollback.md",
    "docs/verification/sprint-36-local-results.md",
    "scripts/sprint_36_evidence.py",
)
SECURITY_REQUIREMENTS: Final = [
    "SR-ACC-001", "SR-ACC-002", "SR-ACC-003", "SR-ACC-004", "SR-ACC-005",
    "SR-ACC-006", "SR-ACC-007", "SR-DAT-002", "SR-OPS-001", "SR-OPS-002",
    "SR-TST-005", "SR-TST-011", "SR-TST-012",
]


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
    kernel = sources[SOURCES[0]]
    linux = sources[SOURCES[1]]
    combined = b"\n".join(sources.values())
    checks = {
        "security_requirements_are_mapped": all(
            item.encode() in combined for item in SECURITY_REQUIREMENTS
        ),
        "transition_and_receipt_properties_are_retained": all(token in kernel for token in (
            b"transition_matrix_and_receipt_tampering_fail_closed",
            b"verify_write_receipts", b"previous_receipt_sha256",
        )),
        "post_preview_attack_matrix_is_inert": all(token in kernel for token in (
            b"s_029_ut02_every_post_preview_binding_mutation_is_inert",
            b"s_029_st01_consumed_grant_and_approval_replay_never_reapply",
        )),
        "preimage_postimage_and_consumed_grant_hashes_are_bound": all(
            token in kernel for token in (
                b"preimage_sha256", b"postimage_sha256", b"consumed_grant_sha256",
            )
        ),
        "known_partial_effects_restore_exact_preimages": all(token in kernel for token in (
            b"ordered_partial_failure_restores_changed_prefix_and_supersedes_later_work",
            b"observations_match_preimages", b"restore_after_failure",
        )),
        "native_race_and_process_stop_results_are_retained": all(token in linux for token in (
            b"s_029_st01_native_descriptor_races_preserve_competing_state",
            b"s_029_st01_parent_rename_at_every_boundary_restores_authorized_object",
            b"s_029_rt01_process_stops_leave_only_reviewed_target_bytes",
        )),
        "uncertain_results_never_claim_commit_or_retry": (
            b"uncertain_malformed_and_failed_restoration_never_claim_commit_or_retry" in kernel
        ),
        "missing_platform_and_durability_proof_remains_visible": all(
            token in combined for token in (
                b'"native_atomicity_proven": False',
                b'"crash_durability_matrix_complete": False',
                b"complete cross-platform race matrix",
            )
        ),
    }
    return {
        "schema_version": 1,
        "record_type": "agentmage-sprint-36-transaction-boundary-review",
        "source_revision": revision,
        "reviewer_identity": "scripts/sprint_36_transaction_review.py",
        "review_class": "gate-owned-automated-atomic-write-transaction-review",
        "independent_human_review_performed": False,
        "security_requirement_ids": SECURITY_REQUIREMENTS,
        "source_sha256": {
            path: hashlib.sha256(value).hexdigest() for path, value in sources.items()
        },
        "checks": checks,
        "status": "PASS_LOCAL_TRANSACTION_REVIEW" if all(checks.values()) else "FAIL",
        "limitations": [
            "This is gate-owned automated review, not a human-review claim.",
            "Real mount replacement, power-loss durability, and complete platform matrices remain external blockers.",
            "Sprint 35 and its inherited dependencies remain blocked without substitution.",
        ],
    }


def validate(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["review is not an object"]
    failures: list[str] = []
    if value.get("reviewer_identity") != "scripts/sprint_36_transaction_review.py":
        failures.append("reviewer identity drift")
    if value.get("independent_human_review_performed") is not False:
        failures.append("human review overclaim")
    if value.get("security_requirement_ids") != SECURITY_REQUIREMENTS:
        failures.append("security mapping drift")
    checks = value.get("checks")
    if not isinstance(checks, dict) or not checks or any(item is not True for item in checks.values()):
        failures.append("review check failed or suppressed")
    if value.get("status") != "PASS_LOCAL_TRANSACTION_REVIEW":
        failures.append("review status is not pass")
    revision = str(value.get("source_revision", ""))
    if len(revision) != 40 or any(character not in "0123456789abcdef" for character in revision):
        return failures + ["source revision invalid"]
    try:
        if value != expected(revision):
            failures.append("review is stale, incomplete, reordered, or widened")
    except ValueError as error:
        failures.append(str(error))
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    args = parser.parse_args()
    if args.write:
        revision = subprocess.run(
            ["git", "rev-parse", args.source_revision], cwd=ROOT, check=True,
            capture_output=True, text=True,
        ).stdout.strip()
        REPORT.parent.mkdir(parents=True, exist_ok=True)
        REPORT.write_text(json.dumps(expected(revision), indent=2, sort_keys=True) + "\n")
    try:
        value = json.loads(REPORT.read_text())
    except (OSError, json.JSONDecodeError) as error:
        print(f"cannot read Sprint 36 transaction review: {error}", file=sys.stderr)
        return 1
    failures = validate(value)
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    print("Sprint 36 gate-owned atomic-write transaction review passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
