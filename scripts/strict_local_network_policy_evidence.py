#!/usr/bin/env python3
"""Build and validate strict-local normal-operation network policy evidence."""

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
    from scripts.linux_docker_runtime_evidence import validate_report as validate_docker
    from scripts.linux_native_runtime_evidence import validate_report as validate_native
except ModuleNotFoundError:
    from linux_docker_runtime_evidence import validate_report as validate_docker
    from linux_native_runtime_evidence import validate_report as validate_native


ROOT: Final = Path(__file__).resolve().parents[1]
REPORT_PATH: Final = (
    ROOT / "artifacts/sprints/sprint-10/story-10.1/strict-local-network-policy.json"
)
SOURCE_PATHS: Final = (
    "docs/architecture/strict-local-boundary.md",
    "kernel/contracts/src/network.rs",
    "kernel/engine/src/strict_local.rs",
    "platforms/linux/src/sandbox.rs",
    "security/strict-local-source-policy.json",
    "scripts/strict_local_network_policy_evidence.py",
    "scripts/strict_local_source_audit.py",
    "tests/test_strict_local_network_policy_evidence.py",
    "tests/test_strict_local_source_audit.py",
)
COMPONENTS: Final = (
    "visual_studio_code_extension",
    "native_bridge",
    "kernel",
    "kernel_native_inference_adapter",
    "kernel_docker_inference_adapter",
    "tool_worker",
    "converter",
    "indexer",
    "model_installer",
    "undeclared",
)
ENDPOINTS: Final = (
    (
        "native",
        "kernel_native_inference_adapter",
        "authenticated_unix_socket",
        "authenticated_local_socket",
    ),
    (
        "docker",
        "kernel_docker_inference_adapter",
        "guarded_loopback_tcp",
        "loopback",
    ),
)
DENIED_DESTINATIONS: Final = (
    "local_socket",
    "unspecified",
    "link_local",
    "private_lan",
    "multicast",
    "container_network",
    "proxy",
    "dns",
    "external",
    "unknown",
)
COMMAND_SPECS: Final = (
    (
        (
            "cargo",
            "test",
            "-p",
            "agentmage-kernel-engine",
            "strict_local::tests",
            "--locked",
        ),
        "test result: ok. 14 passed",
    ),
    (
        ("python3", "-m", "unittest", "tests.test_strict_local_source_audit"),
        "Ran 11 tests",
    ),
    (
        ("python3", "scripts/strict_local_source_audit.py"),
        "Strict-local source audit passed with zero undeclared network paths.",
    ),
    (
        (
            "cargo",
            "test",
            "-p",
            "agentmage-platform-linux",
            "worker_receives_only_the_fixed_environment_and_no_network",
            "--locked",
            "--",
            "--ignored",
        ),
        "test result: ok. 1 passed",
    ),
)
LIMITATIONS: Final = (
    "This proves the deterministic component policy, closed product-source boundary, dependency exclusion, and one live Linux worker network-isolation test.",
    "Host, kernel, converter, indexer, and inference process-level confinement remains part of later session-topology and packet-capture tasks.",
    "The 60-minute integrated workflow, acquisition-return, physical-host, macOS, Windows, and release gates remain open.",
)
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class StrictLocalNetworkEvidenceError(ValueError):
    """Raised when strict-local policy evidence is incomplete or overstated."""


def run_checked(arguments: list[str], marker: str) -> dict[str, Any]:
    completed = subprocess.run(
        arguments,
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
        raise StrictLocalNetworkEvidenceError(
            f"verification command failed: {Path(arguments[0]).name}"
        )
    return {
        "command_id": hashlib.sha256("\0".join(arguments).encode()).hexdigest(),
        "exit_code": 0,
        "expected_marker_sha256": hashlib.sha256(marker.encode()).hexdigest(),
        "status": "pass",
    }


def expected_command_records() -> list[dict[str, Any]]:
    return [
        {
            "command_id": hashlib.sha256("\0".join(arguments).encode()).hexdigest(),
            "exit_code": 0,
            "expected_marker_sha256": hashlib.sha256(marker.encode()).hexdigest(),
            "status": "pass",
        }
        for arguments, marker in COMMAND_SPECS
    ]


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
        raise StrictLocalNetworkEvidenceError("source revision is unavailable")
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
        raise StrictLocalNetworkEvidenceError("committed source is unavailable")
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
        raise StrictLocalNetworkEvidenceError("runtime evidence is unavailable") from error
    if not isinstance(value, dict):
        raise StrictLocalNetworkEvidenceError("runtime evidence is invalid")
    return value


def runtime_records() -> list[dict[str, Any]]:
    specifications = (
        (
            "native",
            ROOT
            / "artifacts/sprints/sprint-9/story-9.2/linux-native-runtime-package.json",
            validate_native,
        ),
        (
            "docker",
            ROOT
            / "artifacts/sprints/sprint-9/story-9.2/linux-docker-runtime-profile.json",
            validate_docker,
        ),
    )
    records = []
    for runtime_id, path, validator in specifications:
        value = read_json(path)
        failures = validator(value)
        if failures:
            raise StrictLocalNetworkEvidenceError(
                f"runtime evidence failed: {runtime_id}"
            )
        records.append(
            {
                "runtime_id": runtime_id,
                "path": path.relative_to(ROOT).as_posix(),
                "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                "source_revision": value["source_revision"],
                "enabled_models": value["enabled_models"],
                "inference_started": value["inference_started"],
                "release_claim": value["release_claim"],
            }
        )
    return records


def policy_matrix() -> list[dict[str, Any]]:
    return [
        {
            "topology": topology,
            "selected_client": selected,
            "transport": transport,
            "destination": destination,
            "components": [
                {
                    "component": component,
                    "decision": "allow-exact-authenticated-local-only"
                    if component == selected
                    else "deny-client-not-authorized",
                }
                for component in COMPONENTS
            ],
            "denied_destination_classes": list(DENIED_DESTINATIONS),
            "cross_topology_substitution": "denied",
        }
        for topology, selected, transport, destination in ENDPOINTS
    ]


def build_report(revision: str) -> dict[str, Any]:
    commands = [run_checked(list(arguments), marker) for arguments, marker in COMMAND_SPECS]
    return {
        "schema_version": 1,
        "artifact_id": "strict-local-normal-operation-network-policy",
        "source_revision": revision,
        "task_ids": ["10.1.1.1"],
        "status": "pass-policy-source-worker-no-live-packet-capture",
        "policy_matrix": policy_matrix(),
        "source_audit": {
            "scan_root_count": 8,
            "linux_inference_included": True,
            "undeclared_network_paths": 0,
            "network_capable_runtime_dependencies": 0,
        },
        "verification_commands": commands,
        "runtime_evidence": runtime_records(),
        "claims": {
            "normal_operation_policy_complete": True,
            "linux_worker_network_isolation_tested": True,
            "live_host_process_confinement_tested": False,
            "packet_capture_performed": False,
            "inference_performed": False,
            "release_support": False,
        },
        "limitations": list(LIMITATIONS),
        "private_user_data_used": False,
        "network_used": False,
        "sources": source_records(revision),
    }


def validate_report(value: Any) -> list[str]:
    failures: list[str] = []
    if (
        not isinstance(value, dict)
        or value.get("schema_version") != 1
        or value.get("artifact_id")
        != "strict-local-normal-operation-network-policy"
        or value.get("task_ids") != ["10.1.1.1"]
        or value.get("status")
        != "pass-policy-source-worker-no-live-packet-capture"
        or REVISION.fullmatch(str(value.get("source_revision"))) is None
    ):
        return ["strict-local network evidence identity changed"]
    if value.get("policy_matrix") != policy_matrix():
        failures.append("strict-local component policy matrix changed")
    if value.get("source_audit") != {
        "scan_root_count": 8,
        "linux_inference_included": True,
        "undeclared_network_paths": 0,
        "network_capable_runtime_dependencies": 0,
    }:
        failures.append("strict-local source closure changed")
    if value.get("verification_commands") != expected_command_records():
        failures.append("strict-local verification command closure changed")
    runtimes = value.get("runtime_evidence")
    if (
        not isinstance(runtimes, list)
        or [item.get("runtime_id") for item in runtimes] != ["native", "docker"]
        or any(
            item.get("enabled_models") != 0
            or item.get("inference_started") is not False
            or item.get("release_claim") != "none"
            or SHA256.fullmatch(str(item.get("sha256"))) is None
            or REVISION.fullmatch(str(item.get("source_revision"))) is None
            for item in runtimes
        )
    ):
        failures.append("strict-local runtime evidence changed")
    if value.get("claims") != {
        "normal_operation_policy_complete": True,
        "linux_worker_network_isolation_tested": True,
        "live_host_process_confinement_tested": False,
        "packet_capture_performed": False,
        "inference_performed": False,
        "release_support": False,
    }:
        failures.append("strict-local network evidence overclaimed")
    if value.get("limitations") != list(LIMITATIONS):
        failures.append("strict-local network evidence limitations changed")
    if (
        value.get("private_user_data_used") is not False
        or value.get("network_used") is not False
    ):
        failures.append("strict-local network evidence used prohibited data or network")
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
        failures.append("strict-local source evidence changed")
    return failures


def write_atomic(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(prefix=".agentmage-strict-local-network-", dir=path.parent)
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
            failures.append("strict-local source evidence is stale")
        if failures:
            raise StrictLocalNetworkEvidenceError("; ".join(failures))
    except (
        StrictLocalNetworkEvidenceError,
        OSError,
        UnicodeError,
        ValueError,
        subprocess.SubprocessError,
    ) as error:
        print(f"Strict-local network evidence failed: {error}", file=sys.stderr)
        return 1
    print("Strict-local normal-operation network policy evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
