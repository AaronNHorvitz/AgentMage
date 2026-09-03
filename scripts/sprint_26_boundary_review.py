#!/usr/bin/env python3
"""Build the gate-owned Sprint 26 knowledge authority boundary review."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
REPORT: Final = ROOT / "artifacts/sprints/sprint-26/source-boundary-review.json"
SOURCES: Final = (
    "capabilities/knowledge/src/authority.rs",
    "capabilities/knowledge/src/index.rs",
    "capabilities/knowledge/src/lifecycle.rs",
    "capabilities/knowledge/src/operations.rs",
    "capabilities/knowledge/src/store.rs",
    "docs/architecture/knowledge-authority-boundary.md",
)


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
    authority = sources["capabilities/knowledge/src/authority.rs"]
    index = sources["capabilities/knowledge/src/index.rs"]
    lifecycle = sources["capabilities/knowledge/src/lifecycle.rs"]
    operations = sources["capabilities/knowledge/src/operations.rs"]
    store = sources["capabilities/knowledge/src/store.rs"]
    architecture = sources["docs/architecture/knowledge-authority-boundary.md"]
    checks = {
        "field_policy_has_one_owner_and_lifecycle": all(token in authority for token in (
            b"KnowledgeDataOwner", b"KnowledgeStorageRule", b"recovery_source",
        )),
        "canonical_markdown_is_user_owned": b"CanonicalMarkdown" in authority
        and b"UserOwnedMarkdown" in authority,
        "index_is_derived_and_disposable": b"derived_only" in index
        and b"DisposableSqlite" in authority,
        "lifecycle_is_preview_only": b"KnowledgeRestorePlan" in lifecycle
        and b"KnowledgeMigrationPlan" in lifecycle,
        "operations_expose_no_apply_authority": all(
            token not in operations for token in (b"apply_write", b"delete_file", b"rename_file")
        ),
        "operational_store_is_not_a_dependency": b"use agentmage_kernel_engine" not in store
        and b"no create, update, delete, move, filesystem, or operational-store method" in store
        and b"Operational sessions" in architecture,
        "unsupported_platform_and_release_claims_absent": b"no filesystem handle" in architecture
        and b"v0.3 grant path" in architecture,
    }
    return {
        "schema_version": 1,
        "record_type": "agentmage-sprint-26-boundary-review",
        "source_revision": revision,
        "reviewer_identity": "scripts/sprint_26_boundary_review.py",
        "review_class": "gate-owned-automated-knowledge-authority-boundary",
        "independent_human_review_performed": False,
        "source_sha256": {path: digest(value) for path, value in sources.items()},
        "checks": checks,
        "status": "PASS_LOCAL_BOUNDARY_REVIEW" if all(checks.values()) else "FAIL",
        "limitations": [
            "This is gate-owned automated review, not a human-review claim.",
            "The privacy and records Decisions remain unaccepted human governance blockers.",
            "Supported-package integration and upstream release closure remain separate blockers.",
        ],
    }


def validate(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["review is not an object"]
    failures: list[str] = []
    if value.get("reviewer_identity") != "scripts/sprint_26_boundary_review.py":
        failures.append("reviewer identity drift")
    if value.get("independent_human_review_performed") is not False:
        failures.append("human review overclaim")
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
            print(f"cannot read Sprint 26 review: {error}", file=sys.stderr)
            return 1
    failures = validate(value)
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    print("Sprint 26 gate-owned knowledge authority review passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
