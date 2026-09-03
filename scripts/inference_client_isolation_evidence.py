#!/usr/bin/env python3
"""Build and validate direct inference-client isolation evidence."""

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
    from scripts.linux_docker_reachability_evidence import validate as validate_reachability
    from scripts.linux_native_runtime_evidence import validate_report as validate_native
except ModuleNotFoundError:
    from linux_docker_reachability_evidence import validate as validate_reachability
    from linux_native_runtime_evidence import validate_report as validate_native


ROOT: Final = Path(__file__).resolve().parents[1]
REPORT_PATH: Final = (
    ROOT / "artifacts/sprints/sprint-10/story-10.1/inference-client-isolation.json"
)
REACHABILITY_PATH: Final = (
    ROOT / "artifacts/sprints/sprint-9/story-9.2/linux-docker-reachability.json"
)
NATIVE_PATH: Final = (
    ROOT / "artifacts/sprints/sprint-9/story-9.2/linux-native-runtime-package.json"
)
SOURCE_PATHS: Final = (
    "platforms/linux/src/sandbox.rs",
    "scripts/inference_client_isolation_evidence.py",
    "scripts/strict_local_source_audit.py",
    "security/strict-local-source-policy.json",
    "shells/vscode/src/host_bootstrap.ts",
    "shells/vscode/src/host_bridge.ts",
    "tests/test_inference_client_isolation_evidence.py",
    "tests/test_strict_local_source_audit.py",
)
COMMAND_SPECS: Final = (
    (
        ("python3", "-m", "unittest", "tests.test_strict_local_source_audit"),
        "Ran 11 tests",
    ),
    (
        ("python3", "scripts/strict_local_source_audit.py"),
        "Strict-local source audit passed with zero undeclared network paths.",
    ),
    (("npm", "--prefix", "shells/vscode", "test"), "pass 86"),
    (
        (
            "cargo",
            "test",
            "-p",
            "agentmage-platform-linux",
            "policy_denies_every_worker_socket_and_connection_syscall",
            "--locked",
        ),
        "test result: ok. 1 passed",
    ),
    (
        (
            "cargo",
            "test",
            "-p",
            "agentmage-platform-linux",
            "tool_worker_cannot_connect_to_raw_inference_loopback",
            "--locked",
            "--",
            "--ignored",
        ),
        "test result: ok. 1 passed",
    ),
)
CLIENT_BOUNDARIES: Final = [
    {
        "client": "agentmage-vscode-extension",
        "admitted_transport": "authenticated-package-bootstrap-host-unix-socket-only",
        "direct_raw_runtime_client": False,
        "controls": [
            "one-exact-node-net-import",
            "one-exact-unix-path-connection-call",
            "package-bootstrap-endpoint-only",
            "raw-loopback-substitution-mutation-denied",
        ],
    },
    {
        "client": "agentmage-tool-worker",
        "admitted_transport": "none",
        "direct_raw_runtime_client": False,
        "controls": [
            "fresh-private-network-namespace",
            "network-syscall-seccomp-denial",
            "fixed-environment-without-endpoint",
            "live-raw-loopback-connect-denial",
        ],
    },
]
LIMITATIONS: Final = [
    "The native runtime remains inactive with zero enabled models, so enabled native inference reachability must be rerun in Sprint 13.",
    "The Visual Studio Code result constrains the shipped AgentMage extension source and Docker hostile position; it is not confinement of every unrelated extension or same-user process.",
    "This artifact does not perform inference, packet capture, a 60-minute workflow, physical-host certification, or release acceptance.",
]
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class InferenceClientIsolationError(ValueError):
    """Raised when direct-client isolation evidence is incomplete or overstated."""


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
        raise InferenceClientIsolationError("source revision is unavailable")
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
        raise InferenceClientIsolationError("committed source is unavailable")
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
        raise InferenceClientIsolationError("prerequisite evidence is unavailable") from error
    if not isinstance(value, dict):
        raise InferenceClientIsolationError("prerequisite evidence is invalid")
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
        raise InferenceClientIsolationError(
            f"verification command failed: {Path(arguments[0]).name}"
        )
    return command_record(arguments, marker)


def prerequisite_records() -> dict[str, Any]:
    reachability = read_json(REACHABILITY_PATH)
    native = read_json(NATIVE_PATH)
    if validate_reachability(reachability):
        raise InferenceClientIsolationError("Docker reachability evidence failed")
    if validate_native(native):
        raise InferenceClientIsolationError("native runtime evidence failed")
    targets = []
    for target in reachability["targets"]:
        selected = [
            probe
            for probe in target["observation"]["probes"]
            if probe["position"] in {"vscode-extension", "tool-worker"}
        ]
        targets.append(
            {
                "target_id": target["target_id"],
                "positions": selected,
                "authenticated_guard_control": True,
                "inference_request_sent": False,
                "cleanup_complete": all(target["observation"]["cleanup"].values())
                and all(target["host_cleanup"].values()),
            }
        )
    return {
        "docker": {
            "path": REACHABILITY_PATH.relative_to(ROOT).as_posix(),
            "sha256": hashlib.sha256(REACHABILITY_PATH.read_bytes()).hexdigest(),
            "source_revision": reachability["source_revision"],
            "targets": targets,
        },
        "native": {
            "path": NATIVE_PATH.relative_to(ROOT).as_posix(),
            "sha256": hashlib.sha256(NATIVE_PATH.read_bytes()).hexdigest(),
            "source_revision": native["source_revision"],
            "enabled_models": native["enabled_models"],
            "inference_started": native["inference_started"],
            "release_claim": native["release_claim"],
        },
    }


def build_report(revision: str) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "artifact_id": "inference-client-isolation",
        "source_revision": revision,
        "task_ids": ["10.1.1.2"],
        "status": "pass-shipped-vscode-and-linux-worker-no-enabled-native-inference",
        "client_boundaries": CLIENT_BOUNDARIES,
        "verification_commands": [
            run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS
        ],
        "prerequisite_evidence": prerequisite_records(),
        "claims": {
            "shipped_vscode_direct_raw_client_absent": True,
            "tool_worker_direct_raw_client_denied": True,
            "docker_vscode_and_worker_positions_denied": True,
            "enabled_native_runtime_tested": False,
            "arbitrary_same_user_process_confinement_tested": False,
            "inference_performed": False,
            "release_support": False,
        },
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "network_used": False,
        "sources": source_records(revision),
    }


def validate_report(value: Any) -> list[str]:
    failures: list[str] = []
    if (
        not isinstance(value, dict)
        or value.get("schema_version") != 1
        or value.get("artifact_id") != "inference-client-isolation"
        or value.get("task_ids") != ["10.1.1.2"]
        or value.get("status")
        != "pass-shipped-vscode-and-linux-worker-no-enabled-native-inference"
        or REVISION.fullmatch(str(value.get("source_revision"))) is None
    ):
        return ["inference client isolation identity changed"]
    if value.get("client_boundaries") != CLIENT_BOUNDARIES:
        failures.append("inference client boundary matrix changed")
    if value.get("verification_commands") != expected_commands():
        failures.append("inference client verification commands changed")
    evidence = value.get("prerequisite_evidence")
    docker = evidence.get("docker", {}) if isinstance(evidence, dict) else {}
    native = evidence.get("native", {}) if isinstance(evidence, dict) else {}
    targets = docker.get("targets")
    if (
        docker.get("path") != REACHABILITY_PATH.relative_to(ROOT).as_posix()
        or SHA256.fullmatch(str(docker.get("sha256"))) is None
        or REVISION.fullmatch(str(docker.get("source_revision"))) is None
        or not isinstance(targets, list)
        or [target.get("target_id") for target in targets]
        != ["fedora-44-x86_64", "ubuntu-26.04-x86_64"]
        or any(
            [item.get("position") for item in target.get("positions", [])]
            != ["vscode-extension", "tool-worker"]
            or any(
                item.get("status") != "denied"
                or item.get("raw_tcp_connected") is not False
                for item in target.get("positions", [])
            )
            or target.get("authenticated_guard_control") is not True
            or target.get("inference_request_sent") is not False
            or target.get("cleanup_complete") is not True
            for target in targets or []
        )
    ):
        failures.append("Docker direct-client isolation evidence changed")
    if (
        native.get("path") != NATIVE_PATH.relative_to(ROOT).as_posix()
        or SHA256.fullmatch(str(native.get("sha256"))) is None
        or REVISION.fullmatch(str(native.get("source_revision"))) is None
        or native.get("enabled_models") != 0
        or native.get("inference_started") is not False
        or native.get("release_claim") != "none"
    ):
        failures.append("native inactive-runtime evidence changed")
    if value.get("claims") != {
        "shipped_vscode_direct_raw_client_absent": True,
        "tool_worker_direct_raw_client_denied": True,
        "docker_vscode_and_worker_positions_denied": True,
        "enabled_native_runtime_tested": False,
        "arbitrary_same_user_process_confinement_tested": False,
        "inference_performed": False,
        "release_support": False,
    }:
        failures.append("inference client isolation evidence overclaimed")
    if value.get("limitations") != LIMITATIONS:
        failures.append("inference client isolation limitations changed")
    if value.get("private_user_data_used") is not False or value.get("network_used") is not False:
        failures.append("inference client evidence used prohibited data or network")
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
        failures.append("inference client source evidence changed")
    return failures


def write_atomic(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(prefix=".agentmage-client-isolation-", dir=path.parent)
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
            failures.append("inference client source evidence is stale")
        if failures:
            raise InferenceClientIsolationError("; ".join(failures))
    except (
        InferenceClientIsolationError,
        OSError,
        UnicodeError,
        ValueError,
        subprocess.SubprocessError,
    ) as error:
        print(f"Inference client isolation evidence failed: {error}", file=sys.stderr)
        return 1
    print("Inference client isolation evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
