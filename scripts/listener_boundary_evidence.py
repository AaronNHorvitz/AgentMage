#!/usr/bin/env python3
"""Build and validate strict-local listener-boundary evidence."""

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

try:
    from scripts.linux_docker_kvm_evidence import validate_report as validate_topology
    from scripts.linux_docker_reachability_evidence import validate as validate_reachability
except ModuleNotFoundError:
    from linux_docker_kvm_evidence import validate_report as validate_topology
    from linux_docker_reachability_evidence import validate as validate_reachability


ROOT: Final = Path(__file__).resolve().parents[1]
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-10/story-10.1/listener-boundary.json"
TOPOLOGY_PATH: Final = (
    ROOT / "artifacts/sprints/sprint-9/story-9.2/linux-docker-kvm-topology.json"
)
REACHABILITY_PATH: Final = (
    ROOT / "artifacts/sprints/sprint-9/story-9.2/linux-docker-reachability.json"
)
SOURCE_PATHS: Final = (
    "docs/architecture/strict-local-boundary.md",
    "platforms/linux/src/inventory.rs",
    "platforms/linux/src/lib.rs",
    "scripts/listener_boundary_evidence.py",
    "tests/test_listener_boundary_evidence.py",
)
COMMAND_SPECS: Final = (
    (
        (
            "cargo",
            "test",
            "-p",
            "agentmage-platform-linux",
            "strict_local_listener",
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
            "strict_local_listener_live_loopback_passes_and_wildcard_fails",
            "--locked",
            "--",
            "--ignored",
        ),
        "test result: ok. 1 passed; 0 failed; 0 ignored",
    ),
    (
        ("python3", "scripts/strict_local_source_audit.py"),
        "Strict-local source audit passed with zero undeclared network paths.",
    ),
)
POLICY_PROFILE: Final = {
    "maximum_declarations": 32,
    "allowed_components": [
        "native_bridge",
        "kernel",
        "kernel_native_inference_adapter",
        "kernel_docker_inference_adapter",
    ],
    "allowed_listener_forms": [
        "exact-named-unix-stream-endpoint-digest",
        "exact-loopback-tcp-protocol-and-port",
    ],
    "denied_destination_classes": [
        "authenticated_local_socket_observation",
        "unspecified",
        "link_local",
        "private_lan",
        "multicast",
        "container_network",
        "proxy",
        "dns",
        "external",
        "unknown",
    ],
    "denied_socket_states": ["bound", "indeterminate"],
    "exactness_failures": [
        "invalid-declaration",
        "resource-limit",
        "duplicate-declaration",
        "unknown-process",
        "unsafe-destination",
        "undeclared-binding",
        "indeterminate-socket-state",
        "undeclared-listener",
        "missing-declared-listener",
        "duplicate-observed-listener",
    ],
}
LIMITATIONS: Final = [
    "The live host test proves point-in-time procfs reconciliation for exact loopback acceptance and wildcard refusal; it is not continuous packet or syscall tracing.",
    "Complete session-manifest binding of process executable and namespace identities remains assigned to later Sprint 10 inventory and acceptance tasks.",
    "Native inference remains inactive, and this artifact does not perform inference, physical-host cross-distribution certification, macOS verification, or release acceptance.",
]
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class ListenerBoundaryEvidenceError(ValueError):
    """Raised when listener evidence is incomplete or overstated."""


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
        raise ListenerBoundaryEvidenceError("source revision is unavailable")
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
        raise ListenerBoundaryEvidenceError("committed source is unavailable")
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


def read_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise ListenerBoundaryEvidenceError("prerequisite evidence is unavailable") from error
    if not isinstance(value, dict):
        raise ListenerBoundaryEvidenceError("prerequisite evidence is invalid")
    return value


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
        raise ListenerBoundaryEvidenceError(
            f"verification command failed: {Path(arguments[0]).name}"
        )
    return command_record(arguments, marker)


def prerequisite_records() -> dict[str, Any]:
    topology = read_json(TOPOLOGY_PATH)
    reachability = read_json(REACHABILITY_PATH)
    if validate_topology(topology):
        raise ListenerBoundaryEvidenceError("Docker topology evidence failed")
    if validate_reachability(reachability):
        raise ListenerBoundaryEvidenceError("Docker reachability evidence failed")
    topology_targets = []
    for target in topology["targets"]:
        observation = target["observation"]
        api = observation["docker"]["collector"]["observation"]["api"]
        topology_targets.append(
            {
                "target_id": target["target_id"],
                "host_listener_count": api["host_listener_count"],
                "private_raw_listener_count": api["raw_listener_count"],
                "private_non_loopback_listener_count": api[
                    "non_loopback_listener_count"
                ],
                "private_active_interface_count": api[
                    "namespace_active_interface_count"
                ],
                "private_non_local_route_count": api[
                    "namespace_non_local_route_count"
                ],
                "cleanup_complete": all(observation["cleanup"].values())
                and all(target["host_cleanup"].values()),
            }
        )
    reachability_targets = []
    for target in reachability["targets"]:
        positions = [
            probe
            for probe in target["observation"]["probes"]
            if probe["position"] in {"host", "lan", "ordinary-container"}
        ]
        reachability_targets.append(
            {"target_id": target["target_id"], "positions": positions}
        )
    return {
        "docker_topology": {
            "path": TOPOLOGY_PATH.relative_to(ROOT).as_posix(),
            "sha256": hashlib.sha256(TOPOLOGY_PATH.read_bytes()).hexdigest(),
            "source_revision": topology["source_revision"],
            "targets": topology_targets,
        },
        "docker_reachability": {
            "path": REACHABILITY_PATH.relative_to(ROOT).as_posix(),
            "sha256": hashlib.sha256(REACHABILITY_PATH.read_bytes()).hexdigest(),
            "source_revision": reachability["source_revision"],
            "targets": reachability_targets,
        },
    }


def build_report(revision: str) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "artifact_id": "strict-local-listener-boundary",
        "source_revision": revision,
        "task_ids": ["10.1.1.3"],
        "status": "pass-linux-listener-reconciliation-and-docker-kvm-boundary",
        "policy_profile": POLICY_PROFILE,
        "verification_commands": [
            run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS
        ],
        "prerequisite_evidence": prerequisite_records(),
        "claims": {
            "deterministic_listener_reconciliation_complete": True,
            "live_linux_loopback_and_wildcard_tested": True,
            "docker_private_listener_topology_tested": True,
            "continuous_process_confinement_tested": False,
            "packet_capture_performed": False,
            "inference_performed": False,
            "release_support": False,
        },
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "external_network_used": False,
        "local_test_sockets_used": True,
        "sources": source_records(revision),
    }


def validate_report(value: Any) -> list[str]:
    failures: list[str] = []
    if (
        not isinstance(value, dict)
        or value.get("schema_version") != 1
        or value.get("artifact_id") != "strict-local-listener-boundary"
        or value.get("task_ids") != ["10.1.1.3"]
        or value.get("status")
        != "pass-linux-listener-reconciliation-and-docker-kvm-boundary"
        or REVISION.fullmatch(str(value.get("source_revision"))) is None
    ):
        return ["listener boundary evidence identity changed"]
    if value.get("policy_profile") != POLICY_PROFILE:
        failures.append("listener policy profile changed")
    if value.get("verification_commands") != expected_commands():
        failures.append("listener verification commands changed")
    evidence = value.get("prerequisite_evidence")
    topology = evidence.get("docker_topology", {}) if isinstance(evidence, dict) else {}
    reachability = evidence.get("docker_reachability", {}) if isinstance(evidence, dict) else {}
    topology_targets = topology.get("targets")
    if (
        topology.get("path") != TOPOLOGY_PATH.relative_to(ROOT).as_posix()
        or SHA256.fullmatch(str(topology.get("sha256"))) is None
        or REVISION.fullmatch(str(topology.get("source_revision"))) is None
        or not isinstance(topology_targets, list)
        or [target.get("target_id") for target in topology_targets]
        != ["fedora-44-x86_64", "ubuntu-26.04-x86_64"]
        or any(
            target.get("host_listener_count") != 0
            or target.get("private_raw_listener_count") != 1
            or target.get("private_non_loopback_listener_count") != 0
            or target.get("private_active_interface_count") != 1
            or target.get("private_non_local_route_count") != 0
            or target.get("cleanup_complete") is not True
            for target in topology_targets or []
        )
    ):
        failures.append("Docker listener topology evidence changed")
    reachability_targets = reachability.get("targets")
    if (
        reachability.get("path") != REACHABILITY_PATH.relative_to(ROOT).as_posix()
        or SHA256.fullmatch(str(reachability.get("sha256"))) is None
        or REVISION.fullmatch(str(reachability.get("source_revision"))) is None
        or not isinstance(reachability_targets, list)
        or [target.get("target_id") for target in reachability_targets]
        != ["fedora-44-x86_64", "ubuntu-26.04-x86_64"]
        or any(
            [item.get("position") for item in target.get("positions", [])]
            != ["host", "lan", "ordinary-container"]
            or any(
                item.get("status") != "denied"
                or item.get("raw_tcp_connected") is not False
                for item in target.get("positions", [])
            )
            for target in reachability_targets or []
        )
    ):
        failures.append("Docker listener reachability evidence changed")
    if value.get("claims") != {
        "deterministic_listener_reconciliation_complete": True,
        "live_linux_loopback_and_wildcard_tested": True,
        "docker_private_listener_topology_tested": True,
        "continuous_process_confinement_tested": False,
        "packet_capture_performed": False,
        "inference_performed": False,
        "release_support": False,
    }:
        failures.append("listener boundary evidence overclaimed")
    if value.get("limitations") != LIMITATIONS:
        failures.append("listener boundary limitations changed")
    if (
        value.get("private_user_data_used") is not False
        or value.get("external_network_used") is not False
        or value.get("local_test_sockets_used") is not True
    ):
        failures.append("listener boundary execution classification changed")
    sources = value.get("sources")
    if (
        not isinstance(sources, list)
        or [item.get("path") for item in sources] != list(SOURCE_PATHS)
        or any(
            not isinstance(item.get("bytes"), int)
            or item.get("bytes", 0) <= 0
            or SHA256.fullmatch(str(item.get("sha256"))) is None
            for item in sources
        )
    ):
        failures.append("listener boundary source evidence changed")
    return failures


def write_atomic(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(prefix=".agentmage-listener-boundary-", dir=path.parent)
    temporary = Path(name)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as handle:
            json.dump(value, handle, indent=2, sort_keys=True)
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        temporary.chmod(0o644)
        os.replace(temporary, path)
    except BaseException:
        temporary.unlink(missing_ok=True)
        raise


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    try:
        if arguments.write:
            write_atomic(REPORT_PATH, build_report(git_revision(arguments.source_revision)))
        report = read_json(REPORT_PATH)
        failures = validate_report(report)
        if not failures and report["sources"] != source_records(report["source_revision"]):
            failures.append("listener boundary source evidence is stale")
        if failures:
            raise ListenerBoundaryEvidenceError("; ".join(failures))
    except (
        ListenerBoundaryEvidenceError,
        OSError,
        UnicodeError,
        ValueError,
        subprocess.SubprocessError,
    ) as error:
        print(f"Listener boundary evidence failed: {error}", file=sys.stderr)
        return 1
    print("Strict-local listener boundary evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
