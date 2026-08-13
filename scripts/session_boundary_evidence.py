#!/usr/bin/env python3
"""Build and validate strict-local Linux session-boundary evidence."""

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
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-10/story-10.1/session-boundary.json"
SOURCE_PATHS: Final = (
    "docs/architecture/strict-local-boundary.md",
    "kernel/engine/src/strict_local.rs",
    "platforms/linux/src/inventory.rs",
    "platforms/linux/src/lib.rs",
    "scripts/session_boundary_evidence.py",
    "tests/test_session_boundary_evidence.py",
)
COMMAND_SPECS: Final = (
    (
        (
            "cargo",
            "test",
            "-p",
            "agentmage-platform-linux",
            "strict_local_session_boundary",
            "--locked",
        ),
        "test result: ok. 5 passed; 0 failed; 1 ignored",
    ),
    (
        (
            "cargo",
            "test",
            "-p",
            "agentmage-platform-linux",
            "inventory::tests::strict_local_session_boundary_live_complete_cgroup_inventory",
            "--locked",
            "--",
            "--ignored",
            "--exact",
        ),
        "test result: ok. 1 passed; 0 failed; 0 ignored",
    ),
    (
        ("python3", "scripts/strict_local_source_audit.py"),
        "Strict-local source audit passed with zero undeclared network paths.",
    ),
)
POLICY_PROFILE: Final = {
    "inventory_scope": "complete-unified-cgroup",
    "membership_checks": [
        "all-declared-pids-share-one-unified-cgroup",
        "declared-pids-equal-kernel-members-before-collection",
        "kernel-members-unchanged-after-collection",
    ],
    "process_fields": [
        "component",
        "parent-component",
        "uid",
        "executable-sha256",
        "count",
    ],
    "socket_fields": [
        "component",
        "protocol",
        "state",
        "local-destination-class",
        "remote-destination-class",
        "local-port",
        "remote-port",
        "endpoint-sha256",
        "unique-object-count",
    ],
    "writable_fields": [
        "component",
        "target-class",
        "target-sha256",
        "count",
    ],
    "tool_fields": ["tool-id", "tool-version", "canonical-definition-sha256"],
    "network_rule": "sole-kernel-strict-local-endpoint-identity",
    "listener_rule": "exact-strict-local-listener-policy",
    "closed_limits": {
        "processes": 128,
        "sockets": 4096,
        "writable_descriptors": 4096,
        "tools": 256,
        "listeners": 32,
    },
    "refusal_classes": [
        "invalid-manifest",
        "resource-limit",
        "process-inventory-mismatch",
        "socket-inventory-mismatch",
        "writable-inventory-mismatch",
        "tool-inventory-mismatch",
        "network-rule-mismatch",
        "listener-boundary",
    ],
}
RESULTS: Final = {
    "exact_topology_admitted": True,
    "caller_selected_process_subset_rejected": True,
    "executable_and_parent_substitution_rejected": True,
    "extra_process_rejected": True,
    "socket_port_and_extra_object_rejected": True,
    "unattributed_socket_rejected": True,
    "writable_target_and_extra_target_rejected": True,
    "tool_definition_substitution_rejected": True,
    "network_rule_substitution_rejected": True,
    "listener_omission_rejected": True,
    "live_disposable_systemd_cgroup_collected": True,
}
CLAIMS: Final = {
    "linux_session_manifest_reconciliation_implemented": True,
    "complete_unified_cgroup_membership_tested_live": True,
    "continuous_process_confinement_tested": False,
    "short_lived_connection_absence_proven": False,
    "packet_capture_performed": False,
    "inference_performed": False,
    "release_support": False,
}
LIMITATIONS: Final = [
    "The report is a point-in-time startup snapshot; it does not establish continuous cgroup confinement or detect every short-lived process, descriptor, or connection after reconciliation.",
    "Runtime syscall tracing, packet capture, and the long-running offline workflow remain separate Sprint 10 acceptance tasks.",
    "This evidence is Linux-only and does not claim macOS, Windows, physical-host cross-distribution certification, inference, or release acceptance.",
]
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class SessionBoundaryEvidenceError(ValueError):
    """Raised when session-boundary evidence is incomplete or overstated."""


def git_revision(candidate: str) -> str:
    completed = subprocess.run(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True,
        timeout=30,
        check=False,
    )
    revision = completed.stdout.strip()
    if completed.returncode != 0 or REVISION.fullmatch(revision) is None:
        raise SessionBoundaryEvidenceError("source revision is unavailable")
    return revision


def git_bytes(revision: str, path: str) -> bytes:
    completed = subprocess.run(
        ["git", "show", f"{revision}:{path}"],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=60,
        check=False,
    )
    if completed.returncode != 0 or not completed.stdout:
        raise SessionBoundaryEvidenceError("committed source is unavailable")
    return completed.stdout


def source_records(revision: str) -> list[dict[str, Any]]:
    return [
        {
            "path": path,
            "bytes": len(data := git_bytes(revision, path)),
            "sha256": hashlib.sha256(data).hexdigest(),
        }
        for path in SOURCE_PATHS
    ]


def command_record(arguments: tuple[str, ...], marker: str) -> dict[str, Any]:
    return {
        "command_id": hashlib.sha256("\0".join(arguments).encode()).hexdigest(),
        "exit_code": 0,
        "expected_marker_sha256": hashlib.sha256(marker.encode()).hexdigest(),
        "status": "pass",
    }


def expected_commands() -> list[dict[str, Any]]:
    return [command_record(arguments, marker) for arguments, marker in COMMAND_SPECS]


def run_checked(arguments: tuple[str, ...], marker: str) -> dict[str, Any]:
    completed = subprocess.run(
        list(arguments),
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        timeout=300,
        check=False,
        env={**os.environ, "LANG": "C", "LC_ALL": "C"},
    )
    output = completed.stdout + completed.stderr
    if completed.returncode != 0 or marker not in output:
        raise SessionBoundaryEvidenceError(
            f"verification command failed: {Path(arguments[0]).name}"
        )
    return command_record(arguments, marker)


def build_report(revision: str) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "artifact_id": "strict-local-linux-session-boundary",
        "source_revision": revision,
        "task_ids": ["10.1.1.5"],
        "status": "pass-linux-point-in-time-session-reconciliation",
        "policy_profile": POLICY_PROFILE,
        "integration_results": RESULTS,
        "verification_commands": [
            run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS
        ],
        "claims": CLAIMS,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "external_network_used": False,
        "local_disposable_process_used": True,
        "sources": source_records(revision),
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    exact = {
        "schema_version": 1,
        "artifact_id": "strict-local-linux-session-boundary",
        "task_ids": ["10.1.1.5"],
        "status": "pass-linux-point-in-time-session-reconciliation",
        "policy_profile": POLICY_PROFILE,
        "integration_results": RESULTS,
        "verification_commands": expected_commands(),
        "claims": CLAIMS,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "external_network_used": False,
        "local_disposable_process_used": True,
    }
    for key, value in exact.items():
        if report.get(key) != value:
            errors.append(f"session boundary {key} changed")
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        errors.append("session boundary source revision is invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(
        SOURCE_PATHS
    ):
        errors.append("session boundary source evidence changed")
    elif any(
        not isinstance(item.get("bytes"), int)
        or item["bytes"] <= 0
        or SHA256.fullmatch(str(item.get("sha256", ""))) is None
        for item in sources
    ):
        errors.append("session boundary source records are invalid")
    return errors


def read_report(path: Path) -> dict[str, Any]:
    try:
        report = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise SessionBoundaryEvidenceError("evidence report is unavailable") from error
    if not isinstance(report, dict):
        raise SessionBoundaryEvidenceError("evidence report is invalid")
    return report


def write_atomic(path: Path, report: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as stream:
            json.dump(report, stream, indent=2, sort_keys=True)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    if arguments.write:
        report = build_report(git_revision(arguments.source_revision))
        if errors := validate_report(report):
            raise SessionBoundaryEvidenceError("; ".join(errors))
        write_atomic(REPORT_PATH, report)
    report = read_report(REPORT_PATH)
    if errors := validate_report(report):
        raise SessionBoundaryEvidenceError("; ".join(errors))
    print("Strict-local Linux session-boundary evidence validated")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except SessionBoundaryEvidenceError as error:
        print(f"Session-boundary evidence failed: {error}", file=sys.stderr)
        sys.exit(1)
