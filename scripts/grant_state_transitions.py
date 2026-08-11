#!/usr/bin/env python3
"""Build and verify the Story 5.1 grant-state transition report."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DOC_PATH = ROOT / "docs/architecture/grant-state-transitions.md"
GRANT_PATH = ROOT / "kernel/contracts/src/grant.rs"
ISSUER_PATH = ROOT / "kernel/engine/src/grants.rs"
REPORT_PATH = ROOT / "artifacts/sprints/sprint-5/story-5.1/grant-state-report.json"
SOURCE_PATHS = (
    "docs/architecture/grant-state-transitions.md",
    "kernel/contracts/src/grant.rs",
    "kernel/engine/src/grants.rs",
    "scripts/grant_state_transitions.py",
    "tests/test_grant_state_transitions.py",
)
GRANT_STATUSES = (
    "Issued",
    "Consumed",
    "Revoked",
    "Expired",
    "Invalidated",
    "Uncertain",
)
TRANSITIONS = (
    ("P1", "session_parent", "Absent", "Issued"),
    ("P2", "session_parent", "Issued", "Issued"),
    ("P3", "session_parent", "Issued", "Consumed"),
    ("O1", "operation", "Absent", "Issued"),
    ("O2", "operation", "Issued", "Consumed"),
    ("O3", "operation", "Issued", "Invalidated"),
    ("O4", "operation", "Issued", "Expired"),
    ("O5", "operation", "Consumed", "Uncertain"),
)
REQUIRED_HEADINGS = (
    "Status and Scope",
    "Status Model",
    "Session Parent Diagram",
    "Operation Diagram",
    "Implemented Transition Table",
    "Non-Transitions",
    "Transition Invariants",
    "Review Checklist",
    "Limitations",
)
ISSUER_MARKERS = (
    "status: GrantStatus::Issued",
    "updated_parent.status = GrantStatus::Consumed",
    "consumed.status = GrantStatus::Consumed",
    "GrantStatus::Expired",
    "GrantStatus::Invalidated",
    "GrantStatus::Uncertain",
    "if updated_parent.use_count == updated_parent.use_limit",
    "if context.now_epoch_ms >= issued.expires_at_epoch_ms",
    "consumed.status != GrantStatus::Consumed",
    "self.transition_status(",
)


class GrantStateError(ValueError):
    """Raised when lifecycle documentation or evidence is inconsistent."""


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-grant-state-", dir=path.parent
    )
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
        temporary.chmod(0o644)
        os.replace(temporary, path)
    except BaseException:
        temporary.unlink(missing_ok=True)
        raise


def rust_enum_variants(source: str, name: str) -> tuple[str, ...]:
    match = re.search(rf"pub enum {re.escape(name)}\s*\{{(.*?)\n\}}", source, re.DOTALL)
    if match is None:
        raise GrantStateError(f"missing Rust enum: {name}")
    return tuple(
        re.findall(
            r"^\s*([A-Z][A-Za-z0-9_]*)\s*(?:\([^\n]*\))?,?\s*$",
            match.group(1),
            re.MULTILINE,
        )
    )


def validate_document(text: str) -> list[str]:
    failures: list[str] = []
    for heading in REQUIRED_HEADINGS:
        if f"## {heading}" not in text:
            failures.append(f"missing heading: {heading}")
    for status in GRANT_STATUSES:
        if f"`{status.lower()}`" not in text:
            failures.append(f"missing grant status: {status}")
    for transition_id, grant_class, source, destination in TRANSITIONS:
        row_markers = (
            f"`{transition_id}`",
            "Session parent" if grant_class == "session_parent" else "Operation",
            f"`{source}`",
            f"`{destination}`",
        )
        if not all(marker in text for marker in row_markers):
            failures.append(f"incomplete transition row: {transition_id}")
    for marker in (
        "stateDiagram-v2",
        "ParentIssued --> ParentConsumed",
        "OperationIssued --> OperationConsumed",
        "OperationIssued --> OperationInvalidated",
        "OperationIssued --> OperationExpired",
        "OperationConsumed --> OperationUncertain",
        "no current issuer transition produces it",
        "`ParentExpired` is currently an error classification",
        "Self-loops in the diagrams visualize state preservation",
        "No macOS implementation",
    ):
        if marker not in text:
            failures.append(f"missing lifecycle marker: {marker}")
    return failures


def validate_sources(root: Path = ROOT) -> dict[str, Any]:
    grant_source = (root / GRANT_PATH.relative_to(ROOT)).read_text(encoding="utf-8")
    issuer_source = (root / ISSUER_PATH.relative_to(ROOT)).read_text(encoding="utf-8")
    document = (root / DOC_PATH.relative_to(ROOT)).read_text(encoding="utf-8")
    statuses = rust_enum_variants(grant_source, "GrantStatus")
    failures = validate_document(document)
    if statuses != GRANT_STATUSES:
        failures.append(f"grant statuses differ: expected {GRANT_STATUSES!r}, got {statuses!r}")
    for marker in ISSUER_MARKERS:
        if marker not in issuer_source:
            failures.append(f"missing issuer transition marker: {marker}")
    if "GrantStatus::Revoked" in issuer_source:
        failures.append("revoked is now implemented but the reference says it is reserved")
    if issuer_source.count("self.transition_status(") != 2:
        failures.append("private terminal transition call-site count changed")
    if failures:
        raise GrantStateError("; ".join(failures))
    return {
        "grant_status_count": len(statuses),
        "implemented_transition_count": len(TRANSITIONS),
        "session_parent_transition_count": 3,
        "operation_transition_count": 5,
        "reserved_without_producer": ["Revoked"],
        "authority_terminal_statuses": [
            "Consumed",
            "Revoked",
            "Expired",
            "Invalidated",
            "Uncertain",
        ],
    }


def git_revision(root: Path = ROOT) -> str:
    completed = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=root,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=10,
    )
    revision = completed.stdout.decode("ascii", "strict").strip()
    if completed.returncode != 0 or re.fullmatch(r"[0-9a-f]{40}", revision) is None:
        raise GrantStateError("source revision is unavailable")
    return revision


def git_file(revision: str, relative: str, root: Path = ROOT) -> bytes:
    completed = subprocess.run(
        ["git", "show", f"{revision}:{relative}"],
        cwd=root,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=10,
    )
    if completed.returncode != 0:
        raise GrantStateError(f"source is absent at revision: {relative}")
    return completed.stdout


def build_report(reference_revision: str, root: Path = ROOT) -> dict[str, Any]:
    coverage = validate_sources(root)
    ancestor = subprocess.run(
        ["git", "merge-base", "--is-ancestor", reference_revision, "HEAD"],
        cwd=root,
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        timeout=10,
    )
    if ancestor.returncode != 0:
        raise GrantStateError("reference revision is not an ancestor of HEAD")
    sources = []
    for relative in SOURCE_PATHS:
        committed = git_file(reference_revision, relative, root)
        current = (root / relative).read_bytes()
        if committed != current:
            raise GrantStateError(f"source differs from reference revision: {relative}")
        sources.append({"path": relative, "sha256": sha256_bytes(committed)})
    return {
        "schema_version": 1,
        "task_id": "5.1.2.2",
        "artifact_id": "grant-state-transition-reference",
        "status": "pass-shared-linux-reference",
        "reference_revision": reference_revision,
        "sources": sources,
        "coverage": coverage,
        "verification": {
            "closed_status_enum": "pass",
            "parent_transition_paths": "pass",
            "operation_transition_paths": "pass",
            "state_preserving_denials": "pass",
            "reserved_revocation_disclosure": "pass",
            "documentation_coverage": "pass",
        },
        "platform_status": {
            "shared_contracts": "verified-local",
            "linux_reference": "verified-local",
            "macos": "blocked-macos",
            "macos_implementation_claim": "none",
        },
        "limitations": [
            "Grant lifecycle state is in-memory and is not crash durable.",
            "Parent expiry during derivation does not rewrite the retained parent.",
            "Revoked is a reserved wire status without an implemented issuer transition.",
            "No isolated worker execution or final effect receipt is claimed.",
            "No macOS implementation or execution evidence is claimed.",
        ],
    }


def write_report(root: Path = ROOT) -> None:
    revision = git_revision(root)
    write_atomic(REPORT_PATH, canonical_json(build_report(revision, root)))


def check_report(root: Path = ROOT) -> None:
    try:
        actual = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        revision = actual["reference_revision"]
    except (OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise GrantStateError(f"cannot read grant-state report: {error}") from error
    if not isinstance(revision, str) or actual != build_report(revision, root):
        raise GrantStateError("grant-state report is stale or malformed")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            write_report()
        check_report()
    except (OSError, GrantStateError, subprocess.SubprocessError) as error:
        print(f"Grant-state validation failed: {error}", file=sys.stderr)
        return 1
    print("Grant-state transition reference validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
