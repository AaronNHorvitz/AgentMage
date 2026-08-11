#!/usr/bin/env python3
"""Build and verify approval-preview and policy-denial review fixtures."""

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
FIXTURE_ROOT = ROOT / "fixtures/grants/v1"
MANIFEST_PATH = FIXTURE_ROOT / "manifest.json"
REPORT_PATH = ROOT / "artifacts/sprints/sprint-5/story-5.1/grant-review-fixture-report.json"
MARKER = "AGENTMAGE_GRANT_REVIEW_FIXTURES="
ZERO_HASH = "0" * 64
EXPECTED_NAMES = (
    "approval-preview.workspace-read.json",
    "denial-receipt.argument-mismatch.json",
    "policy-denial.argument-mismatch.json",
)
SOURCE_PATHS = (
    "fixtures/grants/README.md",
    "fixtures/grants/v1/approval-preview.workspace-read.json",
    "fixtures/grants/v1/denial-receipt.argument-mismatch.json",
    "fixtures/grants/v1/policy-denial.argument-mismatch.json",
    "fixtures/grants/v1/manifest.json",
    "kernel/contracts/src/approval.rs",
    "kernel/contracts/src/evidence.rs",
    "kernel/engine/src/approval.rs",
    "kernel/engine/src/grants.rs",
    "kernel/engine/src/policy.rs",
    "kernel/engine/tests/grant_review_fixtures.rs",
    "scripts/grant_review_fixtures.py",
    "tests/test_grant_review_fixtures.py",
)


class GrantReviewFixtureError(ValueError):
    """Raised when a review fixture or its provenance is inconsistent."""


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def compact_json(value: Any) -> bytes:
    return json.dumps(value, separators=(",", ":"), ensure_ascii=True).encode("utf-8")


def canonical_report_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-grant-fixture-", dir=path.parent
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


def parse_bundle(output: bytes) -> dict[str, bytes]:
    if len(output) > 2 * 1024 * 1024:
        raise GrantReviewFixtureError("typed fixture generator output is oversized")
    decoded = output.decode("utf-8", "strict")
    payloads = [line.split(MARKER, 1)[1] for line in decoded.splitlines() if MARKER in line]
    if len(payloads) != 1:
        raise GrantReviewFixtureError("typed fixture generator marker count is invalid")
    try:
        records = json.loads(payloads[0])
    except json.JSONDecodeError as error:
        raise GrantReviewFixtureError("typed fixture bundle is malformed") from error
    if not isinstance(records, list):
        raise GrantReviewFixtureError("typed fixture bundle must be an array")
    fixtures: dict[str, bytes] = {}
    for record in records:
        if not isinstance(record, dict) or set(record) != {"canonical_json", "name"}:
            raise GrantReviewFixtureError("typed fixture record shape is invalid")
        name = record["name"]
        content = record["canonical_json"]
        if (
            not isinstance(name, str)
            or not isinstance(content, str)
            or re.fullmatch(r"[a-z0-9.-]+\.json", name) is None
            or name in fixtures
        ):
            raise GrantReviewFixtureError("typed fixture identity is invalid")
        fixtures[name] = content.encode("utf-8")
    if tuple(sorted(fixtures)) != EXPECTED_NAMES:
        raise GrantReviewFixtureError("typed fixture set is incomplete or broadened")
    return fixtures


def generated_fixtures(root: Path = ROOT) -> dict[str, bytes]:
    environment = {**os.environ, "AGENTMAGE_EMIT_GRANT_REVIEW_FIXTURES": "1"}
    completed = subprocess.run(
        [
            "cargo",
            "test",
            "--locked",
            "--offline",
            "-p",
            "agentmage-kernel-engine",
            "--test",
            "grant_review_fixtures",
            "approval_and_denial_review_fixtures_match_kernel_behavior",
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
        raise GrantReviewFixtureError("typed Rust fixture generator failed")
    return parse_bundle(completed.stdout)


def manifest_for(fixtures: dict[str, bytes]) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "fixture_set": "grant-review-v1",
        "generator": "kernel/engine/tests/grant_review_fixtures.rs",
        "fixtures": [
            {
                "path": f"fixtures/grants/v1/{name}",
                "sha256": sha256_bytes(fixtures[name]),
                "size_bytes": len(fixtures[name]),
            }
            for name in EXPECTED_NAMES
        ],
        "invariants": {
            "approval_is_non_authoritative": True,
            "approval_confirmation_is_exact": True,
            "denial_uses_actual_policy_engine": True,
            "denial_terminalizes_without_use": True,
            "receipt_is_non_authoritative": True,
            "receipt_runtime_builder_status": "not-implemented",
        },
        "platform_status": {
            "shared_contracts": "verified-local",
            "linux_reference": "verified-local",
            "macos": "blocked-macos",
            "macos_implementation_claim": "none",
        },
    }


def validate_fixture_values(fixtures: dict[str, bytes]) -> dict[str, Any]:
    try:
        approval = json.loads(fixtures[EXPECTED_NAMES[0]])
        receipt = json.loads(fixtures[EXPECTED_NAMES[1]])
        denial = json.loads(fixtures[EXPECTED_NAMES[2]])
    except (KeyError, json.JSONDecodeError) as error:
        raise GrantReviewFixtureError("fixture JSON is missing or malformed") from error
    forbidden = {"nonce", "use_limit", "use_count", "status", "grant_class"}
    if forbidden.intersection(approval):
        raise GrantReviewFixtureError("approval fixture contains grant authority fields")
    confirmation = approval.get("confirmation_sha256")
    approval_preimage = dict(approval)
    approval_preimage["confirmation_sha256"] = ZERO_HASH
    if confirmation != sha256_bytes(compact_json(approval_preimage)):
        raise GrantReviewFixtureError("approval confirmation digest is invalid")
    if set(denial) != {
        "schema_version",
        "grant_id",
        "policy_sha256",
        "allowed",
        "denial_scope",
        "code",
        "consume_code",
        "resulting_status",
        "resulting_revision",
        "resulting_use_count",
    }:
        raise GrantReviewFixtureError("policy denial fixture shape changed")
    if (
        denial["allowed"] is not False
        or denial["denial_scope"] != "argument"
        or denial["code"] != "policy.deny.argument"
        or denial["consume_code"] != "grant.consume.policy_denied"
        or denial["resulting_status"] != "invalidated"
        or denial["resulting_revision"] != 2
        or denial["resulting_use_count"] != 0
    ):
        raise GrantReviewFixtureError("policy denial fixture does not fail closed")
    evidence = receipt.get("evidence")
    error = receipt.get("error")
    if (
        receipt.get("outcome") != "denied"
        or not isinstance(evidence, list)
        or len(evidence) != 1
        or evidence[0].get("kind") != "decision"
        or evidence[0].get("content_sha256") != sha256_bytes(fixtures[EXPECTED_NAMES[2]])
        or evidence[0].get("object_id") != denial["grant_id"]
        or not isinstance(error, dict)
        or error.get("code") != denial["code"]
        or error.get("category") != "policy"
        or error.get("retry") != "never"
        or receipt.get("previous_receipt_sha256") != ZERO_HASH
    ):
        raise GrantReviewFixtureError("denial receipt does not bind the policy decision")
    receipt_preimage = dict(receipt)
    recorded_receipt_hash = receipt_preimage["receipt_sha256"]
    receipt_preimage["receipt_sha256"] = ZERO_HASH
    if recorded_receipt_hash != sha256_bytes(compact_json(receipt_preimage)):
        raise GrantReviewFixtureError("denial receipt digest is invalid")
    if (
        approval.get("proposed_grant_id") != denial["grant_id"]
        or approval.get("policy_sha256") != denial["policy_sha256"]
        or approval.get("tool_call", {}).get("correlation_id") != receipt.get("correlation_id")
        or approval.get("tool_call", {}).get("action_id") != receipt.get("action_id")
    ):
        raise GrantReviewFixtureError("fixture identities do not cross-link")
    return {
        "fixture_count": 3,
        "approval_field_count": len(approval),
        "approval_forbidden_authority_field_count": 0,
        "denial_scope": denial["denial_scope"],
        "denial_resulting_status": denial["resulting_status"],
        "denial_resulting_use_count": denial["resulting_use_count"],
        "receipt_outcome": receipt["outcome"],
    }


def write_fixtures(root: Path = ROOT) -> None:
    fixtures = generated_fixtures(root)
    validate_fixture_values(fixtures)
    for name, content in fixtures.items():
        write_atomic(root / FIXTURE_ROOT.relative_to(ROOT) / name, content)
    write_atomic(
        root / MANIFEST_PATH.relative_to(ROOT),
        canonical_report_json(manifest_for(fixtures)),
    )


def check_fixtures(root: Path = ROOT) -> dict[str, Any]:
    generated = generated_fixtures(root)
    retained = {
        name: (root / FIXTURE_ROOT.relative_to(ROOT) / name).read_bytes()
        for name in EXPECTED_NAMES
    }
    if retained != generated:
        raise GrantReviewFixtureError("retained grant review fixtures are stale")
    coverage = validate_fixture_values(retained)
    manifest = json.loads((root / MANIFEST_PATH.relative_to(ROOT)).read_text(encoding="utf-8"))
    if manifest != manifest_for(retained):
        raise GrantReviewFixtureError("grant review fixture manifest is stale")
    return coverage


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
        raise GrantReviewFixtureError("source revision is unavailable")
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
        raise GrantReviewFixtureError(f"source is absent at revision: {relative}")
    return completed.stdout


def build_report(reference_revision: str, root: Path = ROOT) -> dict[str, Any]:
    coverage = check_fixtures(root)
    ancestor = subprocess.run(
        ["git", "merge-base", "--is-ancestor", reference_revision, "HEAD"],
        cwd=root,
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        timeout=10,
    )
    if ancestor.returncode != 0:
        raise GrantReviewFixtureError("reference revision is not an ancestor of HEAD")
    sources = []
    for relative in SOURCE_PATHS:
        committed = git_file(reference_revision, relative, root)
        current = (root / relative).read_bytes()
        if committed != current:
            raise GrantReviewFixtureError(f"source differs from reference revision: {relative}")
        sources.append({"path": relative, "sha256": sha256_bytes(committed)})
    return {
        "schema_version": 1,
        "task_id": "5.1.2.3",
        "artifact_id": "grant-approval-denial-review-fixtures",
        "status": "pass-shared-linux-reference",
        "reference_revision": reference_revision,
        "sources": sources,
        "coverage": coverage,
        "verification": {
            "typed_generation": "pass",
            "approval_confirmation_digest": "pass",
            "authority_field_absence": "pass",
            "actual_policy_denial": "pass",
            "denial_terminalization": "pass",
            "receipt_decision_binding": "pass",
            "cross_fixture_identity": "pass",
        },
        "platform_status": manifest_for(generated_fixtures(root))["platform_status"],
        "limitations": [
            "Approval display verification does not record an authenticated user decision.",
            "The denial receipt is a review fixture over the shared Receipt contract.",
            "A production receipt builder, durable chain, and keyed integrity are not implemented.",
            "No isolated worker execution or effect reconciliation is claimed.",
            "No macOS implementation or execution evidence is claimed.",
        ],
    }


def write_report(root: Path = ROOT) -> None:
    write_atomic(REPORT_PATH, canonical_report_json(build_report(git_revision(root), root)))


def check_report(root: Path = ROOT) -> None:
    try:
        actual = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        revision = actual["reference_revision"]
    except (OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise GrantReviewFixtureError(f"cannot read grant review report: {error}") from error
    if not isinstance(revision, str) or actual != build_report(revision, root):
        raise GrantReviewFixtureError("grant review fixture report is stale or malformed")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write-fixtures", action="store_true")
    parser.add_argument("--fixtures-only", action="store_true")
    parser.add_argument("--write-report", action="store_true")
    args = parser.parse_args()
    try:
        if sum((args.write_fixtures, args.fixtures_only, args.write_report)) > 1:
            raise GrantReviewFixtureError("select only one operation")
        if args.write_fixtures:
            write_fixtures()
        elif args.fixtures_only:
            check_fixtures()
        elif args.write_report:
            write_report()
        else:
            check_report()
    except (OSError, GrantReviewFixtureError, subprocess.SubprocessError) as error:
        print(f"Grant review fixture validation failed: {error}", file=sys.stderr)
        return 1
    print("Grant approval and denial review fixtures validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
