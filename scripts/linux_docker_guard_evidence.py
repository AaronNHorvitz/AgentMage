#!/usr/bin/env python3
"""Build and validate the inactive Linux Docker raw-endpoint guard evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
PROFILE_PATH: Final = (
    ROOT / "model-profiles/runtimes/docker-model-runner-guard-v1-linux-x86_64.json"
)
REPORT_PATH: Final = (
    ROOT / "artifacts/sprints/sprint-9/story-9.2/linux-docker-endpoint-guard.json"
)
PROFILE_SHA256: Final = (
    "88fb0d5a78829cbdfc34af5cbcbfe3ca2a80f550889947e6478fdb66bf7edb2a"
)
RUNNER_DIGEST: Final = (
    "sha256:bd94095bbc1ddc4266c3a88f582a92562c6b63eceb175572c9a60045663727c9"
)
SOURCE_PATHS: Final = (
    "Cargo.lock",
    "Cargo.toml",
    "RUNTIME-BOUNDARIES.md",
    "architecture/build-contract.json",
    "architecture/component-inventory-policy.json",
    "docs/decisions/0030-closed-linux-docker-model-runner-compatibility-profile.md",
    "docs/decisions/0031-private-docker-model-runner-endpoint-guard.md",
    "model-profiles/runtimes/docker-model-runner-guard-v1-linux-x86_64.json",
    "model-profiles/runtimes/docker-model-runner-v1.2.6-linux-x86_64.json",
    "package.json",
    "platforms/linux-inference/Cargo.toml",
    "platforms/linux-inference/README.md",
    "platforms/linux-inference/src/docker_guard.rs",
    "platforms/linux-inference/src/docker_runtime.rs",
    "platforms/linux-inference/src/lib.rs",
    "scripts/build_contract.py",
    "scripts/component_inventory.py",
    "scripts/linux_docker_guard_evidence.py",
    "tests/test_component_inventory.py",
    "tests/test_linux_docker_guard_evidence.py",
)
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
DENIED_CALLERS: Final = [
    "vscode-extension",
    "tool-worker",
    "unrelated-same-user-process",
    "arbitrary-container",
    "undeclared",
]
LIMITATIONS: Final = [
    "The guard is a compiled fail-closed contract; no production proxy, Docker daemon, container, namespace, socket, or packet-filter rule was started on this host.",
    "The current host has no Docker CLI, and no live Docker Engine reachability claim is made.",
    "The exact runner and model identities remain inactive with zero enabled models and no inference operation.",
    "Independent control disablement and hostile live probes remain owned by Sub-tasks 9.2.1.4 and 9.2.2.2.",
]


class LinuxDockerGuardEvidenceError(ValueError):
    """Raised when Docker guard evidence is missing, stale, or overstated."""


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(prefix=".agentmage-docker-guard-", dir=path.parent)
    temporary = Path(name)
    try:
        with os.fdopen(descriptor, "wb") as stream:
            stream.write(content)
            stream.flush()
            os.fsync(stream.fileno())
        temporary.chmod(0o644)
        os.replace(temporary, path)
    except BaseException:
        temporary.unlink(missing_ok=True)
        raise


def run_command(arguments: list[str], timeout: int = 900) -> str:
    environment = os.environ.copy()
    environment["CARGO_NET_OFFLINE"] = "true"
    result = subprocess.run(
        arguments,
        cwd=ROOT,
        env=environment,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=timeout,
        check=False,
    )
    if result.returncode != 0:
        raise LinuxDockerGuardEvidenceError(
            f"verification command failed: {Path(arguments[0]).name}"
        )
    return result.stdout


def git_revision(candidate: str) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"],
        cwd=ROOT,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True,
        timeout=20,
        check=False,
    )
    revision = result.stdout.strip()
    if result.returncode != 0 or REVISION.fullmatch(revision) is None:
        raise LinuxDockerGuardEvidenceError("source revision is unavailable")
    return revision


def committed_file(revision: str, relative: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{relative}"],
        cwd=ROOT,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=20,
        check=False,
    )
    if result.returncode != 0:
        raise LinuxDockerGuardEvidenceError(f"committed source absent: {relative}")
    return result.stdout


def source_records(revision: str) -> list[dict[str, Any]]:
    records = []
    for relative in SOURCE_PATHS:
        content = committed_file(revision, relative)
        current = ROOT / relative
        if not current.is_file() or current.read_bytes() != content:
            raise LinuxDockerGuardEvidenceError(f"source differs from revision: {relative}")
        records.append(
            {"bytes": len(content), "path": relative, "sha256": sha256_bytes(content)}
        )
    return records


def load_profile() -> dict[str, Any]:
    profile = json.loads(PROFILE_PATH.read_text(encoding="utf-8"))
    failures = validate_profile(profile)
    if sha256_file(PROFILE_PATH) != PROFILE_SHA256:
        failures.append("Docker guard profile SHA-256 changed")
    if failures:
        raise LinuxDockerGuardEvidenceError("; ".join(failures))
    return profile


def validate_profile(profile: Any) -> list[str]:
    if not isinstance(profile, dict):
        return ["Docker guard profile must be an object"]
    failures = []
    if (
        profile.get("schema_version") != 1
        or profile.get("record_type") != "linux_docker_model_runner_guard_profile"
        or profile.get("platform") != "linux"
        or profile.get("architecture") != "x86_64"
        or profile.get("runner_image_digest") != RUNNER_DIGEST
    ):
        failures.append("Docker guard profile identity changed")
    if profile.get("authority") != {
        "credential": False,
        "docker_daemon_control": False,
        "grant": False,
        "host_filesystem": False,
        "network_egress": False,
        "tool": False,
        "workspace": False,
    }:
        failures.append("Docker guard authority closure changed")
    if profile.get("callers") != {
        "allowed": ["guarded-kernel-docker-adapter"],
        "denied": DENIED_CALLERS,
    }:
        failures.append("Docker guard caller matrix changed")
    if profile.get("raw_runtime_endpoint") != {
        "container_bridge_routes": 0,
        "host": "127.0.0.1",
        "host_tcp_listeners": 0,
        "namespace": "private-runner-and-guard-only",
        "non_loopback_listeners": 0,
        "port": 12434,
        "transport": "loopback-tcp",
    }:
        failures.append("Docker raw endpoint isolation changed")
    if profile.get("guard_process") != {
        "docker_socket_mounts": 0,
        "executable_identity_required": True,
        "network_namespace": "same-private-namespace-as-runner",
        "process_cgroup_identity_required": True,
        "runtime_user": "dedicated-non-root-agentmage-dmr-guard",
        "workspace_mounts": 0,
    }:
        failures.append("Docker guard process closure changed")
    if profile.get("kernel_transport") != {
        "authentication": "fresh-session-peer-credentials-and-challenge",
        "mode": 0o600,
        "owner": "invoking-standard-user",
        "raw_endpoint_fields_exposed": False,
        "transport": "authenticated-unix-socket",
    }:
        failures.append("Docker guard kernel transport changed")
    if profile.get("decision") != {
        "docker_engine_directly_tested": False,
        "enforcement_live_tested": False,
        "inference_implemented": False,
        "release_approval": False,
        "status": "GUARD_CONTRACT_PINNED_NOT_ACTIVATED",
    }:
        failures.append("Docker endpoint guard was overclaimed")
    return failures


def host_identity() -> dict[str, Any]:
    values = {}
    for line in Path("/etc/os-release").read_text(encoding="utf-8").splitlines():
        if "=" in line:
            key, value = line.split("=", 1)
            values[key] = value.strip('"')
    return {
        "architecture": platform.machine(),
        "distribution": values.get("ID", "unknown"),
        "version": values.get("VERSION_ID", "unknown"),
        "docker_cli_available": shutil.which("docker") is not None,
        "live_enforcement_executed": False,
    }


def build_report(revision: str) -> dict[str, Any]:
    profile = load_profile()
    run_command(["cargo", "test", "-p", "agentmage-platform-linux-inference", "--locked"])
    run_command(
        [
            "cargo",
            "clippy",
            "-p",
            "agentmage-platform-linux-inference",
            "--all-targets",
            "--locked",
            "--",
            "-D",
            "warnings",
        ]
    )
    run_command(["python3", "scripts/build_contract.py"])
    host = host_identity()
    if host != {
        "architecture": "x86_64",
        "distribution": "fedora",
        "version": "44",
        "docker_cli_available": False,
        "live_enforcement_executed": False,
    }:
        raise LinuxDockerGuardEvidenceError(
            "this evidence increment requires the declared Docker-absent Fedora host"
        )
    return {
        "allowed_caller": "guarded-kernel-docker-adapter",
        "artifact_id": "linux-docker-model-runner-endpoint-guard",
        "authority": profile["authority"],
        "denied_callers": DENIED_CALLERS,
        "docker_engine_directly_tested": False,
        "guard_process": profile["guard_process"],
        "host": host,
        "inference_started": False,
        "kernel_transport": profile["kernel_transport"],
        "limitations": LIMITATIONS,
        "model_count": 0,
        "permit_contract": {
            "cloneable": False,
            "constructible_outside_guard": False,
            "contains_address": False,
            "contains_credential": False,
            "serializable": False,
        },
        "profile_sha256": PROFILE_SHA256,
        "raw_runtime_endpoint": profile["raw_runtime_endpoint"],
        "release_claim": "none",
        "runner_image_digest": RUNNER_DIGEST,
        "schema_version": 1,
        "source_revision": revision,
        "sources": source_records(revision),
        "status": "pass-guard-contract-no-live-docker",
        "task_ids": ["9.2.1.3"],
    }


def validate_report(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["Linux Docker guard report must be an object"]
    failures = []
    if (
        value.get("schema_version") != 1
        or value.get("artifact_id") != "linux-docker-model-runner-endpoint-guard"
        or value.get("task_ids") != ["9.2.1.3"]
        or value.get("status") != "pass-guard-contract-no-live-docker"
        or REVISION.fullmatch(str(value.get("source_revision"))) is None
    ):
        failures.append("Linux Docker guard report identity changed")
    profile = json.loads(PROFILE_PATH.read_text(encoding="utf-8"))
    if validate_profile(profile):
        failures.append("checked Docker guard profile is invalid")
    if (
        value.get("authority") != profile.get("authority")
        or value.get("allowed_caller") != "guarded-kernel-docker-adapter"
        or value.get("denied_callers") != DENIED_CALLERS
        or value.get("guard_process") != profile.get("guard_process")
        or value.get("kernel_transport") != profile.get("kernel_transport")
        or value.get("raw_runtime_endpoint") != profile.get("raw_runtime_endpoint")
    ):
        failures.append("Docker guard topology or caller closure changed")
    if value.get("permit_contract") != {
        "cloneable": False,
        "constructible_outside_guard": False,
        "contains_address": False,
        "contains_credential": False,
        "serializable": False,
    }:
        failures.append("Docker raw-endpoint permit surface broadened")
    host = value.get("host", {})
    if (
        host.get("architecture") != "x86_64"
        or host.get("distribution") != "fedora"
        or host.get("version") != "44"
        or host.get("docker_cli_available") is not False
        or host.get("live_enforcement_executed") is not False
    ):
        failures.append("Docker-absent guard host evidence changed")
    if (
        value.get("docker_engine_directly_tested") is not False
        or value.get("inference_started") is not False
        or value.get("model_count") != 0
        or value.get("profile_sha256") != PROFILE_SHA256
        or value.get("runner_image_digest") != RUNNER_DIGEST
        or value.get("release_claim") != "none"
        or value.get("limitations") != LIMITATIONS
    ):
        failures.append("Docker endpoint guard state was overclaimed")
    sources = value.get("sources")
    if (
        not isinstance(sources, list)
        or [item.get("path") for item in sources] != list(SOURCE_PATHS)
        or any(
            SHA256.fullmatch(str(item.get("sha256"))) is None
            or not isinstance(item.get("bytes"), int)
            for item in sources
        )
    ):
        failures.append("Docker guard source closure is invalid")
    return failures


def check_report() -> None:
    try:
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise LinuxDockerGuardEvidenceError(
            f"cannot read Linux Docker guard report: {error}"
        ) from error
    failures = validate_report(report)
    revision = report.get("source_revision")
    if isinstance(revision, str):
        try:
            expected_sources = source_records(revision)
        except LinuxDockerGuardEvidenceError as error:
            failures.append(str(error))
        else:
            if report.get("sources") != expected_sources:
                failures.append("Docker guard report source records are stale")
    if failures:
        raise LinuxDockerGuardEvidenceError("; ".join(failures))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-revision", default="HEAD")
    parser.add_argument("--write", action="store_true")
    arguments = parser.parse_args(argv)
    try:
        if arguments.write:
            write_atomic(
                REPORT_PATH,
                canonical_json(build_report(git_revision(arguments.source_revision))),
            )
        check_report()
    except (
        LinuxDockerGuardEvidenceError,
        OSError,
        TypeError,
        UnicodeError,
        json.JSONDecodeError,
        subprocess.SubprocessError,
    ) as error:
        print(f"Linux Docker guard evidence failed: {error}", file=sys.stderr)
        return 1
    print("Linux Docker Model Runner endpoint guard evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
