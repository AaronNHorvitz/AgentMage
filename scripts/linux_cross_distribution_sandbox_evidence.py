#!/usr/bin/env python3
"""Build bounded Fedora/Ubuntu Linux worker attack evidence."""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import os
import platform
import re
import subprocess
import sys
import tarfile
import tempfile
import time
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
REPORT_PATH = (
    ROOT
    / "artifacts"
    / "sprints"
    / "sprint-9"
    / "story-9.1"
    / "linux-cross-distribution-sandbox-attacks.json"
)
POLICY_PATH = ROOT / "architecture" / "clean-build-policy.json"
CONTAINERFILE_PATH = ROOT / "release" / "control-test" / "Containerfile.ubuntu"
UBUNTU_BUILD_IMAGE = "localhost/agentmage-clean-build:ubuntu-x86_64"
UBUNTU_CONTROL_IMAGE = "localhost/agentmage-ubuntu-control-test:26.04"
UBUNTU_BASE_IMAGE = (
    "docker.io/library/ubuntu@"
    "sha256:7b202b0e2e0028c6250f5fcf41d04df492d145a1654c6995a6553f0c1f6f1960"
)
SOURCE_PATHS = (
    "Cargo.lock",
    "Cargo.toml",
    "architecture/clean-build-policy.json",
    "docs/architecture/linux-worker-isolation.md",
    "package.json",
    "platforms/linux/Cargo.toml",
    "platforms/linux/src/sandbox.rs",
    "release/control-test/Containerfile.ubuntu",
    "scripts/linux_cross_distribution_sandbox_evidence.py",
    "tests/test_linux_cross_distribution_sandbox_evidence.py",
)
LIVE_TESTS = (
    "bounded_scratch_cannot_escape_into_the_held_object_or_host_workspace",
    "directory_worker_receives_only_the_bounded_exclusion_safe_projection",
    "foreign_workspace_identity_never_starts_a_worker",
    "fresh_worker_reads_only_the_canonical_workspace_file",
    "transient_service_terminates_an_unbounded_worker",
    "worker_cannot_resolve_sibling_parent_or_hidden_descriptor_content",
    "worker_cannot_write_the_read_only_workspace",
    "worker_has_no_ambient_host_paths_devices_or_processes",
    "worker_kernel_status_confirms_no_new_privileges_and_seccomp",
    "worker_output_is_drained_but_never_retained_past_the_bound",
    "worker_receives_only_the_fixed_environment_and_no_network",
)
ATTACK_TESTS = {
    "ambient_home": "worker_has_no_ambient_host_paths_devices_or_processes",
    "device": "worker_has_no_ambient_host_paths_devices_or_processes",
    "environment": "worker_receives_only_the_fixed_environment_and_no_network",
    "network": "worker_receives_only_the_fixed_environment_and_no_network",
    "process": "worker_has_no_ambient_host_paths_devices_or_processes",
    "secret_store": "worker_has_no_ambient_host_paths_devices_or_processes",
    "ungranted_root": "worker_has_no_ambient_host_paths_devices_or_processes",
    "workspace_write": "worker_cannot_write_the_read_only_workspace",
}
EXPECTED_CONTROLS = {
    "bubblewrap_namespaces": "fresh-user-mount-pid-ipc-uts-cgroup-network",
    "cgroup_limits": [
        "CPUQuota",
        "MemoryMax",
        "MemorySwapMax=0",
        "RuntimeMaxSec",
        "TasksMax",
    ],
    "network": "new-network-namespace-plus-address-family-and-seccomp-denial",
    "privileges": "no-new-privileges-private-devices-cap-drop-all",
    "seccomp": "agentmage.linux.worker.deny.v1-kernel-mode-filter",
    "workspace": "single-descriptor-projected-read-only-object",
}
LIMITATIONS = (
    "The Fedora run is native local execution; the Ubuntu run is a userspace portability test inside a rootless Podman container.",
    "Nested Bubblewrap requires the outer rootless test container's privileged flag; that flag grants only the invoking user's rootless-container envelope but prevents this artifact from claiming native Ubuntu isolation.",
    "The Ubuntu control image resolves test-only operating-system packages during an explicit bootstrap step; exact resulting image and trusted-tool digests are recorded, but the image is not a production or release artifact.",
    "Graphical Visual Studio Code execution, native Ubuntu desktop execution, installation support, and release support remain outside this artifact.",
    "No macOS evidence is substituted and no model or inference process is enabled.",
)
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")
IMAGE_ID = re.compile(r"^sha256:[0-9a-f]{64}$")


class CrossDistributionSandboxError(ValueError):
    """Raised when attack evidence is unavailable, malformed, or overclaimed."""


def pretty_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(prefix=".agentmage-linux-attacks-", dir=path.parent)
    temporary = Path(name)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
        temporary.chmod(0o644)
        os.replace(temporary, path)
    except BaseException:
        temporary.unlink(missing_ok=True)
        raise


def run(
    argv: list[str],
    *,
    cwd: Path = ROOT,
    timeout: int = 300,
    environment: dict[str, str] | None = None,
    binary: bool = False,
) -> subprocess.CompletedProcess[Any]:
    return subprocess.run(
        argv,
        cwd=cwd,
        env=environment,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=not binary,
        timeout=timeout,
        check=False,
    )


def checked(
    argv: list[str],
    *,
    cwd: Path = ROOT,
    timeout: int = 300,
    environment: dict[str, str] | None = None,
) -> str:
    completed = run(argv, cwd=cwd, timeout=timeout, environment=environment)
    if completed.returncode != 0:
        raise CrossDistributionSandboxError(f"verification command failed: {argv[0]}")
    return completed.stdout


def git_revision(candidate: str) -> str:
    revision = checked(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"], timeout=30
    ).strip()
    if REVISION.fullmatch(revision) is None:
        raise CrossDistributionSandboxError("source revision is invalid")
    return revision


def git_file(revision: str, relative: str) -> bytes:
    completed = run(
        ["git", "show", f"{revision}:{relative}"], timeout=30, binary=True
    )
    if completed.returncode != 0:
        raise CrossDistributionSandboxError(f"committed source is absent: {relative}")
    return completed.stdout


def source_records(revision: str) -> list[dict[str, str]]:
    records = []
    for relative in SOURCE_PATHS:
        committed = git_file(revision, relative)
        if committed != (ROOT / relative).read_bytes():
            raise CrossDistributionSandboxError(f"source differs from revision: {relative}")
        records.append({"path": relative, "sha256": sha256_bytes(committed)})
    return records


def committed_source(revision: str, destination: Path) -> None:
    archive = run(["git", "archive", "--format=tar", revision], timeout=60, binary=True)
    if archive.returncode != 0:
        raise CrossDistributionSandboxError("committed source archive is unavailable")
    with tarfile.open(fileobj=io.BytesIO(archive.stdout), mode="r:") as bundle:
        bundle.extractall(destination, filter="data")


def read_policy() -> dict[str, Any]:
    try:
        policy = json.loads(POLICY_PATH.read_text(encoding="utf-8"))
        ubuntu = policy["linux_platforms"]["ubuntu-x86_64"]
    except (OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise CrossDistributionSandboxError("clean-build policy is unavailable") from error
    if (
        ubuntu.get("os_id") != "ubuntu"
        or ubuntu.get("version_id") != "26.04"
        or ubuntu.get("base_image") != UBUNTU_BASE_IMAGE
    ):
        raise CrossDistributionSandboxError("Ubuntu clean-build identity changed")
    return policy


def rootless_podman() -> None:
    value = checked(
        ["podman", "info", "--format", "{{.Host.Security.Rootless}}"], timeout=60
    ).strip()
    if value != "true":
        raise CrossDistributionSandboxError("rootless Podman is required")


def image_id(tag: str) -> str:
    value = checked(
        ["podman", "image", "inspect", tag, "--format", "{{.Id}}"], timeout=60
    ).strip()
    if SHA256.fullmatch(value):
        value = f"sha256:{value}"
    if IMAGE_ID.fullmatch(value) is None:
        raise CrossDistributionSandboxError(f"container image identity is invalid: {tag}")
    return value


def bootstrap_ubuntu_image(source: Path, base_image: str) -> str:
    completed = run(
        [
            "podman",
            "build",
            "--pull=never",
            "--build-arg",
            f"BASE_IMAGE={base_image}",
            "--file",
            str(source / "release" / "control-test" / "Containerfile.ubuntu"),
            "--tag",
            UBUNTU_CONTROL_IMAGE,
            str(source),
        ],
        timeout=1800,
    )
    if completed.returncode != 0:
        raise CrossDistributionSandboxError("Ubuntu control image bootstrap failed")
    return image_id(UBUNTU_CONTROL_IMAGE)


def compile_ubuntu_tests(source: Path, target: Path) -> Path:
    environment = os.environ.copy()
    environment["CARGO_NET_OFFLINE"] = "true"
    completed = run(
        [
            "podman",
            "run",
            "--rm",
            "--network=none",
            "--security-opt=label=disable",
            "--userns=keep-id:uid=10001,gid=10001",
            "--memory=16g",
            "--cpus=8",
            "--pids-limit=2048",
            "--tmpfs=/tmp:rw,nosuid,nodev,size=2g",
            "--env=CARGO_INCREMENTAL=0",
            "--env=CARGO_TARGET_DIR=/target",
            f"--volume={source}:/workspace:ro",
            f"--volume={target}:/target",
            "--workdir=/workspace",
            UBUNTU_BUILD_IMAGE,
            "cargo",
            "test",
            "--locked",
            "--offline",
            "-p",
            "agentmage-platform-linux",
            "--no-run",
        ],
        timeout=900,
        environment=environment,
    )
    if completed.returncode != 0:
        raise CrossDistributionSandboxError("Ubuntu test binary compilation failed")
    binaries = sorted(
        path
        for path in (target / "debug" / "deps").glob("agentmage_platform_linux-*")
        if path.is_file() and os.access(path, os.X_OK) and path.suffix == ""
    )
    if len(binaries) != 1:
        raise CrossDistributionSandboxError("Ubuntu test binary closure is not exact")
    return binaries[0]


def parse_test_output(output: str) -> list[dict[str, str]]:
    observed = {
        match.group(1)
        for line in output.splitlines()
        if (match := re.fullmatch(r"test sandbox::tests::([^: ]+) \.\.\. ok", line))
    }
    if observed != set(LIVE_TESTS):
        raise CrossDistributionSandboxError("live sandbox test closure is incomplete")
    return [{"test": name, "status": "pass"} for name in LIVE_TESTS]


def fedora_identity() -> dict[str, Any]:
    release: dict[str, str] = {}
    for line in Path("/etc/os-release").read_text(encoding="utf-8").splitlines():
        if "=" in line:
            key, value = line.split("=", 1)
            release[key] = value.strip('"')
    if (
        release.get("ID") != "fedora"
        or release.get("VERSION_ID") != "44"
        or platform.machine() != "x86_64"
    ):
        raise CrossDistributionSandboxError("native Fedora 44 x86_64 host is required")
    if " - cgroup2 " not in Path("/proc/self/mountinfo").read_text(encoding="utf-8"):
        raise CrossDistributionSandboxError("native Fedora cgroup v2 is unavailable")
    return {
        "architecture": "x86_64",
        "cgroup_filesystem": "cgroup2",
        "distribution": "fedora",
        "version": "44",
    }


def run_fedora_tests(source: Path, target: Path) -> list[dict[str, str]]:
    environment = os.environ.copy()
    environment["CARGO_NET_OFFLINE"] = "true"
    environment["CARGO_INCREMENTAL"] = "0"
    environment["CARGO_TARGET_DIR"] = str(target)
    output = checked(
        [
            "cargo",
            "test",
            "--locked",
            "--offline",
            "-p",
            "agentmage-platform-linux",
            "sandbox::tests",
            "--",
            "--ignored",
            "--test-threads=1",
        ],
        cwd=source,
        timeout=300,
        environment=environment,
    )
    return parse_test_output(output)


def container_exec(name: str, argv: list[str], *, user: str | None = None) -> str:
    command = ["podman", "exec"]
    if user is not None:
        command.extend(["--user", user])
    command.extend([name, *argv])
    return checked(command, timeout=300)


def wait_for_user_manager(name: str) -> None:
    for _ in range(50):
        completed = run(
            [
                "podman",
                "exec",
                name,
                "/usr/bin/systemctl",
                "show",
                "--property=Version",
            ],
            timeout=10,
        )
        if completed.returncode == 0:
            break
        time.sleep(0.1)
    else:
        raise CrossDistributionSandboxError("Ubuntu system manager did not activate")
    checked(
        [
            "podman",
            "exec",
            name,
            "/usr/bin/systemctl",
            "start",
            "user@10001.service",
        ],
        timeout=60,
    )
    for _ in range(50):
        completed = run(
            [
                "podman",
                "exec",
                name,
                "/usr/bin/systemctl",
                "is-active",
                "user@10001.service",
            ],
            timeout=10,
        )
        if completed.returncode == 0 and completed.stdout.strip() == "active":
            return
        time.sleep(0.1)
    raise CrossDistributionSandboxError("Ubuntu synthetic user manager did not activate")


def ubuntu_tool(name: str, path: str, container: str) -> dict[str, Any]:
    canonical = container_exec(
        container, ["/usr/bin/readlink", "-f", path]
    ).strip()
    metadata = container_exec(
        container, ["/usr/bin/stat", "-Lc", "%u %a %F", canonical]
    ).strip()
    fields = metadata.split(maxsplit=2)
    try:
        unsafe_mode = int(fields[1], 8) & 0o022 if len(fields) == 3 else True
    except ValueError as error:
        raise CrossDistributionSandboxError(
            f"Ubuntu trusted tool metadata is invalid: {name}"
        ) from error
    if len(fields) != 3 or fields[0] != "0" or unsafe_mode:
        raise CrossDistributionSandboxError(f"Ubuntu trusted tool is unsafe: {name}")
    digest_fields = container_exec(
        container, ["/usr/bin/sha256sum", canonical]
    ).split()
    digest = digest_fields[0] if digest_fields else ""
    if SHA256.fullmatch(digest) is None:
        raise CrossDistributionSandboxError(f"Ubuntu trusted tool digest is invalid: {name}")
    version_lines = container_exec(container, [path, "--version"]).splitlines()
    version = version_lines[0].strip() if version_lines else ""
    if not version or len(version) > 160:
        raise CrossDistributionSandboxError(f"Ubuntu trusted tool version is invalid: {name}")
    return {
        "canonical_path_class": "root-owned-system-path",
        "group_or_world_writable": False,
        "id": name,
        "owner_uid": 0,
        "sha256": digest,
        "version": version,
    }


def run_ubuntu_tests(
    binary: Path, build_image_id: str, control_image_id: str
) -> dict[str, Any]:
    container = f"agentmage-ubuntu-attacks-{os.getpid()}"
    started = False
    try:
        checked(
            [
                "podman",
                "run",
                "--detach",
                "--name",
                container,
                "--network=none",
                "--cgroupns=private",
                "--privileged",
                "--memory=2g",
                "--pids-limit=512",
                UBUNTU_CONTROL_IMAGE,
            ],
            timeout=60,
        )
        started = True
        wait_for_user_manager(container)
        checked(
            [
                "podman",
                "cp",
                str(binary),
                f"{container}:/usr/local/bin/agentmage-platform-linux-tests",
            ],
            timeout=60,
        )
        checked(
            [
                "podman",
                "exec",
                container,
                "/usr/bin/chmod",
                "0755",
                "/usr/local/bin/agentmage-platform-linux-tests",
            ],
            timeout=30,
        )
        identity = container_exec(
            container,
            [
                "/usr/bin/bash",
                "-c",
                ". /etc/os-release; printf '%s %s %s' \"$ID\" \"$VERSION_ID\" \"$(uname -m)\"",
            ],
        ).strip()
        if identity != "ubuntu 26.04 x86_64":
            raise CrossDistributionSandboxError("Ubuntu runtime identity changed")
        mountinfo = container_exec(
            container, ["/usr/bin/cat", "/proc/self/mountinfo"]
        )
        if " - cgroup2 " not in mountinfo:
            raise CrossDistributionSandboxError("Ubuntu cgroup v2 is unavailable")
        uid = container_exec(container, ["/usr/bin/id", "-u"], user="10001:10001").strip()
        if uid != "10001":
            raise CrossDistributionSandboxError("Ubuntu test user identity changed")
        output = checked(
            [
                "podman",
                "exec",
                "--user",
                "10001:10001",
                "--env",
                "XDG_RUNTIME_DIR=/run/user/10001",
                "--env",
                "DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/10001/bus",
                container,
                "/usr/local/bin/agentmage-platform-linux-tests",
                "sandbox::tests",
                "--ignored",
                "--test-threads=1",
            ],
            timeout=300,
        )
        inspect = json.loads(checked(["podman", "inspect", container], timeout=60))[0]
        host = inspect.get("HostConfig", {})
        running_image = str(inspect.get("Image", ""))
        if SHA256.fullmatch(running_image):
            running_image = f"sha256:{running_image}"
        if (
            running_image != control_image_id
            or inspect.get("State", {}).get("Running") is not True
            or host.get("NetworkMode") != "none"
            or host.get("CgroupMode") != "private"
            or host.get("Privileged") is not True
            or host.get("PidsLimit") != 512
            or host.get("Memory") != 2 * 1024 * 1024 * 1024
        ):
            raise CrossDistributionSandboxError("Ubuntu outer test envelope changed")
        return {
            "architecture": "x86_64",
            "base_image": UBUNTU_BASE_IMAGE,
            "binary_sha256": sha256_bytes(binary.read_bytes()),
            "build_image_id": build_image_id,
            "cgroup_filesystem": "cgroup2",
            "container_controls": {
                "cgroup_namespace": "private",
                "memory_limit_bytes": 2 * 1024 * 1024 * 1024,
                "network": "none",
                "outer_privileged_flag": True,
                "pids_limit": 512,
                "podman_rootless": True,
            },
            "container_image_id": control_image_id,
            "distribution": "ubuntu",
            "execution_context": "standard-user-in-rootless-privileged-test-envelope",
            "native_platform_claim": False,
            "tests": parse_test_output(output),
            "trusted_tools": [
                ubuntu_tool("bubblewrap", "/usr/bin/bwrap", container),
                ubuntu_tool("systemd-run", "/usr/bin/systemd-run", container),
            ],
            "uid": 10001,
            "version": "26.04",
        }
    finally:
        if started:
            run(["podman", "stop", "--time", "5", container], timeout=30)
            run(["podman", "rm", "--force", container], timeout=30)


def build_report(revision: str, *, bootstrap: bool) -> dict[str, Any]:
    rootless_podman()
    policy = read_policy()
    sources = source_records(revision)
    with tempfile.TemporaryDirectory(prefix="agentmage-linux-attacks-") as temporary:
        root = Path(temporary)
        source = root / "source"
        fedora_target = root / "fedora-target"
        ubuntu_target = root / "ubuntu-target"
        source.mkdir()
        fedora_target.mkdir()
        ubuntu_target.mkdir()
        committed_source(revision, source)
        fedora = {
            **fedora_identity(),
            "execution_context": "native-standard-user",
            "native_platform_claim": True,
            "tests": run_fedora_tests(source, fedora_target),
            "uid": os.getuid(),
        }
        if bootstrap:
            control_image_id = bootstrap_ubuntu_image(
                source,
                policy["linux_platforms"]["ubuntu-x86_64"]["base_image"],
            )
        else:
            control_image_id = image_id(UBUNTU_CONTROL_IMAGE)
        build_image_id = image_id(UBUNTU_BUILD_IMAGE)
        binary = compile_ubuntu_tests(source, ubuntu_target)
        ubuntu = run_ubuntu_tests(binary, build_image_id, control_image_id)
    platforms = {"fedora-44-x86_64": fedora, "ubuntu-26.04-x86_64": ubuntu}
    return {
        "schema_version": 1,
        "artifact_id": "linux-cross-distribution-sandbox-attacks",
        "task_ids": ["9.1.3.2"],
        "test_ids": ["S-009-ST01"],
        "status": "pass-bounded-cross-distribution",
        "source_revision": revision,
        "sources": sources,
        "controls": EXPECTED_CONTROLS,
        "platforms": platforms,
        "attack_results": [
            {
                "attack": attack,
                "fedora": "pass-zero-escape",
                "test": test,
                "ubuntu": "pass-zero-escape-in-declared-envelope",
            }
            for attack, test in ATTACK_TESTS.items()
        ],
        "summary": {
            "attack_class_count": len(ATTACK_TESTS),
            "cross_distribution_zero_escape_within_declared_boundaries": True,
            "live_test_count_per_platform": len(LIVE_TESTS),
            "native_ubuntu_isolation_verified": False,
        },
        "private_values_present": False,
        "macos_evidence_substituted": False,
        "release_claim": "none",
        "limitations": list(LIMITATIONS),
    }


def valid_tests(value: Any) -> bool:
    return (
        isinstance(value, list)
        and len(value) == len(LIVE_TESTS)
        and all(isinstance(item, dict) for item in value)
        and [item.get("test") for item in value] == list(LIVE_TESTS)
        and all(item.get("status") == "pass" for item in value)
    )


def validate_report(value: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict):
        return ["cross-distribution sandbox report must be an object"]
    if (
        value.get("schema_version") != 1
        or value.get("artifact_id") != "linux-cross-distribution-sandbox-attacks"
        or value.get("task_ids") != ["9.1.3.2"]
        or value.get("test_ids") != ["S-009-ST01"]
        or value.get("status") != "pass-bounded-cross-distribution"
        or REVISION.fullmatch(str(value.get("source_revision"))) is None
    ):
        failures.append("cross-distribution sandbox report identity changed")
    if value.get("controls") != EXPECTED_CONTROLS:
        failures.append("cross-distribution sandbox controls changed")
    platforms = value.get("platforms")
    if not isinstance(platforms, dict) or set(platforms) != {
        "fedora-44-x86_64",
        "ubuntu-26.04-x86_64",
    }:
        failures.append("cross-distribution platform closure changed")
    else:
        fedora = platforms["fedora-44-x86_64"]
        ubuntu = platforms["ubuntu-26.04-x86_64"]
        if (
            not isinstance(fedora, dict)
            or fedora.get("distribution") != "fedora"
            or fedora.get("version") != "44"
            or fedora.get("architecture") != "x86_64"
            or fedora.get("execution_context") != "native-standard-user"
            or fedora.get("native_platform_claim") is not True
            or not isinstance(fedora.get("uid"), int)
            or fedora.get("uid") <= 0
            or not valid_tests(fedora.get("tests"))
        ):
            failures.append("native Fedora sandbox evidence is incomplete")
        controls = ubuntu.get("container_controls", {}) if isinstance(ubuntu, dict) else {}
        if (
            not isinstance(ubuntu, dict)
            or ubuntu.get("distribution") != "ubuntu"
            or ubuntu.get("version") != "26.04"
            or ubuntu.get("architecture") != "x86_64"
            or ubuntu.get("base_image") != UBUNTU_BASE_IMAGE
            or ubuntu.get("execution_context")
            != "standard-user-in-rootless-privileged-test-envelope"
            or ubuntu.get("native_platform_claim") is not False
            or ubuntu.get("uid") != 10001
            or not valid_tests(ubuntu.get("tests"))
            or controls
            != {
                "cgroup_namespace": "private",
                "memory_limit_bytes": 2 * 1024 * 1024 * 1024,
                "network": "none",
                "outer_privileged_flag": True,
                "pids_limit": 512,
                "podman_rootless": True,
            }
            or IMAGE_ID.fullmatch(str(ubuntu.get("build_image_id"))) is None
            or IMAGE_ID.fullmatch(str(ubuntu.get("container_image_id"))) is None
            or SHA256.fullmatch(str(ubuntu.get("binary_sha256"))) is None
        ):
            failures.append("bounded Ubuntu sandbox evidence is incomplete or overclaimed")
        tools = ubuntu.get("trusted_tools") if isinstance(ubuntu, dict) else None
        if (
            not isinstance(tools, list)
            or not all(isinstance(item, dict) for item in tools)
            or [item.get("id") for item in tools] != ["bubblewrap", "systemd-run"]
            or any(
                item.get("canonical_path_class") != "root-owned-system-path"
                or item.get("owner_uid") != 0
                or item.get("group_or_world_writable") is not False
                or SHA256.fullmatch(str(item.get("sha256"))) is None
                or not isinstance(item.get("version"), str)
                or not item.get("version")
                for item in tools
            )
        ):
            failures.append("Ubuntu trusted-tool evidence is incomplete")
    expected_attacks = [
        {
            "attack": attack,
            "fedora": "pass-zero-escape",
            "test": test,
            "ubuntu": "pass-zero-escape-in-declared-envelope",
        }
        for attack, test in ATTACK_TESTS.items()
    ]
    if value.get("attack_results") != expected_attacks:
        failures.append("cross-distribution attack closure changed")
    if value.get("summary") != {
        "attack_class_count": len(ATTACK_TESTS),
        "cross_distribution_zero_escape_within_declared_boundaries": True,
        "live_test_count_per_platform": len(LIVE_TESTS),
        "native_ubuntu_isolation_verified": False,
    }:
        failures.append("cross-distribution sandbox summary changed or overclaimed")
    sources = value.get("sources")
    if (
        not isinstance(sources, list)
        or not all(isinstance(item, dict) for item in sources)
        or [item.get("path") for item in sources] != list(SOURCE_PATHS)
        or any(SHA256.fullmatch(str(item.get("sha256"))) is None for item in sources)
    ):
        failures.append("cross-distribution sandbox source closure changed")
    if (
        value.get("private_values_present") is not False
        or value.get("macos_evidence_substituted") is not False
        or value.get("release_claim") != "none"
        or value.get("limitations") != list(LIMITATIONS)
    ):
        failures.append("cross-distribution sandbox report made an unsupported claim")
    return failures


def check_report() -> None:
    try:
        value = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        revision = value["source_revision"]
    except (OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise CrossDistributionSandboxError("sandbox attack report is unavailable") from error
    failures = validate_report(value)
    if not isinstance(revision, str):
        failures.append("sandbox attack source revision is invalid")
    else:
        expected_sources = source_records(revision)
        if value.get("sources") != expected_sources:
            failures.append("sandbox attack report is stale")
    if failures:
        raise CrossDistributionSandboxError("; ".join(failures))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    parser.add_argument("--bootstrap-ubuntu-image", action="store_true")
    arguments = parser.parse_args()
    try:
        if arguments.bootstrap_ubuntu_image and not arguments.write:
            raise CrossDistributionSandboxError(
                "Ubuntu image bootstrap is valid only while writing fresh evidence"
            )
        if arguments.write:
            revision = git_revision(arguments.source_revision)
            report = build_report(
                revision, bootstrap=arguments.bootstrap_ubuntu_image
            )
            write_atomic(REPORT_PATH, pretty_json(report))
        check_report()
    except (
        CrossDistributionSandboxError,
        IndexError,
        OSError,
        TypeError,
        UnicodeError,
        subprocess.SubprocessError,
    ) as error:
        print(f"Cross-distribution sandbox evidence failed: {error}", file=sys.stderr)
        return 1
    print("Fedora and bounded Ubuntu Linux sandbox attack evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
