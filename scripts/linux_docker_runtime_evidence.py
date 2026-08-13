#!/usr/bin/env python3
"""Build and validate the pinned, inactive Linux Docker compatibility evidence."""

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
    ROOT
    / "model-profiles/runtimes/docker-model-runner-v1.2.6-linux-x86_64.json"
)
REPORT_PATH: Final = (
    ROOT
    / "artifacts/sprints/sprint-9/story-9.2/linux-docker-runtime-profile.json"
)
PROFILE_SHA256: Final = (
    "ab8cde6bc1440f8a0013390aa2e291a315cdebcfe44aa1d339f0aa0b1d70899c"
)
RUNNER_DIGEST: Final = (
    "sha256:bd94095bbc1ddc4266c3a88f582a92562c6b63eceb175572c9a60045663727c9"
)
MODEL_DIGEST: Final = (
    "sha256:08fa7b1d44f255be48cfc12359211725bfd659742612ed4b221cd5be90d14444"
)
SOURCE_PATHS: Final = (
    "Cargo.lock",
    "Cargo.toml",
    "RUNTIME-BOUNDARIES.md",
    "architecture/build-contract.json",
    "architecture/component-inventory-policy.json",
    "docs/decisions/0028-isolated-linux-inference-package-boundary.md",
    "docs/decisions/0029-closed-linux-native-llama-runtime-package.md",
    "docs/decisions/0030-closed-linux-docker-model-runner-compatibility-profile.md",
    "docs/decisions/0031-private-docker-model-runner-endpoint-guard.md",
    "model-profiles/candidates/gemma-4-e4b/artifact-admission.json",
    "model-profiles/runtimes/docker-model-runner-v1.2.6-linux-x86_64.json",
    "model-profiles/runtimes/docker-model-runner-guard-v1-linux-x86_64.json",
    "package.json",
    "platforms/linux-inference/Cargo.toml",
    "platforms/linux-inference/README.md",
    "platforms/linux-inference/src/docker_runtime.rs",
    "platforms/linux-inference/src/docker_guard.rs",
    "platforms/linux-inference/src/lib.rs",
    "platforms/linux-inference/src/main.rs",
    "platforms/linux-inference/tests/process_boundary.rs",
    "scripts/build_contract.py",
    "scripts/component_inventory.py",
    "scripts/linux_docker_runtime_evidence.py",
    "tests/test_component_inventory.py",
    "tests/test_linux_docker_runtime_evidence.py",
)
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
EXPECTED_DESCRIPTOR: Final = {
    "accepted_operation": "self-check-only",
    "authority_inputs": [],
    "component_id": "platform-linux-native-inference",
    "contract": "authenticated-local-endpoint-v1",
    "docker_compatibility_available": False,
    "docker_guard_profile_sha256": "88fb0d5a78829cbdfc34af5cbcbfe3ca2a80f550889947e6478fdb66bf7edb2a",
    "docker_model_artifact_digest": MODEL_DIGEST,
    "docker_model_runner_image_digest": RUNNER_DIGEST,
    "docker_runtime_profile_sha256": PROFILE_SHA256,
    "enabled_models": 0,
    "inference_available": False,
    "native_runtime_package": "agentmage-llama-cpp-b10333-cpu-linux-x86_64",
    "native_runtime_profile_sha256": "21346c06fb86b418706326b186609f8e1f690d6b57b53e73d02b4ad8e28e53ea",
    "network_listener": False,
    "process_boundary_version": 4,
}
LIMITATIONS: Final = [
    "Docker Engine and docker-model-plugin are absent on this Fedora host; no Docker daemon, socket, container, or API was started.",
    "Prior exact-image execution under rootless Podman is retained only as quarantined compatibility evidence and is not Docker Engine verification.",
    "The pinned Gemma 4 E4B OCI artifact remains blocked by source-lineage, reproducibility, and quality gates; it is not enabled or loaded.",
    "Raw loopback endpoint isolation, drift preflight, clean Fedora/Ubuntu Docker Engine execution, inference, and release support remain later sub-tasks.",
]


class LinuxDockerRuntimeEvidenceError(ValueError):
    """Raised when Docker compatibility evidence is missing, stale, or overstated."""


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(prefix=".agentmage-linux-docker-", dir=path.parent)
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
        raise LinuxDockerRuntimeEvidenceError(
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
        raise LinuxDockerRuntimeEvidenceError("source revision is unavailable")
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
        raise LinuxDockerRuntimeEvidenceError(f"committed source absent: {relative}")
    return result.stdout


def source_records(revision: str) -> list[dict[str, Any]]:
    records = []
    for relative in SOURCE_PATHS:
        committed = committed_file(revision, relative)
        current = ROOT / relative
        if not current.is_file() or current.read_bytes() != committed:
            raise LinuxDockerRuntimeEvidenceError(
                f"source differs from revision: {relative}"
            )
        records.append(
            {"bytes": len(committed), "path": relative, "sha256": sha256_bytes(committed)}
        )
    return records


def load_profile() -> dict[str, Any]:
    profile = json.loads(PROFILE_PATH.read_text(encoding="utf-8"))
    failures = validate_profile(profile)
    if sha256_file(PROFILE_PATH) != PROFILE_SHA256:
        failures.append("Docker compatibility profile SHA-256 changed")
    if failures:
        raise LinuxDockerRuntimeEvidenceError("; ".join(failures))
    return profile


def validate_profile(profile: Any) -> list[str]:
    if not isinstance(profile, dict):
        return ["Docker compatibility profile must be an object"]
    failures = []
    if (
        profile.get("schema_version") != 1
        or profile.get("record_type")
        != "linux_docker_model_runner_compatibility_profile"
        or profile.get("platform") != "linux"
        or profile.get("architecture") != "x86_64"
    ):
        failures.append("Docker compatibility profile identity changed")
    if profile.get("package") != {
        "install_authority": "separate-administrator-operation",
        "package_id": "docker-model-plugin",
        "package_version": "1.2.6",
        "replacement": "exact-version-only",
    }:
        failures.append("Docker plugin package declaration changed")
    if profile.get("engine") != {
        "image": "docker.io/docker/model-runner",
        "manifest_digest": RUNNER_DIGEST,
        "runtime_source_revision": "72874f559c598b8f89fbb24864868337cf5afb4c",
        "runtime_version": "b9879",
        "tag_observed": "v1.2.6-cuda",
    }:
        failures.append("Docker runner image identity changed")
    model = profile.get("model_artifact", {})
    if (
        model.get("manifest_digest") != MODEL_DIGEST
        or model.get("source_admission") != "blocked"
        or not all(
            re.fullmatch(r"sha256:[0-9a-f]{64}", str(model.get(key, "")))
            for key in ("config_digest", "model_layer_digest", "projector_layer_digest")
        )
    ):
        failures.append("Docker model artifact identity or blocked state changed")
    if profile.get("authority") != {
        "credential": False,
        "docker_daemon_control": False,
        "grant": False,
        "host_filesystem": False,
        "model_acquisition": False,
        "network_egress": False,
        "tool": False,
        "workspace": False,
    }:
        failures.append("Docker compatibility authority closure changed")
    daemon = profile.get("daemon_prerequisites", {})
    if (
        daemon.get("daemon_uid") != 0
        or daemon.get("docker_engine_required") is not True
        or daemon.get("docker_socket_exposed_to_adapter") is not False
        or daemon.get("docker_socket_exposed_to_runner") is not False
        or daemon.get("exact_daemon_binary_identity_required") is not True
        or daemon.get("rootless_engine_accepted") is not False
        or daemon.get("launch_user_docker_socket_group_membership") is not False
        or daemon.get("docker_socket_group_risk")
        != "host-equivalent-docker-daemon-authority"
    ):
        failures.append("Docker daemon privilege declaration changed")
    if profile.get("mount_policy") != {
        "credential_mounts": 0,
        "docker_socket_mounts": 0,
        "host_root_mounts": 0,
        "model_content_store": "docker-managed-exact-oci-artifact-only",
        "model_content_store_write": False,
        "private_runtime_tmpfs": True,
        "workspace_mounts": 0,
    }:
        failures.append("Docker mount closure changed")
    if profile.get("network_policy") != {
        "acquisition_allowed": False,
        "ambient_dns": False,
        "ambient_proxy": False,
        "do_not_track": True,
        "outbound_bytes": 0,
        "registry_access": False,
    }:
        failures.append("Docker offline network declaration changed")
    ipc = profile.get("ipc", {})
    if (
        ipc.get("host") != "127.0.0.1"
        or ipc.get("port") != 12434
        or ipc.get("transport") != "guarded-loopback-tcp"
        or ipc.get("management_endpoints_allowed") is not False
        or ipc.get("raw_endpoint_exposed_to_extension") is not False
        or ipc.get("raw_endpoint_exposed_to_tool_worker") is not False
    ):
        failures.append("Docker guarded endpoint declaration changed")
    if profile.get("resource_ceiling") != {
        "cpu_percent": 3200,
        "memory_bytes": 64 * 1024 * 1024 * 1024,
        "output_bytes": 16 * 1024 * 1024,
        "parallel_slots": 1,
        "runtime_seconds": 3600,
        "swap_bytes": 0,
        "tasks": 64,
    }:
        failures.append("Docker resource ceiling changed")
    if profile.get("decision") != {
        "docker_engine_directly_tested": False,
        "enabled_models": 0,
        "inference_implemented": False,
        "release_approval": False,
        "status": "COMPATIBILITY_PROFILE_PINNED_NOT_ACTIVATED",
    }:
        failures.append("Docker compatibility profile was overclaimed")
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
        "docker_engine_contacted": False,
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
    descriptor = json.loads(
        run_command([str(ROOT / "target/debug/agentmage-native-inference"), "--self-check"])
    )
    if descriptor != EXPECTED_DESCRIPTOR:
        raise LinuxDockerRuntimeEvidenceError("adapter compatibility descriptor changed")
    host = host_identity()
    if host != {
        "architecture": "x86_64",
        "distribution": "fedora",
        "version": "44",
        "docker_cli_available": False,
        "docker_engine_contacted": False,
    }:
        raise LinuxDockerRuntimeEvidenceError(
            "this evidence increment requires the declared Docker-absent Fedora host"
        )
    model_state = json.loads(
        (ROOT / "evidence/current/model-activation-report.json").read_text(encoding="utf-8")
    )
    if model_state.get("enabled_profile_count") != 0:
        raise LinuxDockerRuntimeEvidenceError("a model was enabled")
    return {
        "artifact_id": "linux-docker-model-runner-compatibility-profile",
        "authority": profile["authority"],
        "daemon_prerequisites": profile["daemon_prerequisites"],
        "enabled_models": 0,
        "host": host,
        "inference_started": False,
        "limitations": LIMITATIONS,
        "model_artifact": profile["model_artifact"],
        "mount_policy": profile["mount_policy"],
        "network_policy": profile["network_policy"],
        "package": profile["package"],
        "prior_evidence": {
            "docker_engine_directly_tested": False,
            "podman_compatibility_bundle": "artifacts/sprints/sprint-0/story-0.3-dmr/evidence-manifest.json",
            "podman_compatibility_bundle_sha256": sha256_file(
                ROOT / "artifacts/sprints/sprint-0/story-0.3-dmr/evidence-manifest.json"
            ),
            "quality_status": "FAIL",
        },
        "process_boundary": descriptor,
        "profile_sha256": PROFILE_SHA256,
        "release_claim": "none",
        "resource_ceiling": profile["resource_ceiling"],
        "runner_engine": profile["engine"],
        "schema_version": 1,
        "source_revision": revision,
        "sources": source_records(revision),
        "status": "pass-pinned-contract-docker-not-installed",
        "task_ids": ["9.2.1.2"],
    }


def validate_report(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["Linux Docker compatibility report must be an object"]
    failures = []
    if (
        value.get("schema_version") != 1
        or value.get("artifact_id")
        != "linux-docker-model-runner-compatibility-profile"
        or value.get("task_ids") != ["9.2.1.2"]
        or value.get("status") != "pass-pinned-contract-docker-not-installed"
        or REVISION.fullmatch(str(value.get("source_revision"))) is None
    ):
        failures.append("Linux Docker compatibility report identity changed")
    profile = json.loads(PROFILE_PATH.read_text(encoding="utf-8"))
    if validate_profile(profile):
        failures.append("checked Docker compatibility profile is invalid")
    for key in (
        "authority",
        "daemon_prerequisites",
        "model_artifact",
        "mount_policy",
        "network_policy",
        "package",
        "resource_ceiling",
    ):
        if value.get(key) != profile.get(key):
            failures.append(f"Docker compatibility {key} changed")
    if value.get("runner_engine") != profile.get("engine"):
        failures.append("Docker compatibility runner engine changed")
    host = value.get("host", {})
    if (
        host.get("architecture") != "x86_64"
        or host.get("distribution") != "fedora"
        or host.get("version") != "44"
        or host.get("docker_cli_available") is not False
        or host.get("docker_engine_contacted") is not False
    ):
        failures.append("Docker-absent host evidence changed")
    if (
        value.get("enabled_models") != 0
        or value.get("inference_started") is not False
        or value.get("release_claim") != "none"
        or value.get("process_boundary") != EXPECTED_DESCRIPTOR
        or value.get("profile_sha256") != PROFILE_SHA256
        or value.get("limitations") != LIMITATIONS
    ):
        failures.append("Docker compatibility state or limitations were overclaimed")
    prior = value.get("prior_evidence", {})
    if (
        prior.get("docker_engine_directly_tested") is not False
        or prior.get("quality_status") != "FAIL"
        or SHA256.fullmatch(str(prior.get("podman_compatibility_bundle_sha256")))
        is None
    ):
        failures.append("prior compatibility evidence was overstated")
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
        failures.append("Docker compatibility source closure is invalid")
    return failures


def check_report() -> None:
    try:
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise LinuxDockerRuntimeEvidenceError(
            f"cannot read Linux Docker compatibility report: {error}"
        ) from error
    failures = validate_report(report)
    revision = report.get("source_revision")
    if isinstance(revision, str):
        try:
            expected_sources = source_records(revision)
        except LinuxDockerRuntimeEvidenceError as error:
            failures.append(str(error))
        else:
            if report.get("sources") != expected_sources:
                failures.append("Docker compatibility report source records are stale")
    if failures:
        raise LinuxDockerRuntimeEvidenceError("; ".join(failures))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-revision", default="HEAD")
    parser.add_argument("--write", action="store_true")
    arguments = parser.parse_args(argv)
    try:
        if arguments.write:
            revision = git_revision(arguments.source_revision)
            write_atomic(REPORT_PATH, canonical_json(build_report(revision)))
        check_report()
    except (
        LinuxDockerRuntimeEvidenceError,
        OSError,
        TypeError,
        UnicodeError,
        json.JSONDecodeError,
        subprocess.SubprocessError,
    ) as error:
        print(f"Linux Docker runtime evidence failed: {error}", file=sys.stderr)
        return 1
    print("Linux Docker Model Runner compatibility profile evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
