#!/usr/bin/env python3
"""Build and validate the reviewable Sprint 10 strict-local policy profile."""

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
REPORT_PATH: Final = (
    ROOT / "artifacts/sprints/sprint-10/story-10.1/strict-local-policy-profile.json"
)
CONFIGURATION_PATH: Final = "configuration/profiles/strict-local-read-only.json"
SOURCE_PATHS: Final = (
    CONFIGURATION_PATH,
    "configuration/profiles/catalog.json",
    "docs/architecture/strict-local-boundary.md",
    "kernel/engine/src/configuration.rs",
    "kernel/engine/src/policy.rs",
    "kernel/engine/src/strict_local.rs",
    "platforms/linux/src/inventory.rs",
    "platforms/linux/src/strict_local.rs",
    "scripts/strict_local_policy_profile.py",
    "security/strict-local-source-policy.json",
    "tests/test_strict_local_policy_profile.py",
)
DEPENDENCY_EVIDENCE_PATHS: Final = (
    "artifacts/sprints/sprint-3/story-3.1/profile-catalog-report.json",
    "artifacts/sprints/sprint-10/story-10.1/strict-local-network-policy.json",
    "artifacts/sprints/sprint-10/story-10.1/inference-client-isolation.json",
    "artifacts/sprints/sprint-10/story-10.1/listener-boundary.json",
    "artifacts/sprints/sprint-10/story-10.1/state-root-policy.json",
    "artifacts/sprints/sprint-10/story-10.1/session-boundary.json",
    "artifacts/sprints/sprint-10/story-10.1/hidden-network-surfaces.json",
    "artifacts/sprints/sprint-10/story-10.1/offline-proof.json",
)
COMMAND_SPECS: Final = (
    (
        ("python3", "scripts/configuration_profiles.py"),
        "Story 3.1 configuration profiles validated",
    ),
    (
        (
            "cargo",
            "test",
            "-p",
            "agentmage-kernel-engine",
            "strict_local::tests",
            "--locked",
        ),
        "14 passed; 0 failed",
    ),
    (
        (
            "cargo",
            "test",
            "-p",
            "agentmage-platform-linux",
            "strict_local_",
            "--locked",
        ),
        "16 passed; 0 failed; 2 ignored",
    ),
    (
        ("python3", "scripts/strict_local_source_audit.py"),
        "Strict-local source audit passed with zero undeclared network paths.",
    ),
)
POLICY_PROFILE: Final = {
    "configuration": {
        "profile_id": "strict-local-read-only",
        "schema_version": 2,
        "strict_local": True,
        "startup_failure_policy": "fail-closed",
        "inheritance_mode": "restrict-only",
        "activation_status": "inactive-no-product-registration",
        "effective_capabilities": ["workspace.read"],
        "enabled_model_count": 0,
        "registered_tool_count": 0,
    },
    "authority": {
        "default_effect": "deny",
        "model_authority": "none",
        "grant_scope": "exact-single-use-expiring-nontransferable",
        "workspace_access": "one-explicit-read-only-root",
        "direct_command_execution": False,
        "model_to_tool_channel": "prohibited",
    },
    "network": {
        "normal_operation": "deny-all-except-selected-guarded-local-inference-path",
        "configuration_endpoint_count": 0,
        "native_client": "kernel-native-inference-adapter",
        "native_transport": "authenticated-unix-socket",
        "docker_client": "kernel-docker-inference-adapter",
        "docker_transport": "guarded-loopback-tcp",
        "undeclared_listener_policy": "reject",
        "content_free_attempt_limit": 4096,
    },
    "storage": {
        "strict_local_state_root": "local-known-filesystem-only",
        "cloud_synchronized": "reject",
        "remote_filesystem": "reject",
        "fuse_filesystem": "reject",
        "unknown_filesystem": "reject",
    },
    "privacy": {
        "record_prompts": False,
        "record_tool_arguments": False,
        "record_environment_values": False,
        "private_paths": "redact",
        "credentials": "redact",
    },
    "offline_return": {
        "workflow": "non-cloneable-one-way-per-instance",
        "requires_zero_processes_sockets_rules_outbound_bytes_and_dns": True,
        "requires_session_and_firewall_identities": True,
        "requires_exact_terminal_artifact_state": True,
    },
}
CLAIMS: Final = {
    "reviewable_policy_profile_produced": True,
    "kernel_and_linux_policy_matrices_passed": True,
    "product_profile_registered": False,
    "model_enabled": False,
    "production_startup_composed": False,
    "live_packet_capture_performed": False,
    "inference_performed": False,
    "cross_platform_release_support": False,
}
LIMITATIONS: Final = [
    "This is the reviewable current-foundation profile and its policy closure; the application host does not yet register or compose it into a complete product startup.",
    "The checked configuration has zero enabled models and zero registered tools, so the guarded local inference policy is present but inactive.",
    "The configuration carries a Fedora platform identity; Ubuntu equivalence and every other supported-platform claim require their separately assigned execution evidence.",
    "Live acquisition wiring, continuous process confinement, firewall observation, packet capture, all-workflow offline acceptance, inference, and release support remain open.",
]
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class StrictLocalPolicyProfileError(ValueError):
    """Raised when the reviewable strict-local profile is invalid or overstated."""


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
        raise StrictLocalPolicyProfileError("source revision is unavailable")
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
        raise StrictLocalPolicyProfileError("committed source is unavailable")
    return completed.stdout


def record(revision: str, path: str) -> dict[str, Any]:
    data = git_bytes(revision, path)
    return {"path": path, "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}


def validate_configuration(revision: str) -> str:
    try:
        configuration = json.loads(git_bytes(revision, CONFIGURATION_PATH))
        catalog = json.loads(
            git_bytes(revision, "configuration/profiles/catalog.json")
        )
    except (UnicodeError, json.JSONDecodeError) as error:
        raise StrictLocalPolicyProfileError("configuration is invalid") from error
    entries = [
        item
        for item in catalog.get("profiles", [])
        if item.get("profile_id") == "strict-local-read-only"
    ]
    expected = {
        "core": {
            "profile_id": "strict-local-read-only",
            "configuration_mode": "strict-local-read-only",
            "strict_local": True,
            "startup_failure_policy": "fail-closed",
            "inheritance_mode": "restrict-only",
        },
        "permission": {
            "allowed_capabilities": ["workspace.read"],
            "default_effect": "deny",
            "model_authority": "none",
            "network": {"mode": "deny-all", "allowed_endpoints": []},
        },
    }
    if len(entries) != 1 or any(
        configuration.get(section, {}).get(key) != value
        for section, values in expected.items()
        for key, value in values.items()
    ):
        raise StrictLocalPolicyProfileError("strict-local configuration changed")
    entry = entries[0]
    checks = (
        configuration.get("schema_version") == 2,
        entry.get("activation_status") == "inactive-no-product-registration",
        entry.get("product_registration") is False,
        entry.get("effective_capabilities") == ["workspace.read"],
        configuration.get("model", {}).get("enabled") is False,
        configuration.get("model", {}).get("network_access") is False,
        configuration.get("model", {}).get("automatic_routing") is False,
        configuration.get("tool", {}).get("tools") == [],
        configuration.get("shell", {}).get("network_access") is False,
        configuration.get("shell", {}).get("direct_command_execution") is False,
        configuration.get("shell", {}).get("model_to_tool_channel") == "prohibited",
        configuration.get("workspace", {}).get("default_access") == "deny",
        len(configuration.get("workspace", {}).get("roots", [])) == 1,
        configuration.get("workspace", {}).get("roots", [{}])[0].get("access")
        == "read-only",
        configuration.get("logging", {}).get("record_prompts") is False,
        configuration.get("logging", {}).get("record_tool_arguments") is False,
        configuration.get("logging", {}).get("record_environment_values") is False,
    )
    if not all(checks):
        raise StrictLocalPolicyProfileError("strict-local configuration broadened")
    return hashlib.sha256(git_bytes(revision, CONFIGURATION_PATH)).hexdigest()


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
        raise StrictLocalPolicyProfileError(
            f"verification command failed: {Path(arguments[0]).name}"
        )
    return command_record(arguments, marker)


def build_report(revision: str) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "artifact_id": "strict-local-policy-profile",
        "source_revision": revision,
        "task_ids": ["10.1.2.1"],
        "status": "pass-reviewable-current-foundation-profile",
        "configuration_sha256": validate_configuration(revision),
        "policy_profile": POLICY_PROFILE,
        "verification_commands": [
            run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS
        ],
        "claims": CLAIMS,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "external_network_used": False,
        "sources": [record(revision, path) for path in SOURCE_PATHS],
        "dependency_evidence": [
            record(revision, path) for path in DEPENDENCY_EVIDENCE_PATHS
        ],
    }


def valid_records(value: Any, paths: tuple[str, ...]) -> bool:
    return (
        isinstance(value, list)
        and [item.get("path") for item in value] == list(paths)
        and all(
            isinstance(item.get("bytes"), int)
            and item["bytes"] > 0
            and SHA256.fullmatch(str(item.get("sha256", ""))) is not None
            for item in value
        )
    )


def validate_report(report: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    exact = {
        "schema_version": 1,
        "artifact_id": "strict-local-policy-profile",
        "task_ids": ["10.1.2.1"],
        "status": "pass-reviewable-current-foundation-profile",
        "policy_profile": POLICY_PROFILE,
        "verification_commands": expected_commands(),
        "claims": CLAIMS,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "external_network_used": False,
    }
    for key, value in exact.items():
        if report.get(key) != value:
            errors.append(f"strict-local profile {key} changed")
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        errors.append("strict-local profile source revision is invalid")
    if SHA256.fullmatch(str(report.get("configuration_sha256", ""))) is None:
        errors.append("strict-local profile configuration identity is invalid")
    if not valid_records(report.get("sources"), SOURCE_PATHS):
        errors.append("strict-local profile source evidence changed")
    elif report["configuration_sha256"] != report["sources"][0]["sha256"]:
        errors.append("strict-local profile configuration identity changed")
    if not valid_records(report.get("dependency_evidence"), DEPENDENCY_EVIDENCE_PATHS):
        errors.append("strict-local profile dependency evidence changed")
    return errors


def validate_committed_bindings(report: dict[str, Any]) -> None:
    revision = str(report["source_revision"])
    expected = {
        item["path"]: item["sha256"]
        for family in (report["sources"], report["dependency_evidence"])
        for item in family
    }
    for path, digest in expected.items():
        if hashlib.sha256(git_bytes(revision, path)).hexdigest() != digest:
            raise StrictLocalPolicyProfileError("committed evidence binding changed")
    if validate_configuration(revision) != report["configuration_sha256"]:
        raise StrictLocalPolicyProfileError("committed configuration binding changed")


def read_report(path: Path) -> dict[str, Any]:
    try:
        report = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise StrictLocalPolicyProfileError("policy profile is unavailable") from error
    if not isinstance(report, dict):
        raise StrictLocalPolicyProfileError("policy profile is invalid")
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
            raise StrictLocalPolicyProfileError("; ".join(errors))
        write_atomic(REPORT_PATH, report)
    report = read_report(REPORT_PATH)
    if errors := validate_report(report):
        raise StrictLocalPolicyProfileError("; ".join(errors))
    validate_committed_bindings(report)
    print("Strict-local policy profile evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
