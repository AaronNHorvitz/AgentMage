#!/usr/bin/env python3
"""Build the gate-owned Sprint 38 Markdown/knowledge boundary review."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
REPORT: Final = ROOT / "artifacts/sprints/sprint-38/source-boundary-review.json"
SOURCES: Final = (
    "capabilities/knowledge/src/markdown_write.rs",
    "capabilities/knowledge/src/knowledge_write.rs",
    "shells/host/src/knowledge_write.rs",
    "docs/verification/sprint-38-markdown-knowledge-corpus.json",
    "docs/verification/sprint-38-local-results.md",
    "artifacts/sprints/sprint-38/local-evidence-report.json",
    "scripts/sprint_38_evidence.py",
)
REQUIREMENTS: Final = [
    "SR-ACC-004", "SR-ACC-005", "SR-ACC-006", "SR-ACC-007", "SR-ACC-008",
    "SR-DAT-001", "SR-DAT-002", "SR-DAT-003", "SR-CIV-003", "SR-CIV-004",
    "SR-TST-004", "SR-TST-005",
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
    local = json.loads(sources[SOURCES[5]])
    implemented = local.get("implemented_contracts", {})
    verification = local.get("verification_evidence", {})
    checks = {
        "security_requirements_are_mapped": local.get("security_requirement_ids") == REQUIREMENTS,
        "parser_writer_round_trips_are_bound": all(implemented.get(key) is True for key in (
            "byte_preserving_markdown_parser", "closed_structure_preserving_edits",
            "raw_notes_protected", "plain_folder_obsidian_domain_parity",
        )),
        "scoped_diffs_and_collision_controls_are_bound": all(implemented.get(key) is True for key in (
            "exact_create_and_update_previews", "stable_identity_and_source_hash_binding",
            "case_and_unicode_namespace_collision_denial", "bulk_reorganization_unrepresentable",
        )),
        "canonical_index_hashes_are_bound": all(implemented.get(key) is True for key in (
            "canonical_first_index_publication", "namespace_compare_and_swap",
            "native_end_to_end_knowledge_transaction",
        )),
        "recovery_evidence_is_retained": verification.get("native_source_index_process_stop_matrix") is True,
        "host_authority_remains_closed": all(token in combined for token in (
            b'"generic_shell": false', b'"network_access": false', b'"external_delivery": false',
        )),
        "missing_proof_remains_false": all(verification.get(key) is False for key in (
            "upstream_sprint_37_gate", "complete_native_crash_and_race_matrix",
            "trusted_package_launcher_test_environment", "non_fedora_native_evidence",
            "independent_review", "manual_fuzzing",
        )),
    }
    return {
        "schema_version": 1,
        "record_type": "agentmage-sprint-38-source-boundary-review",
        "source_revision": revision,
        "reviewer_identity": "scripts/sprint_38_boundary_review.py",
        "review_class": "gate-owned-automated-markdown-knowledge-review",
        "independent_human_review_performed": False,
        "security_requirement_ids": REQUIREMENTS,
        "source_sha256": {path: hashlib.sha256(data).hexdigest() for path, data in sources.items()},
        "checks": checks,
        "status": "PASS_LOCAL_BOUNDARY_REVIEW" if all(checks.values()) else "FAIL",
        "limitations": [
            "This is gate-owned automated review, not a human-review claim.",
            "Upstream, trusted-launcher, non-Fedora, and complete crash/race proof remain blocked.",
            "Manual fuzzing remains deferred and no release claim is made.",
        ],
    }


def validate(value: Any) -> list[str]:
    if not isinstance(value, dict): return ["review is not an object"]
    failures: list[str] = []
    if value.get("reviewer_identity") != "scripts/sprint_38_boundary_review.py": failures.append("reviewer identity drift")
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
    except (OSError, json.JSONDecodeError) as error: print(f"cannot read Sprint 38 review: {error}", file=sys.stderr); return 1
    failures = validate(value)
    if failures: print("\n".join(failures), file=sys.stderr); return 1
    print("Sprint 38 gate-owned Markdown/knowledge review passed"); return 0


if __name__ == "__main__": raise SystemExit(main())
