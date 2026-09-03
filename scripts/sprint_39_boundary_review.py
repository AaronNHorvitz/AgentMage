#!/usr/bin/env python3
"""Build the gate-owned Sprint 39 privacy/recovery boundary review."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
REPORT: Final = ROOT / "artifacts/sprints/sprint-39/source-boundary-review.json"
SOURCES: Final = (
    "kernel/engine/src/write_recovery.rs",
    "platforms/linux/src/write_root_inventory.rs",
    "docs/verification/sprint-39-recovery-corpus.json",
    "docs/verification/sprint-39-redacted-audit-fixture.json",
    "docs/verification/sprint-39-local-results.md",
    "artifacts/sprints/sprint-39/local-evidence-report.json",
    "scripts/sprint_39_evidence.py",
)
REQUIREMENTS: Final = [
    "SR-DAT-002", "SR-DAT-003", "SR-DAT-004", "SR-DAT-010", "SR-DAT-011",
    "SR-DAT-012", "SR-OPS-001", "SR-OPS-002", "SR-OPS-003", "SR-OPS-004",
    "SR-OPS-005", "SR-OPS-006", "SR-OPS-007", "SR-TST-005",
]


def git_bytes(revision: str, path: str) -> bytes:
    result = subprocess.run(["git", "show", f"{revision}:{path}"], cwd=ROOT, capture_output=True, check=False, timeout=30)
    if result.returncode: raise ValueError(f"review source unavailable: {path}")
    return result.stdout


def expected(revision: str) -> dict[str, Any]:
    sources = {path: git_bytes(revision, path) for path in SOURCES}
    local = json.loads(sources[SOURCES[5]])
    implemented = local.get("implemented_contracts", {})
    verification = local.get("verification_evidence", {})
    checks = {
        "security_requirements_are_mapped": local.get("security_requirement_ids") == REQUIREMENTS,
        "privacy_canary_scans_are_bound": all(implemented.get(key) is True for key in (
            "eleven_boundary_privacy_gate", "removed_content_not_retained", "native_write_producer_privacy_gate",
        )),
        "checkpoint_and_recovery_matrix_is_bound": all(verification.get(key) is True for key in (
            "native_end_to_end_recovery", "native_derived_index_recovery",
            "native_internal_process_stop_matrices", "complete_native_crash_concurrency_matrix",
        )),
        "cleanup_inventory_is_bound": all(implemented.get(key) is True for key in (
            "content_free_staging_diagnostics", "separately_receipted_cleanup", "all_runtime_roots_scanned",
        )),
        "retention_and_no_replay_are_bound": implemented.get("completed_write_replay_allowed") is False and implemented.get("deterministic_recovery_precedence") is True,
        "audit_chain_is_bound_and_redacted": all(implemented.get(key) is True for key in (
            "hash_chained_checkpoint_validation", "redacted_human_audit", "formal_runtime_checkpoint_schema",
        )),
        "missing_proof_remains_false": all(verification.get(key) is False for key in (
            "upstream_sprint_38_gate", "trusted_package_launcher_environment",
            "non_fedora_native_evidence", "independent_review", "manual_fuzzing",
            "physical_enospc_executed",
        )),
    }
    return {
        "schema_version": 1,
        "record_type": "agentmage-sprint-39-source-boundary-review",
        "source_revision": revision,
        "reviewer_identity": "scripts/sprint_39_boundary_review.py",
        "review_class": "gate-owned-automated-write-privacy-recovery-review",
        "independent_human_review_performed": False,
        "security_requirement_ids": REQUIREMENTS,
        "source_sha256": {path: hashlib.sha256(data).hexdigest() for path, data in sources.items()},
        "checks": checks,
        "status": "PASS_LOCAL_BOUNDARY_REVIEW" if all(checks.values()) else "FAIL",
        "limitations": [
            "This is gate-owned automated review, not a human-review claim.",
            "Upstream, trusted-launcher, non-Fedora, and physical-fault proof remain blocked.",
            "Manual fuzzing remains deferred and no release claim is made.",
        ],
    }


def validate(value: Any) -> list[str]:
    if not isinstance(value, dict): return ["review is not an object"]
    failures: list[str] = []
    if value.get("reviewer_identity") != "scripts/sprint_39_boundary_review.py": failures.append("reviewer identity drift")
    if value.get("independent_human_review_performed") is not False: failures.append("human review overclaim")
    if value.get("security_requirement_ids") != REQUIREMENTS: failures.append("security mapping drift")
    checks = value.get("checks")
    if not isinstance(checks, dict) or not checks or any(item is not True for item in checks.values()): failures.append("review check failed or suppressed")
    if value.get("status") != "PASS_LOCAL_BOUNDARY_REVIEW": failures.append("review status is not pass")
    revision = str(value.get("source_revision", ""))
    if len(revision) != 40 or any(c not in "0123456789abcdef" for c in revision): return failures + ["source revision invalid"]
    try:
        if value != expected(revision): failures.append("review is stale, incomplete, reordered, or widened")
    except (ValueError, json.JSONDecodeError) as error: failures.append(str(error))
    return failures


def main() -> int:
    parser = argparse.ArgumentParser(); parser.add_argument("--write", action="store_true"); parser.add_argument("--source-revision", default="HEAD"); args = parser.parse_args()
    if args.write:
        revision = subprocess.run(["git", "rev-parse", args.source_revision], cwd=ROOT, check=True, capture_output=True, text=True).stdout.strip()
        REPORT.parent.mkdir(parents=True, exist_ok=True); REPORT.write_text(json.dumps(expected(revision), indent=2, sort_keys=True) + "\n")
    try: value = json.loads(REPORT.read_text())
    except (OSError, json.JSONDecodeError) as error: print(f"cannot read Sprint 39 review: {error}", file=sys.stderr); return 1
    failures = validate(value)
    if failures: print("\n".join(failures), file=sys.stderr); return 1
    print("Sprint 39 gate-owned privacy/recovery review passed"); return 0


if __name__ == "__main__": raise SystemExit(main())
