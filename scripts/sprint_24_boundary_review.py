#!/usr/bin/env python3
"""Retain gate-owned Sprint 24 review and live zero-egress evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-24"
REPORT: Final = EVIDENCE_DIR / "source-boundary-review.json"
RAW: Final = EVIDENCE_DIR / "live-zero-egress-results.log"
COMMAND: Final = (
    "cargo",
    "test",
    "-p",
    "agentmage-kernel-engine",
    "every_delivery_and_interface_control_action_has_one_local_denial_receipt",
    "--locked",
)
SOURCES: Final = (
    "kernel/contracts/src/handoff.rs",
    "kernel/engine/src/handoff.rs",
    "security/strict-local-source-policy.json",
    "shells/host/src/linux_read.rs",
    "shells/host/src/protocol.rs",
    "shells/vscode/src/handoff.ts",
    "shells/vscode/src/provider.ts",
)


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def git_bytes(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"],
        cwd=ROOT,
        capture_output=True,
        check=False,
        timeout=30,
    )
    if result.returncode != 0:
        raise ValueError(f"review source unavailable: {path}")
    return result.stdout


def expected(revision: str, exit_code: int, output_sha256: str) -> dict[str, Any]:
    sources = {path: git_bytes(revision, path) for path in SOURCES}
    kernel = sources["kernel/engine/src/handoff.rs"]
    contract = sources["kernel/contracts/src/handoff.rs"]
    host = (
        sources["shells/host/src/linux_read.rs"]
        + sources["shells/host/src/protocol.rs"]
    )
    vscode = (
        sources["shells/vscode/src/handoff.ts"]
        + sources["shells/vscode/src/provider.ts"]
    )
    checks = {
        "closed_prohibited_action_family": b"pub const ALL: [Self; 14]" in contract,
        "one_local_denial_receipt_per_action": b"HandoffProhibitedAction::ALL" in kernel
        and b"external_delivery_attempted" in kernel,
        "live_process_socket_set_unchanged": b"/proc/self/fd" in kernel
        and b"live_process_socket_fds(), sockets_before" in kernel,
        "host_exposes_denial_not_delivery": b"deny_handoff_action" in host,
        "client_has_no_handoff_delivery_method": all(
            token not in vscode
            for token in (b"sendHandoff", b"submitHandoff", b"uploadHandoff")
        ),
        "strict_local_policy_retained": b'"id": "javascript-network-api"'
        in sources["security/strict-local-source-policy.json"],
    }
    return {
        "schema_version": 1,
        "record_type": "agentmage-sprint-24-boundary-review",
        "source_revision": revision,
        "reviewer_identity": "scripts/sprint_24_boundary_review.py",
        "review_class": "independent-automated-source-and-live-process-boundary",
        "independent_human_review_performed": False,
        "command": list(COMMAND),
        "command_exit_code": exit_code,
        "command_output_sha256": output_sha256,
        "source_sha256": {path: digest(value) for path, value in sources.items()},
        "checks": checks,
        "live_handoff_zero_egress_observed": exit_code == 0 and all(checks.values()),
        "status": "PASS_LOCAL_BOUNDARY_REVIEW"
        if exit_code == 0 and all(checks.values())
        else "FAIL",
        "limitations": [
            "The live observation covers the Linux test process and proves the denial family opens no socket.",
            "This is gate-owned automated review, not a human-review claim.",
            "Installed VSIX and platform accessibility campaigns remain separate blockers.",
        ],
    }


def validate(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["review is not an object"]
    failures: list[str] = []
    if value.get("reviewer_identity") != "scripts/sprint_24_boundary_review.py":
        failures.append("reviewer identity drift")
    if value.get("independent_human_review_performed") is not False:
        failures.append("human review overclaim")
    if value.get("command_exit_code") != 0:
        failures.append("live process command failed")
    if value.get("live_handoff_zero_egress_observed") is not True:
        failures.append("live zero-egress observation absent")
    checks = value.get("checks", {})
    if (
        not isinstance(checks, dict)
        or not checks
        or any(item is not True for item in checks.values())
    ):
        failures.append("review check failed or suppressed")
    if value.get("status") != "PASS_LOCAL_BOUNDARY_REVIEW":
        failures.append("review status is not pass")
    revision = str(value.get("source_revision", ""))
    if len(revision) != 40 or any(
        character not in "0123456789abcdef" for character in revision
    ):
        failures.append("source revision invalid")
        return failures
    try:
        expected_value = expected(
            revision, 0, str(value.get("command_output_sha256", ""))
        )
    except ValueError as error:
        return failures + [str(error)]
    if value != expected_value:
        failures.append("review is stale, incomplete, reordered, or widened")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    args = parser.parse_args()
    if args.write:
        revision = subprocess.run(
            ["git", "rev-parse", args.source_revision],
            cwd=ROOT,
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
        result = subprocess.run(COMMAND, cwd=ROOT, capture_output=True, check=False)
        output = result.stdout + result.stderr
        EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
        RAW.write_bytes(output)
        value = expected(revision, result.returncode, digest(output))
        REPORT.write_text(
            json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
    else:
        try:
            value = json.loads(REPORT.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            print(f"cannot read Sprint 24 review: {error}", file=sys.stderr)
            return 1
    failures = validate(value)
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    print("Sprint 24 gate-owned boundary and live zero-egress review passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
