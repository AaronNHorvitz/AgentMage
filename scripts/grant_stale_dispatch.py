#!/usr/bin/env python3
"""Build and verify post-approval stale-authority dispatch evidence."""

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
REPORT_PATH = ROOT / "artifacts/sprints/sprint-5/story-5.1/grant-stale-dispatch-report.json"
MARKER = "AGENTMAGE_GRANT_STALE_DISPATCH="
MUTATIONS = ("policy", "preimage", "task", "preview")
SCOPES = {
    "policy": "grant",
    "preimage": "preimage",
    "task": "task",
    "preview": "preview",
}
SOURCE_PATHS = (
    "kernel/contracts/src/approval.rs",
    "kernel/contracts/src/grant.rs",
    "kernel/engine/src/approval.rs",
    "kernel/engine/src/grants.rs",
    "kernel/engine/src/policy.rs",
    "kernel/engine/tests/grant_stale_dispatch.rs",
    "scripts/grant_stale_dispatch.py",
    "tests/test_grant_stale_dispatch.py",
)


class GrantStaleDispatchError(ValueError):
    """Raised when stale-dispatch evidence is incomplete or inconsistent."""


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-stale-dispatch-", dir=path.parent
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


def run_traces(root: Path = ROOT) -> list[dict[str, Any]]:
    environment = {**os.environ, "AGENTMAGE_EMIT_GRANT_STALE_DISPATCH": "1"}
    completed = subprocess.run(
        [
            "cargo",
            "test",
            "--locked",
            "--offline",
            "-p",
            "agentmage-kernel-engine",
            "--test",
            "grant_stale_dispatch",
            "post_approval_changes_fail_at_final_consumption_before_worker_start",
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
        raise GrantStaleDispatchError("typed stale-dispatch verifier failed")
    decoded = completed.stdout.decode("utf-8", "strict")
    payloads = [line.split(MARKER, 1)[1] for line in decoded.splitlines() if MARKER in line]
    if len(payloads) != 1:
        raise GrantStaleDispatchError("typed stale-dispatch marker count is invalid")
    try:
        traces = json.loads(payloads[0])
    except json.JSONDecodeError as error:
        raise GrantStaleDispatchError("typed stale-dispatch traces are malformed") from error
    if not isinstance(traces, list):
        raise GrantStaleDispatchError("typed stale-dispatch traces must be an array")
    return traces


def validate_traces(traces: list[dict[str, Any]]) -> dict[str, Any]:
    if tuple(trace.get("mutation") for trace in traces) != MUTATIONS:
        raise GrantStaleDispatchError("stale-dispatch mutation closure or order changed")
    expected_fields = {
        "schema_version",
        "mutation",
        "approval_confirmation_matches_grant",
        "final_boundary",
        "denial_scope",
        "denial_code",
        "atomic_consumption_count",
        "worker_start_count",
        "effect_count",
        "terminal_status",
        "terminal_revision",
        "terminal_use_count",
        "exact_replay_allowed",
    }
    for trace in traces:
        scope = SCOPES[trace["mutation"]]
        if (
            set(trace) != expected_fields
            or trace["schema_version"] != 1
            or trace["approval_confirmation_matches_grant"] is not True
            or trace["final_boundary"] != "GrantIssuer::consume_for_execution"
            or trace["denial_scope"] != scope
            or trace["denial_code"] != f"policy.deny.{scope}"
            or trace["atomic_consumption_count"] != 0
            or trace["worker_start_count"] != 0
            or trace["effect_count"] != 0
            or trace["terminal_status"] != "invalidated"
            or trace["terminal_revision"] != 2
            or trace["terminal_use_count"] != 0
            or trace["exact_replay_allowed"] is not False
        ):
            raise GrantStaleDispatchError("stale-dispatch expectation changed")
    return {
        "mutation_count": len(traces),
        "atomic_consumption_count": sum(trace["atomic_consumption_count"] for trace in traces),
        "worker_start_count": sum(trace["worker_start_count"] for trace in traces),
        "effect_count": sum(trace["effect_count"] for trace in traces),
        "exact_replay_success_count": sum(trace["exact_replay_allowed"] for trace in traces),
        "terminal_invalidated_count": sum(
            trace["terminal_status"] == "invalidated" for trace in traces
        ),
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
        raise GrantStaleDispatchError("source revision is unavailable")
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
        raise GrantStaleDispatchError(f"source is absent at revision: {relative}")
    return completed.stdout


def build_report(reference_revision: str, root: Path = ROOT) -> dict[str, Any]:
    traces = run_traces(root)
    coverage = validate_traces(traces)
    ancestor = subprocess.run(
        ["git", "merge-base", "--is-ancestor", reference_revision, "HEAD"],
        cwd=root,
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        timeout=10,
    )
    if ancestor.returncode != 0:
        raise GrantStaleDispatchError("reference revision is not an ancestor of HEAD")
    sources = []
    for relative in SOURCE_PATHS:
        committed = git_file(reference_revision, relative, root)
        current = (root / relative).read_bytes()
        if committed != current:
            raise GrantStaleDispatchError(f"source differs from reference revision: {relative}")
        sources.append({"path": relative, "sha256": sha256_bytes(committed)})
    return {
        "schema_version": 1,
        "task_id": "5.1.3.4",
        "artifact_id": "post-approval-stale-dispatch-verification",
        "status": "pass-shared-linux-reference",
        "reference_revision": reference_revision,
        "sources": sources,
        "coverage": coverage,
        "traces": traces,
        "verification": {
            "rendered_approval_binding": "pass",
            "changed_policy_rejection": "pass",
            "changed_preimage_rejection": "pass",
            "changed_task_rejection": "pass",
            "changed_preview_rejection": "pass",
            "final_boundary_terminalization": "pass",
            "zero_worker_start_and_effect": "pass",
            "exact_replay_denial": "pass",
        },
        "platform_status": {
            "shared_contracts": "verified-local",
            "linux_reference": "verified-local",
            "macos": "blocked-macos",
            "macos_implementation_claim": "none",
        },
        "limitations": [
            "Final dispatch is represented by in-memory atomic consumption without a worker.",
            "The approval decision is rendered and hash-bound but is not authenticated or durable.",
            "Canonical platform preimage observation remains assigned to Sprint 6.",
            "No macOS implementation or execution evidence is claimed.",
        ],
    }


def write_report(root: Path = ROOT) -> None:
    write_atomic(REPORT_PATH, canonical_json(build_report(git_revision(root), root)))


def check_report(root: Path = ROOT) -> None:
    try:
        actual = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        revision = actual["reference_revision"]
    except (OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise GrantStaleDispatchError(f"cannot read stale-dispatch report: {error}") from error
    if not isinstance(revision, str) or actual != build_report(revision, root):
        raise GrantStaleDispatchError("stale-dispatch report is stale or malformed")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--traces-only", action="store_true")
    args = parser.parse_args()
    try:
        if args.write and args.traces_only:
            raise GrantStaleDispatchError("select only one operation")
        if args.write:
            write_report()
        elif args.traces_only:
            validate_traces(run_traces())
        else:
            check_report()
    except (OSError, GrantStaleDispatchError, subprocess.SubprocessError) as error:
        print(f"Stale-dispatch validation failed: {error}", file=sys.stderr)
        return 1
    print("Post-approval stale-dispatch verification validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
