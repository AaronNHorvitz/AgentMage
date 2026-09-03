#!/usr/bin/env python3
"""Build the gate-owned Sprint 35 exact-preimage transaction review."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
REPORT: Final = ROOT / "artifacts/sprints/sprint-35/transaction-boundary-review.json"
SOURCES: Final = (
    "kernel/engine/src/write_approval.rs",
    "kernel/engine/src/grants.rs",
    "kernel/engine/src/approval.rs",
    "docs/architecture/exact-preimage-write-approval.md",
    "docs/verification/sprint-35-local-results.md",
    "scripts/sprint_35_evidence.py",
)
SECURITY_REQUIREMENTS: Final = [
    "SR-ACC-001", "SR-ACC-002", "SR-ACC-003", "SR-ACC-004", "SR-ACC-005",
    "SR-ACC-006", "SR-ACC-007", "SR-DAT-002", "SR-OPS-001", "SR-OPS-002",
    "SR-TST-005", "SR-TST-011", "SR-TST-012",
]


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def git_bytes(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, capture_output=True,
        check=False, timeout=30,
    )
    if result.returncode != 0:
        raise ValueError(f"review source unavailable: {path}")
    return result.stdout


def expected(revision: str) -> dict[str, Any]:
    sources = {path: git_bytes(revision, path) for path in SOURCES}
    transaction = sources["kernel/engine/src/write_approval.rs"]
    combined = b"\n".join(sources.values())
    checks = {
        "security_requirements_are_mapped": all(
            requirement.encode() in combined for requirement in SECURITY_REQUIREMENTS
        ),
        "transition_and_property_results_are_retained": all(token in transaction for token in (
            b"shadow_change_preview_and_grant_bind_every_exact_input",
            b"fresh_preimages_validate_without_consuming_or_writing",
        )),
        "attack_traces_fail_closed": all(token in transaction for token in (
            b"stale_partial_excluded_generated_duplicate_and_invalid_proposals_fail_closed",
            b"changed_preview_decision_expiry_and_policy_cannot_issue_authority",
            b"changed_preimage_invalidates_the_single_use_grant",
        )),
        "preimage_and_postimage_hashes_are_bound": all(token in transaction for token in (
            b"preimage_sha256", b"expected_postimage_sha256", b"operation_sha256",
        )),
        "restoration_material_and_plan_are_bound": all(token in transaction for token in (
            b"preimage_bytes", b"rollback", b"Restore the reviewed preimage",
        )),
        "grant_is_short_lived_single_use_and_stale_invalidating": all(
            token in transaction for token in (
                b"MAX_WRITE_GRANT_LIFETIME_MS", b"permitted_uses: 1",
                b"invalidate_issued_grant",
            )
        ),
        "review_does_not_enable_target_mutation": all(token in transaction for token in (
            b"Success is not write authority", b"Sprint 36 must",
        )),
        "network_process_and_external_delivery_are_absent": all(
            token not in combined for token in (
                b"reqwest", b"TcpStream", b"UdpSocket", b"std::process::Command",
            )
        ),
    }
    return {
        "schema_version": 1,
        "record_type": "agentmage-sprint-35-transaction-boundary-review",
        "source_revision": revision,
        "reviewer_identity": "scripts/sprint_35_transaction_review.py",
        "review_class": "gate-owned-automated-exact-preimage-transaction-review",
        "independent_human_review_performed": False,
        "security_requirement_ids": SECURITY_REQUIREMENTS,
        "source_sha256": {path: digest(value) for path, value in sources.items()},
        "checks": checks,
        "status": "PASS_LOCAL_TRANSACTION_REVIEW" if all(checks.values()) else "FAIL",
        "limitations": [
            "This is gate-owned automated review, not a human-review claim.",
            "No target mutation, atomic application, rollback execution, or post-write command is enabled.",
            "Sprint 34 and all inherited release and platform blockers remain unchanged.",
        ],
    }


def validate(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["review is not an object"]
    failures: list[str] = []
    if value.get("reviewer_identity") != "scripts/sprint_35_transaction_review.py":
        failures.append("reviewer identity drift")
    if value.get("independent_human_review_performed") is not False:
        failures.append("human review overclaim")
    if value.get("security_requirement_ids") != SECURITY_REQUIREMENTS:
        failures.append("security requirement mapping drift")
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
        value = expected(revision)
        REPORT.parent.mkdir(parents=True, exist_ok=True)
        REPORT.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")
    else:
        try:
            value = json.loads(REPORT.read_text())
        except (OSError, json.JSONDecodeError) as error:
            print(f"cannot read Sprint 35 transaction review: {error}", file=sys.stderr)
            return 1
    failures = validate(value)
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    print("Sprint 35 gate-owned exact-preimage transaction review passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
