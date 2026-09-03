#!/usr/bin/env python3
"""Build the gate-owned Sprint 37 filesystem-control boundary review."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
REPORT: Final = ROOT / "artifacts/sprints/sprint-37/source-boundary-review.json"
SOURCES: Final = (
    "kernel/engine/src/filesystem_control.rs",
    "platforms/linux/src/filesystem_control.rs",
    "docs/verification/sprint-37-protected-path-corpus.json",
    "docs/verification/sprint-37-local-results.md",
    "scripts/sprint_37_evidence.py",
)
REQUIREMENTS: Final = [
    "SR-PLT-004", "SR-ACC-002", "SR-ACC-003", "SR-ACC-004", "SR-ACC-005",
    "SR-ACC-006", "SR-OPS-001", "SR-TST-004", "SR-TST-005",
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
    combined = b"\n".join(sources.values())
    checks = {
        "security_requirements_are_mapped": all(item.encode() in combined for item in REQUIREMENTS),
        "operation_matrix_is_retained": all(token in combined for token in (
            b"create", b"patch", b"copy", b"move", b"trash",
        )),
        "preview_and_receipt_digests_are_bound": all(token in combined for token in (
            b"preview_sha256", b"preimage_sha256", b"postimage_sha256",
        )),
        "protected_collision_corpus_is_closed": all(token in sources[SOURCES[2]] for token in (
            b'"schema_version"', b'"cases"', b'"record_type"',
        )),
        "recovery_and_process_stop_traces_are_retained": all(token in combined for token in (
            b"restoration", b"native_process_stop_matrix", b"terminal_uncertain_no_replay",
        )),
        "platform_comparison_is_truthful": all(token in combined for token in (
            b'"ubuntu_native_fixture_evidence": False', b'"macos_native_driver": False',
            b'"windows_native_driver": False',
        )),
        "worker_and_complete_fault_claims_remain_false": all(token in combined for token in (
            b'"isolated_write_worker_proven": False',
            b'"complete_crash_durability_matrix": False',
        )),
        "network_shell_and_delivery_remain_disabled": all(token in combined for token in (
            b'"generic_shell": False', b'"network_access": False',
            b'"external_delivery": False',
        )),
    }
    return {
        "schema_version": 1,
        "record_type": "agentmage-sprint-37-source-boundary-review",
        "source_revision": revision,
        "reviewer_identity": "scripts/sprint_37_boundary_review.py",
        "review_class": "gate-owned-automated-filesystem-control-review",
        "independent_human_review_performed": False,
        "security_requirement_ids": REQUIREMENTS,
        "source_sha256": {path: hashlib.sha256(data).hexdigest() for path, data in sources.items()},
        "checks": checks,
        "status": "PASS_LOCAL_BOUNDARY_REVIEW" if all(checks.values()) else "FAIL",
        "limitations": [
            "This is gate-owned automated review, not a human-review claim.",
            "Non-Fedora platforms and complete race, fault, recovery, and worker proof remain blocked.",
            "Manual fuzzing remains deferred and no release claim is made.",
        ],
    }


def validate(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["review is not an object"]
    failures: list[str] = []
    if value.get("reviewer_identity") != "scripts/sprint_37_boundary_review.py": failures.append("reviewer identity drift")
    if value.get("independent_human_review_performed") is not False: failures.append("human review overclaim")
    if value.get("security_requirement_ids") != REQUIREMENTS: failures.append("security mapping drift")
    checks = value.get("checks")
    if not isinstance(checks, dict) or not checks or any(item is not True for item in checks.values()): failures.append("review check failed or suppressed")
    if value.get("status") != "PASS_LOCAL_BOUNDARY_REVIEW": failures.append("review status is not pass")
    revision = str(value.get("source_revision", ""))
    if len(revision) != 40 or any(c not in "0123456789abcdef" for c in revision): return failures + ["source revision invalid"]
    try:
        if value != expected(revision): failures.append("review is stale, incomplete, reordered, or widened")
    except ValueError as error: failures.append(str(error))
    return failures


def main() -> int:
    parser = argparse.ArgumentParser(); parser.add_argument("--write", action="store_true"); parser.add_argument("--source-revision", default="HEAD"); args = parser.parse_args()
    if args.write:
        revision = subprocess.run(["git", "rev-parse", args.source_revision], cwd=ROOT, check=True, capture_output=True, text=True).stdout.strip()
        REPORT.parent.mkdir(parents=True, exist_ok=True); REPORT.write_text(json.dumps(expected(revision), indent=2, sort_keys=True) + "\n")
    try: value = json.loads(REPORT.read_text())
    except (OSError, json.JSONDecodeError) as error: print(f"cannot read Sprint 37 review: {error}", file=sys.stderr); return 1
    failures = validate(value)
    if failures: print("\n".join(failures), file=sys.stderr); return 1
    print("Sprint 37 gate-owned filesystem-control review passed"); return 0


if __name__ == "__main__": raise SystemExit(main())
