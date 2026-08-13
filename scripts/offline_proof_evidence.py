#!/usr/bin/env python3
"""Build and validate content-free offline-proof workflow evidence."""

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


ROOT: Final = Path(__file__).resolve().parents[1]
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-10/story-10.1/offline-proof.json"
SOURCE_PATHS: Final = (
    "docs/architecture/strict-local-boundary.md",
    "kernel/engine/src/strict_local.rs",
    "scripts/offline_proof_evidence.py",
    "tests/test_offline_proof_evidence.py",
)
COMMAND_SPECS: Final = (
    (
        (
            "cargo",
            "test",
            "-p",
            "agentmage-kernel-engine",
            "strict_local_offline_proof",
            "--locked",
        ),
        "7 passed; 0 failed",
    ),
    (
        (
            "cargo",
            "test",
            "-p",
            "agentmage-kernel-engine",
            "ledger_",
            "--locked",
        ),
        "3 passed; 0 failed",
    ),
    (
        (
            "cargo",
            "clippy",
            "-p",
            "agentmage-kernel-engine",
            "--all-targets",
            "--locked",
            "--",
            "-D",
            "warnings",
        ),
        "Finished",
    ),
)
POLICY_PROFILE: Final = {
    "ledger_record_limit": 4096,
    "ledger_identity_fields": [
        "sequence",
        "observation-time",
        "executable-sha256",
        "component",
        "destination-class",
        "local-transport-class",
        "endpoint-sha256",
        "attempted-byte-count",
        "kernel-decision",
    ],
    "terminal_artifact_mapping": {
        "completed": "activated-verified",
        "cancelled": "removed",
        "corrupt": "quarantined",
    },
    "required_zero_observations": [
        "acquisition-process-count",
        "acquisition-socket-count",
        "external-network-rule-count",
        "observed-outbound-bytes",
        "observed-dns-attempts",
    ],
    "required_boundary_identities": [
        "session-boundary-sha256",
        "firewall-policy-sha256",
    ],
    "proofs_per_workflow_instance": 1,
}
VERIFICATION_RESULTS: Final = {
    "exact_terminal_outcomes_accepted": 3,
    "lifecycle_and_replay_fail_closed": True,
    "stale_preflight_fails_closed": True,
    "active_process_socket_or_rule_fails_closed": True,
    "artifact_mismatch_matrix_fails_closed": True,
    "missing_boundary_identity_fails_closed": True,
    "outbound_bytes_or_dns_fails_closed": True,
    "invalid_ledger_range_order_time_or_identity_fails_closed": True,
    "ledger_and_proof_hashes_are_deterministic_and_fact_bound": True,
}
CLAIMS: Final = {
    "kernel_content_free_ledger_implemented": True,
    "kernel_one_way_offline_proof_implemented": True,
    "production_model_acquisition_wired": False,
    "live_firewall_observation_performed": False,
    "live_packet_capture_performed": False,
    "inference_performed": False,
    "release_support": False,
}
LIMITATIONS: Final = [
    "This artifact verifies the deterministic kernel ledger and offline-proof state machine; it does not claim that a production model installer supplies the observations.",
    "The workflow is non-cloneable and issues one proof per instance; the future acquisition orchestrator must enforce unique construction for each acquisition epoch.",
    "Live acquisition lifecycle wiring, firewall inspection, syscall, DNS, and socket tracing, packet capture, and all-workflow offline acceptance remain separate Sprint 10 tasks.",
    "This evidence performs no inference and establishes no macOS, Windows, physical-host cross-distribution, or release acceptance.",
]
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class OfflineProofEvidenceError(ValueError):
    """Raised when offline-proof evidence is incomplete or overstated."""


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
        raise OfflineProofEvidenceError("source revision is unavailable")
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
        raise OfflineProofEvidenceError("committed source is unavailable")
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
        raise OfflineProofEvidenceError(
            f"verification command failed: {Path(arguments[0]).name}"
        )
    return command_record(arguments, marker)


def build_report(revision: str) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "artifact_id": "strict-local-offline-proof",
        "source_revision": revision,
        "task_ids": ["10.1.1.7"],
        "status": "pass-kernel-ledger-and-offline-proof-contract",
        "policy_profile": POLICY_PROFILE,
        "verification_results": VERIFICATION_RESULTS,
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
        "artifact_id": "strict-local-offline-proof",
        "task_ids": ["10.1.1.7"],
        "status": "pass-kernel-ledger-and-offline-proof-contract",
        "policy_profile": POLICY_PROFILE,
        "verification_results": VERIFICATION_RESULTS,
        "verification_commands": expected_commands(),
        "claims": CLAIMS,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "external_network_used": False,
    }
    for key, value in exact.items():
        if report.get(key) != value:
            errors.append(f"offline proof {key} changed")
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        errors.append("offline proof source revision is invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(
        SOURCE_PATHS
    ):
        errors.append("offline proof source evidence changed")
    elif any(
        not isinstance(item.get("bytes"), int)
        or item["bytes"] <= 0
        or SHA256.fullmatch(str(item.get("sha256", ""))) is None
        for item in sources
    ):
        errors.append("offline proof source records are invalid")
    return errors


def read_report(path: Path) -> dict[str, Any]:
    try:
        report = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise OfflineProofEvidenceError("evidence report is unavailable") from error
    if not isinstance(report, dict):
        raise OfflineProofEvidenceError("evidence report is invalid")
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
            raise OfflineProofEvidenceError("; ".join(errors))
        write_atomic(REPORT_PATH, report)
    report = read_report(REPORT_PATH)
    if errors := validate_report(report):
        raise OfflineProofEvidenceError("; ".join(errors))
    print("Strict-local offline-proof evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
