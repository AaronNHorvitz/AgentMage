#!/usr/bin/env python3
"""Build and verify the Story 5.1 grant-schema and policy reference report."""

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
DOC_PATH = ROOT / "docs/architecture/grant-policy-reference.md"
GRANT_PATH = ROOT / "kernel/contracts/src/grant.rs"
POLICY_PATH = ROOT / "kernel/engine/src/policy.rs"
REPORT_PATH = (
    ROOT / "artifacts/sprints/sprint-5/story-5.1/grant-policy-reference-report.json"
)
SOURCE_PATHS = (
    "docs/architecture/grant-policy-reference.md",
    "kernel/contracts/src/grant.rs",
    "kernel/engine/src/policy.rs",
    "scripts/grant_policy_reference.py",
    "tests/test_grant_policy_reference.py",
)
GRANT_FIELDS = (
    "schema_version",
    "grant_id",
    "revision",
    "grant_class",
    "actor_id",
    "session_id",
    "task_id",
    "action_id",
    "action_kind",
    "operation",
    "tool_id",
    "tool_version",
    "targets",
    "excluded_targets",
    "sensitivity",
    "argument_sha256",
    "preimages",
    "expected_side_effects",
    "rollback_description",
    "issued_at_epoch_ms",
    "expires_at_epoch_ms",
    "nonce",
    "use_limit",
    "use_count",
    "parent_grant_id",
    "parent_grant_sha256",
    "preview_sha256",
    "policy_sha256",
    "status",
)
DENIAL_SCOPES = (
    "Grant",
    "Actor",
    "Session",
    "Task",
    "Action",
    "Tool",
    "Operation",
    "Path",
    "Argument",
    "Preimage",
    "SideEffect",
    "Preview",
    "Network",
    "Credential",
    "Publication",
)
DENIAL_CODES = (
    "policy.deny.grant",
    "policy.deny.actor",
    "policy.deny.session",
    "policy.deny.task",
    "policy.deny.action",
    "policy.deny.tool",
    "policy.deny.operation",
    "policy.deny.path",
    "policy.deny.argument",
    "policy.deny.preimage",
    "policy.deny.side_effect",
    "policy.deny.preview",
    "policy.deny.network",
    "policy.deny.credential",
    "policy.deny.publication",
)
STRICT_DENIED = (
    "WorkspaceWrite",
    "WorkspaceDelete",
    "CommandExecute",
    "NetworkAccess",
    "GitCommit",
    "GitPush",
    "Publish",
    "Send",
    "Upload",
    "Deploy",
    "DatabaseWrite",
    "CredentialAccess",
)
ALL_OPERATIONS = (
    "WorkspaceRead",
    *STRICT_DENIED[:4],
    *STRICT_DENIED[4:10],
    "DatabaseRead",
    *STRICT_DENIED[10:],
    "ModelInference",
)
REQUIRED_HEADINGS = (
    "Status and Scope",
    "Grant Schema",
    "Policy Document",
    "Policy Decision Table",
    "Strict-Local Operation Matrix",
    "Review Checklist",
    "Limitations",
)


class GrantPolicyReferenceError(ValueError):
    """Raised when the reference or its generated evidence is inconsistent."""


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-grant-reference-", dir=path.parent
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


def rust_struct_fields(source: str, name: str) -> tuple[str, ...]:
    match = re.search(rf"pub struct {re.escape(name)}\s*\{{(.*?)\n\}}", source, re.DOTALL)
    if match is None:
        raise GrantPolicyReferenceError(f"missing Rust struct: {name}")
    return tuple(re.findall(r"^\s*pub\s+([a-z][a-z0-9_]*)\s*:", match.group(1), re.MULTILINE))


def rust_enum_variants(source: str, name: str) -> tuple[str, ...]:
    match = re.search(rf"pub enum {re.escape(name)}\s*\{{(.*?)\n\}}", source, re.DOTALL)
    if match is None:
        raise GrantPolicyReferenceError(f"missing Rust enum: {name}")
    return tuple(
        re.findall(r"^\s*([A-Z][A-Za-z0-9_]*)\s*(?:\([^\n]*\))?,?\s*$", match.group(1), re.MULTILINE)
    )


def strict_denied_operations(source: str) -> tuple[str, ...]:
    match = re.search(
        r"STRICT_LOCAL_DENIED_OPERATIONS:\s*\[GrantOperation;\s*12\]\s*=\s*\[(.*?)\];",
        source,
        re.DOTALL,
    )
    if match is None:
        raise GrantPolicyReferenceError("strict-local denied-operation constant is absent")
    return tuple(re.findall(r"GrantOperation::([A-Z][A-Za-z0-9_]*)", match.group(1)))


def denial_codes(source: str) -> tuple[str, ...]:
    match = re.search(r"pub const fn code\(self\).*?\{(.*?)\n\s*\}\n\s*\}", source, re.DOTALL)
    if match is None:
        raise GrantPolicyReferenceError("policy denial code mapping is absent")
    return tuple(re.findall(r'"(policy\.deny\.[a-z_]+)"', match.group(1)))


def validate_reference_text(text: str) -> list[str]:
    failures: list[str] = []
    for heading in REQUIRED_HEADINGS:
        if f"## {heading}" not in text:
            failures.append(f"missing heading: {heading}")
    for field in GRANT_FIELDS:
        if f"`{field}`" not in text:
            failures.append(f"missing grant field: {field}")
    for scope, code in zip(DENIAL_SCOPES, DENIAL_CODES, strict=True):
        if f"`{scope}`" not in text:
            failures.append(f"missing policy scope: {scope}")
        if f"`{code}`" not in text:
            failures.append(f"missing policy denial code: {code}")
    for operation in ALL_OPERATIONS:
        wire_name = re.sub(r"(?<!^)(?=[A-Z])", "_", operation).lower()
        if f"`{wire_name}`" not in text:
            failures.append(f"missing strict-local operation: {wire_name}")
    for prohibited in (
        "macOS verification passed",
        "crash-durable grant store is complete",
        "production sandbox execution is complete",
    ):
        if prohibited in text:
            failures.append(f"unsupported claim: {prohibited}")
    return failures


def validate_sources(root: Path = ROOT) -> dict[str, Any]:
    grant_source = (root / GRANT_PATH.relative_to(ROOT)).read_text(encoding="utf-8")
    policy_source = (root / POLICY_PATH.relative_to(ROOT)).read_text(encoding="utf-8")
    document = (root / DOC_PATH.relative_to(ROOT)).read_text(encoding="utf-8")
    actual_fields = rust_struct_fields(grant_source, "CapabilityGrant")
    actual_operations = rust_enum_variants(grant_source, "GrantOperation")
    actual_scopes = rust_enum_variants(policy_source, "PolicyDenialScope")
    actual_codes = denial_codes(policy_source)
    actual_denied = strict_denied_operations(policy_source)
    failures = validate_reference_text(document)
    for label, actual, expected in (
        ("grant fields", actual_fields, GRANT_FIELDS),
        ("grant operations", actual_operations, ALL_OPERATIONS),
        ("policy denial scopes", actual_scopes, DENIAL_SCOPES),
        ("policy denial codes", actual_codes, DENIAL_CODES),
        ("strict denied operations", actual_denied, STRICT_DENIED),
    ):
        if actual != expected:
            failures.append(f"{label} differ: expected {expected!r}, got {actual!r}")
    if failures:
        raise GrantPolicyReferenceError("; ".join(failures))
    return {
        "grant_field_count": len(actual_fields),
        "grant_operation_count": len(actual_operations),
        "policy_denial_scope_count": len(actual_scopes),
        "strict_explicit_denial_count": len(actual_denied),
        "strict_allowed_operations": ["WorkspaceRead"],
        "strict_denied_by_absence": ["DatabaseRead", "ModelInference"],
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
        raise GrantPolicyReferenceError("source revision is unavailable")
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
        raise GrantPolicyReferenceError(f"source is absent at revision: {relative}")
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
        raise GrantPolicyReferenceError("reference revision is not an ancestor of HEAD")
    sources = []
    for relative in SOURCE_PATHS:
        committed = git_file(reference_revision, relative, root)
        current = (root / relative).read_bytes()
        if committed != current:
            raise GrantPolicyReferenceError(f"source differs from reference revision: {relative}")
        sources.append({"path": relative, "sha256": sha256_bytes(committed)})
    return {
        "schema_version": 1,
        "task_id": "5.1.2.1",
        "artifact_id": "grant-schema-policy-decision-reference",
        "status": "pass-shared-linux-reference",
        "reference_revision": reference_revision,
        "sources": sources,
        "coverage": coverage,
        "verification": {
            "grant_field_order": "pass",
            "closed_operation_order": "pass",
            "policy_scope_and_code_order": "pass",
            "strict_local_denials": "pass",
            "documentation_coverage": "pass",
        },
        "platform_status": {
            "shared_contracts": "verified-local",
            "linux_reference": "verified-local",
            "macos": "blocked-macos",
            "macos_implementation_claim": "none",
        },
        "limitations": [
            "Grant and policy state is in-memory and is not crash durable.",
            "No production sandbox worker or authority-bearing tool execution is claimed.",
            "Path candidates are not canonical platform descriptors until Sprint 6.",
            "Startup configuration is not yet bound to the strict-local policy constructor.",
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
        raise GrantPolicyReferenceError(f"cannot read reference report: {error}") from error
    if not isinstance(revision, str) or actual != build_report(revision, root):
        raise GrantPolicyReferenceError("grant-policy reference report is stale or malformed")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            write_report()
        check_report()
    except (OSError, GrantPolicyReferenceError, subprocess.SubprocessError) as error:
        print(f"Grant-policy reference validation failed: {error}", file=sys.stderr)
        return 1
    print("Grant-schema and policy decision reference validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
