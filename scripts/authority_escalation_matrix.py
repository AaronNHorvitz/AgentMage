#!/usr/bin/env python3
"""Build and verify actor-attributed descriptive-authority denial evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
from collections import Counter
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
REPORT_PATH = ROOT / "artifacts/sprints/sprint-5/story-5.1/authority-escalation-report.json"
MARKER = "AGENTMAGE_AUTHORITY_ESCALATION_MATRIX="
SOURCES = (
    "model",
    "prompt",
    "shell",
    "tool",
    "plugin",
    "approval_display",
    "simulated_child",
)
ESCALATIONS = ("mint", "widen", "transfer", "combine")
ARTIFACTS = {
    "model": "prompt",
    "prompt": "prompt",
    "shell": "description",
    "tool": "tool_definition",
    "plugin": "description",
    "approval_display": "approval_request",
    "simulated_child": "prompt",
}
SOURCE_PATHS = (
    "kernel/engine/src/authority.rs",
    "kernel/engine/tests/authority_escalation_matrix.rs",
    "scripts/authority_escalation_matrix.py",
    "tests/test_authority_escalation_matrix.py",
)


class AuthorityEscalationError(ValueError):
    """Raised when authority-escalation evidence is incomplete or unsafe."""


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-authority-escalation-", dir=path.parent
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


def run_receipts(root: Path = ROOT) -> list[dict[str, Any]]:
    environment = {**os.environ, "AGENTMAGE_EMIT_AUTHORITY_ESCALATION_MATRIX": "1"}
    completed = subprocess.run(
        [
            "cargo",
            "test",
            "--locked",
            "--offline",
            "-p",
            "agentmage-kernel-engine",
            "--test",
            "authority_escalation_matrix",
            "every_producer_and_escalation_kind_is_denied_with_actor_attribution",
            "--",
            "--exact",
            "--nocapture",
            "--test-threads=1",
        ],
        cwd=root,
        check=False,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=120,
        env=environment,
    )
    if completed.returncode != 0:
        raise AuthorityEscalationError("typed authority-escalation verifier failed")
    if len(completed.stdout) > 2 * 1024 * 1024:
        raise AuthorityEscalationError("typed authority-escalation output is oversized")
    decoded = completed.stdout.decode("utf-8", "strict")
    payloads = [line.split(MARKER, 1)[1] for line in decoded.splitlines() if MARKER in line]
    if len(payloads) != 1:
        raise AuthorityEscalationError("typed authority-escalation marker count is invalid")
    try:
        receipts = json.loads(payloads[0])
    except json.JSONDecodeError as error:
        raise AuthorityEscalationError("typed authority-escalation receipts are malformed") from error
    if not isinstance(receipts, list):
        raise AuthorityEscalationError("typed authority-escalation receipts must be an array")
    return receipts


def validate_receipts(receipts: list[dict[str, Any]]) -> dict[str, Any]:
    expected_order = tuple((source, escalation) for source in SOURCES for escalation in ESCALATIONS)
    actual_order = tuple(
        (receipt.get("source"), receipt.get("escalation_kind")) for receipt in receipts
    )
    if actual_order != expected_order:
        raise AuthorityEscalationError("authority source/escalation closure or order changed")
    expected_fields = {
        "schema_version",
        "actor_id",
        "session_id",
        "task_id",
        "source",
        "escalation_kind",
        "artifact_kind",
        "authority_admitted",
        "error",
    }
    source_counts: Counter[str] = Counter()
    escalation_counts: Counter[str] = Counter()
    for receipt in receipts:
        source = receipt["source"]
        error = receipt.get("error")
        if (
            set(receipt) != expected_fields
            or receipt["schema_version"] != 2
            or receipt["actor_id"] != "actor-local-0001"
            or receipt["session_id"] != "session-0001"
            or receipt["task_id"] != "task-0001"
            or receipt["artifact_kind"] != ARTIFACTS[source]
            or receipt["authority_admitted"] is not False
            or not isinstance(error, dict)
            or set(error) != {
                "schema_version",
                "error_id",
                "code",
                "category",
                "message",
                "field_path",
                "retry",
                "caused_by",
            }
            or error.get("schema_version") != 2
            or error.get("error_id") != "error-descriptive-authority-denied"
            or error.get("code") != "authority.descriptive_artifact.denied"
            or error.get("category") != "policy"
            or error.get("message") != "Descriptive artifacts cannot authorize an operation"
            or error.get("retry") != "after_user_decision"
            or error.get("field_path") != []
            or error.get("caused_by") is not None
        ):
            raise AuthorityEscalationError("actor-attributed denial receipt changed")
        source_counts[source] += 1
        escalation_counts[receipt["escalation_kind"]] += 1
    encoded = json.dumps(receipts, separators=(",", ":"))
    for prohibited in (
        "FORGED_",
        "capability_grant",
        '"nonce"',
        '"use_limit"',
        '"use_count"',
        '"status"',
    ):
        if prohibited in encoded:
            raise AuthorityEscalationError("receipt retained content or authority material")
    if source_counts != Counter({source: 4 for source in SOURCES}):
        raise AuthorityEscalationError("authority source coverage changed")
    if escalation_counts != Counter({kind: 7 for kind in ESCALATIONS}):
        raise AuthorityEscalationError("authority escalation coverage changed")
    return {
        "receipt_count": len(receipts),
        "source_count": len(source_counts),
        "escalation_kind_count": len(escalation_counts),
        "admitted_authority_count": sum(receipt["authority_admitted"] for receipt in receipts),
        "actor_identity_count": len({receipt["actor_id"] for receipt in receipts}),
        "redacted_error_code_count": len({receipt["error"]["code"] for receipt in receipts}),
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
        raise AuthorityEscalationError("source revision is unavailable")
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
        raise AuthorityEscalationError(f"source is absent at revision: {relative}")
    return completed.stdout


def build_report(reference_revision: str, root: Path = ROOT) -> dict[str, Any]:
    receipts = run_receipts(root)
    coverage = validate_receipts(receipts)
    ancestor = subprocess.run(
        ["git", "merge-base", "--is-ancestor", reference_revision, "HEAD"],
        cwd=root,
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        timeout=10,
    )
    if ancestor.returncode != 0:
        raise AuthorityEscalationError("reference revision is not an ancestor of HEAD")
    sources = []
    for relative in SOURCE_PATHS:
        committed = git_file(reference_revision, relative, root)
        current = (root / relative).read_bytes()
        if committed != current:
            raise AuthorityEscalationError(f"source differs from reference revision: {relative}")
        sources.append({"path": relative, "sha256": sha256_bytes(committed)})
    return {
        "schema_version": 1,
        "task_id": "5.1.3.3",
        "artifact_id": "authority-escalation-denial-verification",
        "status": "pass-shared-linux-reference",
        "reference_revision": reference_revision,
        "sources": sources,
        "coverage": coverage,
        "receipts": receipts,
        "verification": {
            "source_class_closure": "pass",
            "escalation_kind_closure": "pass",
            "actor_session_task_attribution": "pass",
            "sealed_descriptive_rejection": "pass",
            "constant_denial_without_success_path": "pass",
            "candidate_content_redaction": "pass",
            "authority_material_absence": "pass",
        },
        "platform_status": {
            "shared_contracts": "verified-local",
            "linux_reference": "verified-local",
            "macos": "blocked-macos",
            "macos_implementation_claim": "none",
        },
        "limitations": [
            "Source and actor attribution are typed evidence but are not authenticated IPC claims.",
            "Receipts are in-memory non-authoritative values without durable keyed integrity.",
            "Plugin and child behavior is simulated with sealed descriptive artifacts.",
            "No production worker, plugin host, child runtime, or macOS implementation is claimed.",
        ],
    }


def write_report(root: Path = ROOT) -> None:
    write_atomic(REPORT_PATH, canonical_json(build_report(git_revision(root), root)))


def check_report(root: Path = ROOT) -> None:
    try:
        actual = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        revision = actual["reference_revision"]
    except (OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise AuthorityEscalationError(f"cannot read authority-escalation report: {error}") from error
    if not isinstance(revision, str) or actual != build_report(revision, root):
        raise AuthorityEscalationError("authority-escalation report is stale or malformed")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--receipts-only", action="store_true")
    args = parser.parse_args()
    try:
        if args.write and args.receipts_only:
            raise AuthorityEscalationError("select only one operation")
        if args.write:
            write_report()
        elif args.receipts_only:
            validate_receipts(run_receipts())
        else:
            check_report()
    except (OSError, AuthorityEscalationError, subprocess.SubprocessError) as error:
        print(f"Authority-escalation validation failed: {error}", file=sys.stderr)
        return 1
    print("Authority-escalation denial verification validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
