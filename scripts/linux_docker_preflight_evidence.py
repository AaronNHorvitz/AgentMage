#!/usr/bin/env python3
"""Build and validate source-bound Docker drift-preflight evidence."""

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
REPORT_PATH: Final = (
    ROOT / "artifacts/sprints/sprint-9/story-9.2/linux-docker-drift-preflight.json"
)
SOURCE_PATHS: Final = (
    "Cargo.lock",
    "Cargo.toml",
    "RUNTIME-BOUNDARIES.md",
    "architecture/build-contract.json",
    "docs/decisions/0030-closed-linux-docker-model-runner-compatibility-profile.md",
    "docs/decisions/0031-private-docker-model-runner-endpoint-guard.md",
    "docs/decisions/0032-fail-closed-docker-topology-preflight.md",
    "model-profiles/runtimes/docker-model-runner-guard-v1-linux-x86_64.json",
    "model-profiles/runtimes/docker-model-runner-v1.2.6-linux-x86_64.json",
    "package.json",
    "platforms/linux-inference/Cargo.toml",
    "platforms/linux-inference/README.md",
    "platforms/linux-inference/src/docker_guard.rs",
    "platforms/linux-inference/src/docker_preflight.rs",
    "platforms/linux-inference/src/docker_runtime.rs",
    "platforms/linux-inference/src/lib.rs",
    "scripts/build_contract.py",
    "scripts/linux_docker_preflight_evidence.py",
    "tests/test_linux_docker_preflight_evidence.py",
)
REFUSAL_CLASSES: Final = [
    "profile-identity",
    "observation-identity",
    "daemon-privilege",
    "socket-ownership",
    "api-binding",
    "container-reachability",
    "image-identity",
    "resource-limits",
    "zero-egress",
]
MUTATION_DIMENSIONS: Final = {
    "observation_identity": [
        "collector-executable",
        "session-identity",
        "completeness",
        "freshness",
        "replay",
    ],
    "daemon_privilege": [
        "daemon-executable",
        "daemon-uid",
        "rootless-mode",
        "runtime-uid",
        "runtime-user-docker-group-membership",
    ],
    "socket_ownership": [
        "socket-object-identity",
        "socket-type",
        "socket-owner-uid",
        "socket-group-gid",
        "socket-mode",
    ],
    "api_binding": [
        "private-namespace",
        "raw-host",
        "raw-port",
        "raw-listener-count",
        "host-listener-count",
        "non-loopback-listener-count",
        "management-listener-count",
    ],
    "container_reachability": [
        "runner-count",
        "guard-count",
        "shared-private-namespace",
        "guard-uid",
        "guard-executable",
        "guard-cgroup",
        "host-route-count",
        "bridge-route-count",
        "foreign-reachable-peer-count",
    ],
    "image_identity": [
        "runner-manifest",
        "model-manifest",
        "mutable-tag-admission",
        "runtime-repull",
    ],
    "resource_limits": [
        "workspace-mounts",
        "credential-mounts",
        "host-root-mounts",
        "docker-socket-mounts",
        "private-runtime-tmpfs",
        "model-content-store-write",
        "privileged",
        "capabilities-added",
        "no-new-privileges",
        "read-only-root",
        "memory",
        "tasks",
        "cpu",
        "runtime",
        "output",
        "swap",
        "parallel-slots",
    ],
    "zero_egress": [
        "do-not-track",
        "acquisition",
        "registry",
        "proxy",
        "dns",
        "firewall-default-deny",
        "egress-interface-count",
        "outbound-bytes",
    ],
}
LIMITATIONS: Final = [
    "The exact validator and mutation matrix executed, but no privileged topology collector is implemented or activated.",
    "Docker Engine and the Docker CLI are absent on this Fedora host; no daemon, socket, container, namespace, firewall, image, model, or packet path was inspected live.",
    "A successful synthetic exact observation proves deterministic contract behavior, not that a live Docker topology currently satisfies it.",
    "Clean live topology inspection, hostile reachability, independent control disablement, inference, and release support remain later Story 9.2 gates.",
]
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")


class LinuxDockerPreflightEvidenceError(ValueError):
    """Raised when Docker preflight evidence is stale, malformed, or overstated."""


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(prefix=".agentmage-docker-preflight-", dir=path.parent)
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


def run_command(arguments: list[str], timeout: int = 900) -> None:
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
        raise LinuxDockerPreflightEvidenceError(
            f"verification command failed: {Path(arguments[0]).name}"
        )


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
        raise LinuxDockerPreflightEvidenceError("source revision is unavailable")
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
        raise LinuxDockerPreflightEvidenceError(f"committed source absent: {relative}")
    return result.stdout


def source_records(revision: str) -> list[dict[str, Any]]:
    records = []
    for relative in SOURCE_PATHS:
        content = committed_file(revision, relative)
        current = ROOT / relative
        if not current.is_file() or current.read_bytes() != content:
            raise LinuxDockerPreflightEvidenceError(f"source differs from revision: {relative}")
        records.append(
            {"bytes": len(content), "path": relative, "sha256": sha256_bytes(content)}
        )
    return records


def host_identity() -> dict[str, Any]:
    values = {}
    for line in Path("/etc/os-release").read_text(encoding="utf-8").splitlines():
        if "=" in line:
            key, value = line.split("=", 1)
            values[key] = value.strip('"')
    return {
        "architecture": platform.machine(),
        "distribution": values.get("ID"),
        "version": values.get("VERSION_ID"),
        "docker_cli_available": shutil.which("docker") is not None,
        "live_collector_executed": False,
    }


def build_report(source_revision: str) -> dict[str, Any]:
    revision = git_revision(source_revision)
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
        "live_collector_executed": False,
    }:
        raise LinuxDockerPreflightEvidenceError(
            "this increment requires the declared Docker-absent Fedora host"
        )
    return {
        "artifact_id": "linux-docker-drift-preflight",
        "admission_contract": {
            "automatic_fallback": False,
            "complete_observation_required": True,
            "fresh_session_required": True,
            "permit_cloneable": False,
            "permit_serializable": False,
            "replay_allowed": False,
            "terminal_on_refusal": True,
        },
        "contract_version": 1,
        "docker_engine_directly_tested": False,
        "host": host,
        "limitations": LIMITATIONS,
        "live_topology_inspected": False,
        "mutation_dimensions": MUTATION_DIMENSIONS,
        "mutation_dimension_count": sum(len(value) for value in MUTATION_DIMENSIONS.values()),
        "refusal_classes": REFUSAL_CLASSES,
        "release_claim": "none",
        "schema_version": 1,
        "source_revision": revision,
        "sources": source_records(revision),
        "status": "pass-preflight-contract-no-live-docker",
        "task_ids": ["9.2.1.4"],
    }


def validate_report(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["Linux Docker preflight report must be an object"]
    failures = []
    if (
        value.get("schema_version") != 1
        or value.get("artifact_id") != "linux-docker-drift-preflight"
        or value.get("task_ids") != ["9.2.1.4"]
        or value.get("status") != "pass-preflight-contract-no-live-docker"
        or value.get("contract_version") != 1
        or REVISION.fullmatch(str(value.get("source_revision"))) is None
    ):
        failures.append("Linux Docker preflight report identity changed")
    if value.get("refusal_classes") != REFUSAL_CLASSES:
        failures.append("Docker preflight refusal closure changed")
    if (
        value.get("mutation_dimensions") != MUTATION_DIMENSIONS
        or value.get("mutation_dimension_count")
        != sum(len(item) for item in MUTATION_DIMENSIONS.values())
    ):
        failures.append("Docker preflight mutation closure changed")
    if value.get("admission_contract") != {
        "automatic_fallback": False,
        "complete_observation_required": True,
        "fresh_session_required": True,
        "permit_cloneable": False,
        "permit_serializable": False,
        "replay_allowed": False,
        "terminal_on_refusal": True,
    }:
        failures.append("Docker preflight admission contract broadened")
    host = value.get("host", {})
    if (
        host.get("architecture") != "x86_64"
        or host.get("distribution") != "fedora"
        or host.get("version") != "44"
        or host.get("docker_cli_available") is not False
        or host.get("live_collector_executed") is not False
    ):
        failures.append("Docker-absent preflight host evidence changed")
    if (
        value.get("docker_engine_directly_tested") is not False
        or value.get("live_topology_inspected") is not False
        or value.get("release_claim") != "none"
        or value.get("limitations") != LIMITATIONS
    ):
        failures.append("Docker preflight report made a live or release overclaim")
    sources = value.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(
        SOURCE_PATHS
    ):
        failures.append("Docker preflight source closure changed")
    elif any(
        not isinstance(item.get("bytes"), int)
        or item["bytes"] <= 0
        or SHA256.fullmatch(str(item.get("sha256"))) is None
        for item in sources
    ):
        failures.append("Docker preflight source identity is invalid")
    return failures


def check_report() -> list[str]:
    if not REPORT_PATH.is_file():
        return ["Linux Docker preflight report is missing"]
    value = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    failures = validate_report(value)
    if failures:
        return failures
    revision = value["source_revision"]
    try:
        expected_sources = source_records(revision)
    except LinuxDockerPreflightEvidenceError as error:
        return [str(error)]
    if value["sources"] != expected_sources:
        return ["Linux Docker preflight source evidence is stale"]
    return []


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    try:
        if arguments.write:
            report = build_report(arguments.source_revision)
            failures = validate_report(report)
            if failures:
                raise LinuxDockerPreflightEvidenceError("; ".join(failures))
            write_atomic(REPORT_PATH, canonical_json(report))
        failures = check_report()
        if failures:
            raise LinuxDockerPreflightEvidenceError("; ".join(failures))
    except (LinuxDockerPreflightEvidenceError, OSError, ValueError) as error:
        print(f"Linux Docker preflight evidence failed: {error}", file=sys.stderr)
        return 1
    print("Linux Docker fail-closed drift preflight evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
