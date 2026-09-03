#!/usr/bin/env python3
"""Build and validate hidden network-surface detection evidence."""

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
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-10/story-10.1/hidden-network-surfaces.json"
SOURCE_PATHS: Final = (
    "Cargo.lock",
    "Cargo.toml",
    "capabilities/read-only/Cargo.toml",
    "docs/architecture/strict-local-boundary.md",
    "kernel/contracts/Cargo.toml",
    "kernel/engine/Cargo.toml",
    "package-lock.json",
    "package.json",
    "platforms/linux-inference/Cargo.toml",
    "platforms/linux/Cargo.toml",
    "platforms/windows/Cargo.toml",
    "release/xtask/Cargo.toml",
    "scripts/hidden_network_surface_evidence.py",
    "scripts/strict_local_source_audit.py",
    "security/strict-local-source-policy.json",
    "shells/host/Cargo.toml",
    "shells/vscode/package.json",
    "tests/test_hidden_network_surface_evidence.py",
    "tests/test_strict_local_source_audit.py",
)
COMMAND_SPECS: Final = (
    (
        ("python3", "-m", "unittest", "tests.test_strict_local_source_audit"),
        "Ran 11 tests",
    ),
    (
        ("npm", "run", "product:lint"),
        "Structural effect mediation boundary validated.",
    ),
)
POLICY_PROFILE: Final = {
    "source_root_count": 9,
    "compiled_vscode_source_included": True,
    "exact_bridge_fragment_count": 12,
    "network_symbol_rule_count": 10,
    "exact_external_uri_fixture_count": 3,
    "reviewed_cargo_package_identity_count": 60,
    "sha256_bound_cargo_manifest_count": 9,
    "approved_first_party_build_script_count": 0,
    "approved_vscode_runtime_package_count": 0,
    "vscode_manifest_boundary": [
        "exact-top-level-keys",
        "exact-startup-event",
        "ui-process-only",
        "exact-compiled-entry-point",
        "exact-package-scripts",
        "single-chat-provider-contribution",
        "runtime-manifest-lock-agreement",
    ],
}
MUTATION_RESULTS: Final = {
    "source_features_rejected": [
        "ambient-proxy",
        "child-process-download",
        "crash-upload",
        "marketplace-socket",
        "remote-font-or-asset",
        "telemetry-upload",
        "update-download",
        "vscode-external-open",
    ],
    "compiled_artifact_telemetry_rejected": True,
    "extension_dependency_rejected": True,
    "uri_activation_rejected": True,
    "install_script_rejected": True,
    "extra_contribution_surface_rejected": True,
    "npm_runtime_lock_drift_rejected": True,
    "cargo_package_addition_rejected": True,
    "cargo_manifest_or_feature_change_rejected": True,
    "first_party_build_script_rejected": True,
}
CLAIMS: Final = {
    "static_source_and_package_gate_implemented": True,
    "standard_product_lint_integration_tested": True,
    "runtime_attempt_absence_proven": False,
    "packet_capture_performed": False,
    "all_third_party_code_semantically_audited": False,
    "inference_performed": False,
    "release_support": False,
}
LIMITATIONS: Final = [
    "The gate detects reviewed source, compiled JavaScript, manifest, script, feature, package, and known network-API changes; it is not a semantic proof that every third-party implementation is benign.",
    "Runtime syscall tracing, DNS and socket observation, packet capture, and hostile dependency startup tests remain separate Sprint 10 tasks.",
    "This evidence performs no inference and does not establish macOS, Windows, physical-host cross-distribution, or release acceptance.",
]
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class HiddenNetworkEvidenceError(ValueError):
    """Raised when hidden-network evidence is incomplete or overstated."""


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
        raise HiddenNetworkEvidenceError("source revision is unavailable")
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
        raise HiddenNetworkEvidenceError("committed source is unavailable")
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
        raise HiddenNetworkEvidenceError(
            f"verification command failed: {Path(arguments[0]).name}"
        )
    return command_record(arguments, marker)


def build_report(revision: str) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "artifact_id": "strict-local-hidden-network-surfaces",
        "source_revision": revision,
        "task_ids": ["10.1.1.6"],
        "status": "pass-static-source-artifact-manifest-and-dependency-gate",
        "policy_profile": POLICY_PROFILE,
        "mutation_results": MUTATION_RESULTS,
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
        "artifact_id": "strict-local-hidden-network-surfaces",
        "task_ids": ["10.1.1.6"],
        "status": "pass-static-source-artifact-manifest-and-dependency-gate",
        "policy_profile": POLICY_PROFILE,
        "mutation_results": MUTATION_RESULTS,
        "verification_commands": expected_commands(),
        "claims": CLAIMS,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "external_network_used": False,
    }
    for key, value in exact.items():
        if report.get(key) != value:
            errors.append(f"hidden network {key} changed")
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        errors.append("hidden network source revision is invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(
        SOURCE_PATHS
    ):
        errors.append("hidden network source evidence changed")
    elif any(
        not isinstance(item.get("bytes"), int)
        or item["bytes"] <= 0
        or SHA256.fullmatch(str(item.get("sha256", ""))) is None
        for item in sources
    ):
        errors.append("hidden network source records are invalid")
    return errors


def read_report(path: Path) -> dict[str, Any]:
    try:
        report = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise HiddenNetworkEvidenceError("evidence report is unavailable") from error
    if not isinstance(report, dict):
        raise HiddenNetworkEvidenceError("evidence report is invalid")
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
            raise HiddenNetworkEvidenceError("; ".join(errors))
        write_atomic(REPORT_PATH, report)
    report = read_report(REPORT_PATH)
    if errors := validate_report(report):
        raise HiddenNetworkEvidenceError("; ".join(errors))
    print("Strict-local hidden network-surface evidence validated")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except HiddenNetworkEvidenceError as error:
        print(f"Hidden network-surface evidence failed: {error}", file=sys.stderr)
        sys.exit(1)
