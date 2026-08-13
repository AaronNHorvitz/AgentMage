#!/usr/bin/env python3
"""Build and validate isolated firewall and packet-capture evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import tempfile
from pathlib import Path
from typing import Any, Final

from scripts import strict_local_capture_harness as harness


ROOT: Final = Path(__file__).resolve().parents[1]
REPORT_PATH: Final = (
    ROOT / "artifacts/sprints/sprint-10/story-10.1/firewall-packet-capture.json"
)
SOURCE_PATHS: Final = (
    "docs/architecture/strict-local-boundary.md",
    "scripts/firewall_packet_capture_evidence.py",
    "scripts/strict_local_capture_harness.py",
    "tests/test_firewall_packet_capture_evidence.py",
    "tests/test_strict_local_capture_harness.py",
)
COMMAND_SPECS: Final = (
    (
        (
            "python3",
            "-m",
            "unittest",
            "tests.test_strict_local_capture_harness",
            "tests.test_firewall_packet_capture_evidence",
        ),
        "Ran 12 tests",
    ),
    (
        ("python3", "scripts/strict_local_capture_harness.py", "--run"),
        "Strict-local isolated firewall and packet-capture harness passed",
    ),
    (
        ("npm", "run", "product:lint"),
        "Structural effect mediation boundary validated.",
    ),
)
HARNESS_PROFILE: Final = {
    "privilege_scope": "disposable-user-and-network-namespace-only",
    "interfaces": ["loopback", "synthetic-test-net-dummy"],
    "default_route_count": 0,
    "host_interface_count": 0,
    "firewall": "exact-nftables-output-default-drop",
    "capture": "in-memory-af-packet-counts-only",
    "allowed_fixture": "bounded-loopback-tcp",
    "blocked_fixture": "synthetic-test-net-dns-datagram",
    "cleanup": "namespace-exit",
}
CLAIMS: Final = {
    "isolated_firewall_harness_implemented": True,
    "structured_firewall_policy_verified": True,
    "loopback_packets_observed": True,
    "synthetic_dns_egress_blocked_with_zero_emitted_frames": True,
    "packet_payload_retained": False,
    "host_firewall_modified": False,
    "product_workflow_observed": False,
    "sixty_minute_capture_performed": False,
    "external_network_used": False,
    "inference_performed": False,
    "release_support": False,
}
LIMITATIONS: Final = [
    "This fixture proves the Linux harness and its cleanup in a disposable user/network namespace; it does not inspect or modify the host firewall.",
    "The capture exercises bounded synthetic loopback and TEST-NET traffic, not an AgentMage product session, model runtime, physical network, or every v0.1 workflow.",
    "The required 60-minute packet, DNS, socket, process, and firewall observation remains assigned to Sub-task 10.1.3.2.",
    "No packet payload or pcap is retained, and no inference, cross-platform certification, or release acceptance is claimed.",
]
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class FirewallCaptureEvidenceError(ValueError):
    """Raised when firewall/capture evidence is invalid or overstated."""


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
        raise FirewallCaptureEvidenceError("source revision is unavailable")
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
        raise FirewallCaptureEvidenceError("committed source is unavailable")
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
    if completed.returncode != 0 or marker not in completed.stdout + completed.stderr:
        raise FirewallCaptureEvidenceError(
            f"verification command failed: {Path(arguments[0]).name}"
        )
    return command_record(arguments, marker)


def build_report(revision: str) -> dict[str, Any]:
    capture_result = harness.run_harness()
    if errors := harness.validate_result(capture_result):
        raise FirewallCaptureEvidenceError("; ".join(errors))
    return {
        "schema_version": 1,
        "artifact_id": "strict-local-isolated-firewall-packet-capture",
        "source_revision": revision,
        "task_ids": ["10.1.2.3"],
        "status": "pass-isolated-linux-firewall-and-capture-harness",
        "harness_profile": HARNESS_PROFILE,
        "capture_result": capture_result,
        "verification_commands": [
            run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS
        ],
        "claims": CLAIMS,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "external_network_used": False,
        "sources": source_records(revision),
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    exact = {
        "schema_version": 1,
        "artifact_id": "strict-local-isolated-firewall-packet-capture",
        "task_ids": ["10.1.2.3"],
        "status": "pass-isolated-linux-firewall-and-capture-harness",
        "harness_profile": HARNESS_PROFILE,
        "verification_commands": expected_commands(),
        "claims": CLAIMS,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "external_network_used": False,
    }
    for key, value in exact.items():
        if report.get(key) != value:
            errors.append(f"firewall capture {key} changed")
    capture = report.get("capture_result")
    if not isinstance(capture, dict):
        errors.append("firewall capture result is invalid")
    else:
        errors.extend(harness.validate_result(capture))
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        errors.append("firewall capture source revision is invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(
        SOURCE_PATHS
    ):
        errors.append("firewall capture source evidence changed")
    elif any(
        not isinstance(item.get("bytes"), int)
        or item["bytes"] <= 0
        or SHA256.fullmatch(str(item.get("sha256", ""))) is None
        for item in sources
    ):
        errors.append("firewall capture source records are invalid")
    return errors


def validate_committed_sources(report: dict[str, Any]) -> None:
    revision = str(report["source_revision"])
    for item in report["sources"]:
        if hashlib.sha256(git_bytes(revision, item["path"])).hexdigest() != item["sha256"]:
            raise FirewallCaptureEvidenceError("committed source binding changed")


def read_report(path: Path) -> dict[str, Any]:
    try:
        report = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise FirewallCaptureEvidenceError("evidence report is unavailable") from error
    if not isinstance(report, dict):
        raise FirewallCaptureEvidenceError("evidence report is invalid")
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
            raise FirewallCaptureEvidenceError("; ".join(errors))
        write_atomic(REPORT_PATH, report)
    report = read_report(REPORT_PATH)
    if errors := validate_report(report):
        raise FirewallCaptureEvidenceError("; ".join(errors))
    validate_committed_sources(report)
    print("Strict-local firewall and packet-capture evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
