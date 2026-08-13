#!/usr/bin/env python3
"""Build and validate native Ubuntu-kernel Linux control evidence."""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import os
import re
import shutil
import signal
import socket
import stat
import subprocess
import sys
import tarfile
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
REPORT_PATH = (
    ROOT
    / "artifacts"
    / "sprints"
    / "sprint-9"
    / "story-9.1"
    / "linux-native-ubuntu-control-verification.json"
)
CACHE_ROOT = Path.home() / ".cache" / "agentmage" / "ubuntu-vm"
OFFICIAL_IMAGE_PATH = CACHE_ROOT / "ubuntu-26.04-server-cloudimg-amd64.img"
PREPARED_IMAGE_PATH = CACHE_ROOT / "ubuntu-26.04-native-controls.qcow2"
PREPARED_METADATA_PATH = CACHE_ROOT / "ubuntu-26.04-native-controls.json"
OFFICIAL_IMAGE_URL = (
    "https://cloud-images.ubuntu.com/releases/resolute/release/"
    "ubuntu-26.04-server-cloudimg-amd64.img"
)
OFFICIAL_IMAGE_SHA256 = (
    "9dc7c5363c0146a08ba0c9aa834d82c2c6dfbb1c471ad9a2f0aba1189e21be05"
)
PREPARED_VIRTUAL_SIZE_BYTES = 12 * 1024 * 1024 * 1024
UBUNTU_BUILD_IMAGE = "localhost/agentmage-clean-build:ubuntu-x86_64"
TEST_UID = 10001
TEST_GID = 10001
SOURCE_PATHS = (
    "Cargo.lock",
    "Cargo.toml",
    "architecture/clean-build-policy.json",
    "docs/architecture/linux-worker-isolation.md",
    "docs/support/linux-native-ubuntu-control-evidence.md",
    "kernel/engine/src/platform_startup.rs",
    "package.json",
    "platforms/linux/Cargo.toml",
    "platforms/linux/src/ipc.rs",
    "platforms/linux/src/platform.rs",
    "platforms/linux/src/sandbox.rs",
    "platforms/linux/src/security_controls.rs",
    "platforms/linux/src/secret_service.rs",
    "scripts/linux_native_ubuntu_control_evidence.py",
    "tests/test_linux_native_ubuntu_control_evidence.py",
)
BOOTSTRAP_PACKAGES = (
    "apparmor-utils",
    "bubblewrap",
    "ca-certificates",
    "coreutils",
    "dbus-user-session",
    "gnome-keyring",
    "iproute2",
    "libsecret-tools",
    "openssh-server",
    "procps",
    "strace",
    "sudo",
    "util-linux",
)
LIVE_SANDBOX_TESTS = (
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
IPC_TESTS = (
    "authenticated_endpoint_exposes_only_bounded_product_frames",
    "every_peer_and_frame_mutation_fails_without_consuming_valid_request",
    "exact_peer_authenticates_once_and_replay_fails",
    "generated_launch_material_is_fresh_and_redacted",
    "listener_drop_removes_only_its_unchanged_socket_identity",
    "private_socket_uses_kernel_peer_credentials_and_exact_frame",
    "process_start_parser_handles_parentheses_and_rejects_malformed_records",
    "unsafe_parent_existing_socket_and_short_frame_fail_closed",
)
SECRET_SERVICE_TESTS = (
    "key_debug_and_errors_never_disclose_candidate_content",
    "keys_values_and_manifests_are_bounded_and_redacted",
    "operational_store_key_decode_is_exact_and_bounded",
    "sensitive_output_is_bounded_without_a_content_digest",
)
SECRET_SERVICE_LIVE_TESTS = (
    "live_operational_key_provisioning_is_exact_non_overwriting_and_cleaned",
    "live_service_probe_returns_only_a_content_free_receipt",
    "live_service_round_trip_is_exact_and_cleanup_is_verified",
)
STARTUP_MAPPING_TESTS = (
    "control_identities_are_nonzero_distinct_and_status_independent",
    "every_linux_control_disablement_maps_to_fail_closed_startup_capabilities",
)
STARTUP_LIVE_TESTS = (
    "live_required_control_preflight_verifies_without_workspace_authority",
)
KERNEL_STARTUP_TESTS = ("every_missing_invalid_or_substituted_mechanism_refuses",)
STARTUP_CONTROL_IDS = (
    "bubblewrap",
    "user-namespaces",
    "seccomp",
    "cgroups-v2",
    "secret-service",
    "descriptor-safe-paths",
    "network-isolation",
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
TRUSTED_TOOLS = (
    ("bubblewrap", "/usr/bin/bwrap", "bubblewrap"),
    ("path-executor", "/usr/bin/env", "coreutils"),
    ("secret-tool", "/usr/bin/secret-tool", "libsecret-tools"),
    ("systemd-run", "/usr/bin/systemd-run", "systemd"),
)
CGROUP_PROPERTIES = (
    "CPUQuota",
    "MemoryMax",
    "MemorySwapMax=0",
    "RuntimeMaxSec",
    "TasksMax",
)
KERNEL_MARKER = "agentmage-kernel-control no-new-privileges=1 seccomp=2"
RESOURCE_MARKER = "agentmage-resource-control runtime-limit=enforced"
TASK_IDS = ("9.1.2.2", "9.1.2.4", "9.1.3.2", "9.1.3.3", "9.1.3.5")
TEST_IDS = ("S-009-UT01", "S-009-ST01", "S-009-UT02")
LIMITATIONS = (
    "The Ubuntu controls execute under an Ubuntu 26.04 kernel in a hardware-accelerated KVM guest; this is native-kernel virtual-machine evidence, not physical-host certification.",
    "Network access is permitted only while preparing the reusable guest from the digest-pinned official image; the acceptance guest uses QEMU restrict mode with one loopback SSH forward and proves external connection denial.",
    "The test binaries are unsigned development candidates compiled from the exact committed revision in the pinned Ubuntu build toolchain image; no supported-release claim is made.",
    "No model artifact is installed, no inference is performed, and no private workspace or credential is introduced.",
    "macOS evidence is outside this artifact and is not substituted.",
)
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")
IMAGE_ID = re.compile(r"^sha256:[0-9a-f]{64}$")
KERNEL_RELEASE = re.compile(r"^[0-9A-Za-z.+_-]{1,128}$")


class NativeUbuntuEvidenceError(ValueError):
    """Raised when native Ubuntu evidence is unavailable or overclaimed."""


@dataclass(frozen=True)
class HostTools:
    """Commands used to invoke QEMU utilities on the immutable host."""

    qemu: tuple[str, ...]
    qemu_img: tuple[str, ...]
    cloud_localds: tuple[str, ...]
    launcher_class: str
    qemu_version: str
    qemu_sha256: str


@dataclass(frozen=True)
class VmHandle:
    """One disposable QEMU process and its host-only SSH endpoint."""

    pid: int
    port: int
    image: Path
    pid_file: Path
    private_key: Path


def pretty_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        while block := handle.read(1024 * 1024):
            digest.update(block)
    return digest.hexdigest()


def write_atomic(path: Path, content: bytes, mode: int = 0o644) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(prefix=".agentmage-ubuntu-native-", dir=path.parent)
    temporary = Path(name)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
        temporary.chmod(mode)
        os.replace(temporary, path)
    except BaseException:
        temporary.unlink(missing_ok=True)
        raise


def run(
    argv: list[str] | tuple[str, ...],
    *,
    cwd: Path = ROOT,
    timeout: int = 300,
    input_value: str | bytes | None = None,
    binary: bool = False,
    environment: dict[str, str] | None = None,
) -> subprocess.CompletedProcess[Any]:
    return subprocess.run(
        list(argv),
        cwd=cwd,
        env=environment,
        input=input_value,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=not binary,
        timeout=timeout,
        check=False,
    )


def checked(
    argv: list[str] | tuple[str, ...],
    *,
    cwd: Path = ROOT,
    timeout: int = 300,
    input_value: str | None = None,
    environment: dict[str, str] | None = None,
) -> str:
    completed = run(
        argv,
        cwd=cwd,
        timeout=timeout,
        input_value=input_value,
        environment=environment,
    )
    if completed.returncode != 0:
        executable = Path(argv[0]).name if argv else "unknown"
        raise NativeUbuntuEvidenceError(f"verification command failed: {executable}")
    return completed.stdout


def git_revision(candidate: str = "HEAD") -> str:
    revision = checked(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"], timeout=30
    ).strip()
    if REVISION.fullmatch(revision) is None:
        raise NativeUbuntuEvidenceError("source revision is invalid")
    return revision


def git_file(revision: str, relative: str) -> bytes:
    completed = run(
        ["git", "show", f"{revision}:{relative}"], timeout=30, binary=True
    )
    if completed.returncode != 0:
        raise NativeUbuntuEvidenceError(f"committed source is absent: {relative}")
    return completed.stdout


def source_records(revision: str) -> list[dict[str, str]]:
    records = []
    for relative in SOURCE_PATHS:
        committed = git_file(revision, relative)
        if committed != (ROOT / relative).read_bytes():
            raise NativeUbuntuEvidenceError(f"source differs from revision: {relative}")
        records.append({"path": relative, "sha256": sha256_bytes(committed)})
    return records


def committed_source(revision: str, destination: Path) -> None:
    archive = run(
        ["git", "archive", "--format=tar", revision], timeout=60, binary=True
    )
    if archive.returncode != 0:
        raise NativeUbuntuEvidenceError("committed source archive is unavailable")
    with tarfile.open(fileobj=io.BytesIO(archive.stdout), mode="r:") as bundle:
        bundle.extractall(destination, filter="data")


def executable_image_id(tag: str) -> str:
    value = checked(
        ["podman", "image", "inspect", tag, "--format", "{{.Id}}"], timeout=60
    ).strip()
    if SHA256.fullmatch(value):
        value = f"sha256:{value}"
    if IMAGE_ID.fullmatch(value) is None:
        raise NativeUbuntuEvidenceError("Ubuntu build image identity is invalid")
    return value


def rootless_podman() -> None:
    value = checked(
        ["podman", "info", "--format", "{{.Host.Security.Rootless}}"], timeout=60
    ).strip()
    if value != "true":
        raise NativeUbuntuEvidenceError("rootless Podman is required")


def _toolbox_prefix(container: str) -> tuple[str, ...]:
    return ("toolbox", "run", "--container", container)


def discover_host_tools(toolbox_container: str) -> HostTools:
    host = {
        name: shutil.which(name)
        for name in ("qemu-system-x86_64", "qemu-img", "cloud-localds")
    }
    if all(host.values()):
        qemu = (str(host["qemu-system-x86_64"]),)
        qemu_img = (str(host["qemu-img"]),)
        cloud_localds = (str(host["cloud-localds"]),)
        launcher_class = "host-system-path"
        qemu_path = str(host["qemu-system-x86_64"])
        qemu_sha = sha256_file(Path(qemu_path))
    else:
        if shutil.which("toolbox") is None:
            raise NativeUbuntuEvidenceError("QEMU utilities are unavailable")
        prefix = _toolbox_prefix(toolbox_container)
        for name in ("qemu-system-x86_64", "qemu-img", "cloud-localds"):
            completed = run([*prefix, "/usr/bin/test", "-x", f"/usr/bin/{name}"], timeout=30)
            if completed.returncode != 0:
                raise NativeUbuntuEvidenceError(
                    f"QEMU utility is unavailable in toolbox: {name}"
                )
        qemu = (*prefix, "/usr/bin/qemu-system-x86_64")
        qemu_img = (*prefix, "/usr/bin/qemu-img")
        cloud_localds = (*prefix, "/usr/bin/cloud-localds")
        launcher_class = "fedora-toolbox"
        qemu_sha = checked(
            [*prefix, "/usr/bin/sha256sum", "/usr/bin/qemu-system-x86_64"],
            timeout=30,
        ).split()[0]
    version_lines = checked([*qemu, "--version"], timeout=30).splitlines()
    version = version_lines[0].strip() if version_lines else ""
    if not version or len(version) > 160 or SHA256.fullmatch(qemu_sha) is None:
        raise NativeUbuntuEvidenceError("QEMU identity is invalid")
    kvm = Path("/dev/kvm")
    try:
        metadata = kvm.stat()
    except OSError as error:
        raise NativeUbuntuEvidenceError("KVM acceleration is unavailable") from error
    if not stat.S_ISCHR(metadata.st_mode) or not os.access(kvm, os.R_OK | os.W_OK):
        raise NativeUbuntuEvidenceError("KVM device is not accessible")
    return HostTools(
        qemu=qemu,
        qemu_img=qemu_img,
        cloud_localds=cloud_localds,
        launcher_class=launcher_class,
        qemu_version=version,
        qemu_sha256=qemu_sha,
    )


def generate_ssh_key(directory: Path) -> tuple[Path, str]:
    private_key = directory / "acceptance-key"
    checked(
        [
            "ssh-keygen",
            "-q",
            "-t",
            "ed25519",
            "-N",
            "",
            "-C",
            "agentmage-native-ubuntu-acceptance",
            "-f",
            str(private_key),
        ],
        timeout=30,
    )
    private_key.chmod(0o600)
    fields = private_key.with_suffix(".pub").read_text(encoding="ascii").split()
    if len(fields) < 2 or fields[0] != "ssh-ed25519":
        raise NativeUbuntuEvidenceError("ephemeral SSH key is invalid")
    return private_key, f"{fields[0]} {fields[1]}"


def render_user_data(public_key: str, *, bootstrap: bool) -> str:
    package_lines = "\n".join(f"  - {name}" for name in BOOTSTRAP_PACKAGES)
    package_section = (
        f"package_update: true\npackage_upgrade: false\npackages:\n{package_lines}\n"
        if bootstrap
        else "package_update: false\npackage_upgrade: false\n"
    )
    return (
        "#cloud-config\n"
        "hostname: agentmage-ubuntu-control\n"
        "manage_etc_hosts: true\n"
        "disable_root: true\n"
        "ssh_pwauth: false\n"
        "users:\n"
        "  - name: agentmage\n"
        f"    uid: {TEST_UID}\n"
        "    gecos: AgentMage acceptance user\n"
        "    shell: /bin/bash\n"
        "    groups: [adm, sudo, systemd-journal]\n"
        "    sudo: ALL=(ALL) NOPASSWD:ALL\n"
        "    lock_passwd: true\n"
        "    ssh_authorized_keys:\n"
        f"      - {public_key}\n"
        f"{package_section}"
        "runcmd:\n"
        "  - [loginctl, enable-linger, agentmage]\n"
        "  - [systemctl, enable, ssh.service]\n"
        "  - [systemctl, start, user@10001.service]\n"
        "  - [install, -d, -o, '10001', -g, '10001', -m, '0700', "
        "/home/agentmage/acceptance]\n"
        "  - [touch, /var/lib/agentmage-cloud-init-complete]\n"
    )


def create_seed(
    tools: HostTools,
    directory: Path,
    public_key: str,
    *,
    bootstrap: bool,
) -> Path:
    user_data = directory / "user-data"
    meta_data = directory / "meta-data"
    seed = directory / "seed.iso"
    user_data.write_text(
        render_user_data(public_key, bootstrap=bootstrap), encoding="ascii"
    )
    meta_data.write_text(
        "instance-id: agentmage-ubuntu-native-"
        f"{'bootstrap' if bootstrap else 'acceptance'}-{os.getpid()}\n"
        "local-hostname: agentmage-ubuntu-control\n",
        encoding="ascii",
    )
    user_data.chmod(0o600)
    meta_data.chmod(0o600)
    checked([*tools.cloud_localds, str(seed), str(user_data), str(meta_data)], timeout=60)
    return seed


def available_loopback_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
        listener.bind(("127.0.0.1", 0))
        return int(listener.getsockname()[1])


def create_overlay(tools: HostTools, base: Path, overlay: Path) -> None:
    checked(
        [
            *tools.qemu_img,
            "create",
            "-q",
            "-f",
            "qcow2",
            "-F",
            "qcow2",
            "-b",
            str(base),
            str(overlay),
        ],
        timeout=60,
    )


def start_vm(
    tools: HostTools,
    image: Path,
    seed: Path,
    private_key: Path,
    directory: Path,
    *,
    restricted_network: bool,
) -> VmHandle:
    pid_file = directory / "qemu.pid"
    serial_log = directory / "serial.log"
    for _ in range(8):
        port = available_loopback_port()
        network = f"user,id=net0,hostfwd=tcp:127.0.0.1:{port}-:22"
        if restricted_network:
            network += ",restrict=on"
        completed = run(
            [
                *tools.qemu,
                "-accel",
                "kvm",
                "-cpu",
                "host",
                "-machine",
                "q35",
                "-smp",
                "4",
                "-m",
                "4096",
                "-nodefaults",
                "-display",
                "none",
                "-daemonize",
                "-no-reboot",
                "-pidfile",
                str(pid_file),
                "-serial",
                f"file:{serial_log}",
                "-drive",
                f"file={image},if=virtio,format=qcow2,cache=none",
                "-drive",
                f"file={seed},if=virtio,format=raw,readonly=on",
                "-device",
                "virtio-net-pci,netdev=net0",
                "-netdev",
                network,
                "-device",
                "virtio-rng-pci",
            ],
            timeout=60,
        )
        if completed.returncode == 0 and pid_file.is_file():
            try:
                pid = int(pid_file.read_text(encoding="ascii").strip())
            except ValueError as error:
                raise NativeUbuntuEvidenceError("QEMU PID is invalid") from error
            if pid > 1 and Path(f"/proc/{pid}").exists():
                return VmHandle(pid, port, image, pid_file, private_key)
        pid_file.unlink(missing_ok=True)
    raise NativeUbuntuEvidenceError("QEMU guest did not start")


def ssh_argv(vm: VmHandle) -> list[str]:
    return [
        "ssh",
        "-q",
        "-o",
        "BatchMode=yes",
        "-o",
        "StrictHostKeyChecking=no",
        "-o",
        "UserKnownHostsFile=/dev/null",
        "-o",
        "ConnectTimeout=5",
        "-i",
        str(vm.private_key),
        "-p",
        str(vm.port),
        "agentmage@127.0.0.1",
    ]


def wait_for_ssh(vm: VmHandle, timeout: int = 240) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        completed = run([*ssh_argv(vm), "/usr/bin/true"], timeout=10)
        if completed.returncode == 0:
            return
        if not Path(f"/proc/{vm.pid}").exists():
            break
        time.sleep(1)
    raise NativeUbuntuEvidenceError("Ubuntu guest SSH did not become available")


def ssh_script(
    vm: VmHandle,
    script: str,
    *,
    timeout: int = 300,
    check: bool = True,
    stage: str = "guest-verification",
) -> str:
    completed = run(
        [*ssh_argv(vm), "/usr/bin/bash", "-s"],
        timeout=timeout,
        input_value=script,
    )
    if check and completed.returncode != 0:
        raise NativeUbuntuEvidenceError(
            f"Ubuntu guest stage failed: {stage} (exit {completed.returncode})"
        )
    return completed.stdout


def scp_to_guest(vm: VmHandle, source: Path, destination: str) -> None:
    completed = run(
        [
            "scp",
            "-q",
            "-o",
            "BatchMode=yes",
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "UserKnownHostsFile=/dev/null",
            "-i",
            str(vm.private_key),
            "-P",
            str(vm.port),
            str(source),
            f"agentmage@127.0.0.1:{destination}",
        ],
        timeout=120,
    )
    if completed.returncode != 0:
        raise NativeUbuntuEvidenceError("test binary transfer failed")


def wait_for_vm_exit(pid: int, timeout: int) -> bool:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if not Path(f"/proc/{pid}").exists():
            return True
        time.sleep(0.25)
    return not Path(f"/proc/{pid}").exists()


def _same_qemu_process(vm: VmHandle) -> bool:
    try:
        command = Path(f"/proc/{vm.pid}/cmdline").read_bytes()
    except OSError:
        return False
    return b"qemu-system-x86_64" in command and os.fsencode(vm.image) in command


def stop_vm(vm: VmHandle) -> dict[str, bool]:
    if Path(f"/proc/{vm.pid}").exists():
        ssh_script(vm, "sudo systemctl poweroff\n", timeout=15, check=False)
        wait_for_vm_exit(vm.pid, 30)
    if Path(f"/proc/{vm.pid}").exists() and _same_qemu_process(vm):
        os.kill(vm.pid, signal.SIGTERM)
        wait_for_vm_exit(vm.pid, 10)
    if Path(f"/proc/{vm.pid}").exists() and _same_qemu_process(vm):
        os.kill(vm.pid, signal.SIGKILL)
        wait_for_vm_exit(vm.pid, 5)
    process_absent = not Path(f"/proc/{vm.pid}").exists()
    vm.pid_file.unlink(missing_ok=True)
    try:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
            listener.bind(("127.0.0.1", vm.port))
        port_released = True
    except OSError:
        port_released = False
    return {
        "qemu_process_absent": process_absent,
        "loopback_ssh_port_released": port_released,
    }


def download_official_image(*, force: bool) -> None:
    CACHE_ROOT.mkdir(parents=True, exist_ok=True, mode=0o700)
    CACHE_ROOT.chmod(0o700)
    if OFFICIAL_IMAGE_PATH.is_file():
        if sha256_file(OFFICIAL_IMAGE_PATH) == OFFICIAL_IMAGE_SHA256:
            return
        if not force:
            raise NativeUbuntuEvidenceError("cached official Ubuntu image digest changed")
        OFFICIAL_IMAGE_PATH.unlink()
    partial = OFFICIAL_IMAGE_PATH.with_suffix(".partial")
    partial.unlink(missing_ok=True)
    try:
        checked(
            [
                "curl",
                "--fail",
                "--location",
                "--proto",
                "=https",
                "--tlsv1.2",
                "--output",
                str(partial),
                OFFICIAL_IMAGE_URL,
            ],
            timeout=1800,
        )
        if sha256_file(partial) != OFFICIAL_IMAGE_SHA256:
            raise NativeUbuntuEvidenceError("official Ubuntu image digest mismatch")
        partial.chmod(0o600)
        os.replace(partial, OFFICIAL_IMAGE_PATH)
    finally:
        partial.unlink(missing_ok=True)


def package_versions(vm: VmHandle) -> list[dict[str, str]]:
    names = " ".join(BOOTSTRAP_PACKAGES)
    output = ssh_script(
        vm,
        "set -eu\n"
        f"dpkg-query -W -f='${{binary:Package}}\\t${{Version}}\\n' {names}\n",
        stage="bootstrap-package-versions",
    )
    observed: dict[str, str] = {}
    for line in output.splitlines():
        fields = line.split("\t", 1)
        if len(fields) == 2:
            observed[fields[0].split(":", 1)[0]] = fields[1]
    if set(observed) != set(BOOTSTRAP_PACKAGES) or any(not value for value in observed.values()):
        raise NativeUbuntuEvidenceError("Ubuntu bootstrap package closure is incomplete")
    return [{"id": name, "version": observed[name]} for name in BOOTSTRAP_PACKAGES]


def load_prepared_metadata() -> dict[str, Any]:
    try:
        metadata = json.loads(PREPARED_METADATA_PATH.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise NativeUbuntuEvidenceError("prepared Ubuntu metadata is unavailable") from error
    if not isinstance(metadata, dict):
        raise NativeUbuntuEvidenceError("prepared Ubuntu metadata is invalid")
    expected_packages = [item.get("id") for item in metadata.get("packages", [])]
    if (
        metadata.get("schema_version") != 1
        or metadata.get("image_url") != OFFICIAL_IMAGE_URL
        or metadata.get("official_image_sha256") != OFFICIAL_IMAGE_SHA256
        or SHA256.fullmatch(str(metadata.get("prepared_image_sha256"))) is None
        or metadata.get("prepared_virtual_size_bytes")
        != PREPARED_VIRTUAL_SIZE_BYTES
        or expected_packages != list(BOOTSTRAP_PACKAGES)
        or any(not item.get("version") for item in metadata.get("packages", []))
        or metadata.get("bootstrap_network") != "qemu-user-network-bootstrap-only"
        or metadata.get("cloud_init_cleaned") is not True
        or metadata.get("bootstrap_ssh_key_retained") is not False
        or metadata.get("private_user_data_used") is not False
    ):
        raise NativeUbuntuEvidenceError("prepared Ubuntu metadata changed")
    if not PREPARED_IMAGE_PATH.is_file():
        raise NativeUbuntuEvidenceError("prepared Ubuntu image is unavailable")
    if (
        not OFFICIAL_IMAGE_PATH.is_file()
        or sha256_file(OFFICIAL_IMAGE_PATH) != OFFICIAL_IMAGE_SHA256
    ):
        raise NativeUbuntuEvidenceError("official Ubuntu backing image is unavailable")
    if sha256_file(PREPARED_IMAGE_PATH) != metadata["prepared_image_sha256"]:
        raise NativeUbuntuEvidenceError("prepared Ubuntu image digest changed")
    return metadata


def bootstrap_image(tools: HostTools, *, force: bool) -> None:
    if PREPARED_IMAGE_PATH.exists() or PREPARED_METADATA_PATH.exists():
        if not force:
            load_prepared_metadata()
            return
        PREPARED_IMAGE_PATH.unlink(missing_ok=True)
        PREPARED_METADATA_PATH.unlink(missing_ok=True)
    download_official_image(force=force)
    CACHE_ROOT.mkdir(parents=True, exist_ok=True, mode=0o700)
    with tempfile.TemporaryDirectory(prefix="agentmage-ubuntu-bootstrap-", dir=CACHE_ROOT) as name:
        temporary = Path(name)
        temporary.chmod(0o700)
        private_key, public_key = generate_ssh_key(temporary)
        seed = create_seed(tools, temporary, public_key, bootstrap=True)
        building = temporary / "prepared.qcow2"
        create_overlay(tools, OFFICIAL_IMAGE_PATH, building)
        checked(
            [
                *tools.qemu_img,
                "resize",
                "-q",
                str(building),
                str(PREPARED_VIRTUAL_SIZE_BYTES),
            ],
            timeout=60,
        )
        vm = start_vm(
            tools,
            building,
            seed,
            private_key,
            temporary,
            restricted_network=False,
        )
        shutdown = {"qemu_process_absent": False, "loopback_ssh_port_released": False}
        try:
            wait_for_ssh(vm)
            ssh_script(
                vm,
                "cloud-init status --wait >/dev/null\n",
                timeout=1800,
                stage="bootstrap-cloud-init",
            )
            ssh_script(
                vm,
                "test -f /var/lib/agentmage-cloud-init-complete\n",
                stage="bootstrap-completion-marker",
            )
            ssh_script(
                vm,
                f"test \"$(id -u)\" = {TEST_UID}\n"
                f"test \"$(id -g)\" = {TEST_GID}\n",
                stage="bootstrap-user-identity",
            )
            packages = package_versions(vm)
            ssh_script(
                vm,
                "set -eu\n"
                "sudo /usr/bin/bash -c '\n"
                "  test -f /home/agentmage/.ssh/authorized_keys\n"
                "  unlink /home/agentmage/.ssh/authorized_keys\n"
                "  cloud-init clean --logs --machine-id\n"
                "  sync\n"
                "  systemctl poweroff\n"
                "'\n",
                timeout=30,
                check=False,
                stage="bootstrap-clean-and-poweroff",
            )
            wait_for_vm_exit(vm.pid, 60)
        finally:
            shutdown = stop_vm(vm)
        if not all(shutdown.values()):
            raise NativeUbuntuEvidenceError("Ubuntu bootstrap guest cleanup failed")
        checked([*tools.qemu_img, "check", "-q", str(building)], timeout=120)
        try:
            image_info = json.loads(
                checked(
                    [*tools.qemu_img, "info", "--output=json", str(building)],
                    timeout=60,
                )
            )
        except json.JSONDecodeError as error:
            raise NativeUbuntuEvidenceError(
                "prepared Ubuntu image metadata is invalid"
            ) from error
        if (
            not isinstance(image_info, dict)
            or image_info.get("format") != "qcow2"
            or image_info.get("virtual-size") != PREPARED_VIRTUAL_SIZE_BYTES
        ):
            raise NativeUbuntuEvidenceError("prepared Ubuntu virtual disk size changed")
        prepared_sha = sha256_file(building)
        building.chmod(0o600)
        os.replace(building, PREPARED_IMAGE_PATH)
        metadata = {
            "schema_version": 1,
            "image_url": OFFICIAL_IMAGE_URL,
            "official_image_sha256": OFFICIAL_IMAGE_SHA256,
            "prepared_image_sha256": prepared_sha,
            "prepared_virtual_size_bytes": PREPARED_VIRTUAL_SIZE_BYTES,
            "packages": packages,
            "bootstrap_network": "qemu-user-network-bootstrap-only",
            "cloud_init_cleaned": True,
            "bootstrap_ssh_key_retained": False,
            "private_user_data_used": False,
            "qemu_launcher_class": tools.launcher_class,
            "qemu_version": tools.qemu_version,
            "qemu_sha256": tools.qemu_sha256,
        }
        write_atomic(PREPARED_METADATA_PATH, pretty_json(metadata), mode=0o600)
    load_prepared_metadata()


def compile_tests(revision: str, directory: Path) -> dict[str, Any]:
    rootless_podman()
    source = directory / "source"
    target = directory / "target"
    source.mkdir()
    target.mkdir()
    committed_source(revision, source)
    image_id = executable_image_id(UBUNTU_BUILD_IMAGE)
    environment = os.environ.copy()
    environment["CARGO_NET_OFFLINE"] = "true"
    completed = run(
        [
            "podman",
            "run",
            "--rm",
            "--network=none",
            "--cap-drop=all",
            "--security-opt=no-new-privileges",
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
            "-p",
            "agentmage-kernel-engine",
            "--no-run",
        ],
        timeout=900,
        environment=environment,
    )
    if completed.returncode != 0:
        raise NativeUbuntuEvidenceError("Ubuntu acceptance test compilation failed")
    binaries: dict[str, Path] = {}
    for binary_id, pattern in (
        ("platform-tests", "agentmage_platform_linux-*"),
        ("kernel-engine-tests", "agentmage_kernel_engine-*"),
    ):
        candidates = sorted(
            path
            for path in (target / "debug" / "deps").glob(pattern)
            if path.is_file() and os.access(path, os.X_OK) and path.suffix == ""
        )
        if len(candidates) != 1:
            raise NativeUbuntuEvidenceError(
                f"compiled Ubuntu test binary closure changed: {binary_id}"
            )
        binaries[binary_id] = candidates[0]
    versions = checked(
        [
            "podman",
            "run",
            "--rm",
            "--network=none",
            "--entrypoint=/usr/bin/bash",
            UBUNTU_BUILD_IMAGE,
            "-c",
            "printf '%s\\n' \"$(rustc --version)\" \"$(cargo --version)\"",
        ],
        timeout=60,
    ).splitlines()
    if len(versions) != 2 or not all(line for line in versions):
        raise NativeUbuntuEvidenceError("Ubuntu build toolchain identity is invalid")
    return {
        "container_image_id": image_id,
        "network_used": False,
        "source_revision": revision,
        "toolchains": {"rustc": versions[0], "cargo": versions[1]},
        "binaries": [
            {
                "id": binary_id,
                "path": path,
                "sha256": sha256_file(path),
            }
            for binary_id, path in binaries.items()
        ],
    }


def parse_test_output(
    output: str,
    expected: tuple[str, ...],
) -> list[dict[str, str]]:
    observed = {
        match.group(1)
        for line in output.splitlines()
        if (match := re.fullmatch(r"test .*::([^: ]+) \.\.\. ok", line))
    }
    if observed != set(expected):
        raise NativeUbuntuEvidenceError("Ubuntu test closure is incomplete")
    return [{"test": name, "status": "pass"} for name in expected]


def observation_test_passed(output: str, marker: str) -> bool:
    """Require one passing test and one exact non-sensitive observation marker."""

    return marker in output and re.search(
        r"test result: ok\. 1 passed; 0 failed; 0 ignored; 0 measured; "
        r"[0-9]+ filtered out; finished in [0-9.]+s",
        output,
    ) is not None


def guest_test(
    vm: VmHandle,
    binary: str,
    filter_name: str,
    expected: tuple[str, ...],
    *,
    ignored: bool = False,
    nocapture: bool = False,
) -> tuple[list[dict[str, str]], str]:
    arguments = [filter_name]
    if ignored:
        arguments.append("--ignored")
    if nocapture:
        arguments.append("--nocapture")
    arguments.append("--test-threads=1")
    command = " ".join([f"/home/agentmage/acceptance/{binary}", *arguments])
    output = ssh_script(
        vm,
        "set -eu\n"
        "export XDG_RUNTIME_DIR=/run/user/10001\n"
        "export DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/10001/bus\n"
        f"{command}\n",
        timeout=360,
        stage=f"test-{filter_name}",
    )
    try:
        records = parse_test_output(output, expected)
    except NativeUbuntuEvidenceError as error:
        raise NativeUbuntuEvidenceError(
            f"Ubuntu test closure is incomplete: {filter_name}"
        ) from error
    return records, output


def guest_observation_test(
    vm: VmHandle,
    filter_name: str,
    marker: str,
) -> None:
    output = ssh_script(
        vm,
        "set -eu\n"
        "export XDG_RUNTIME_DIR=/run/user/10001\n"
        "export DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/10001/bus\n"
        f"/home/agentmage/acceptance/platform-tests {filter_name} "
        "--ignored --nocapture --test-threads=1\n",
        timeout=360,
        stage=f"observation-{filter_name}",
    )
    if not observation_test_passed(output, marker):
        raise NativeUbuntuEvidenceError(
            f"Ubuntu control observation is incomplete: {filter_name}"
        )


def guest_identity(vm: VmHandle) -> dict[str, Any]:
    output = ssh_script(
        vm,
        "python3 - <<'PY'\n"
        "import json, os, platform, subprocess\n"
        "release = {}\n"
        "for line in open('/etc/os-release', encoding='utf-8'):\n"
        "    if '=' in line:\n"
        "        key, value = line.rstrip().split('=', 1)\n"
        "        release[key] = value.strip('\\\"')\n"
        "mountinfo = open('/proc/self/mountinfo', encoding='utf-8').read()\n"
        "print(json.dumps({\n"
        "  'distribution': release.get('ID'),\n"
        "  'version': release.get('VERSION_ID'),\n"
        "  'architecture': platform.machine(),\n"
        "  'kernel_release': platform.release(),\n"
        "  'virtualization': subprocess.check_output(\n"
        "      ['systemd-detect-virt'], text=True).strip(),\n"
        "  'cgroup_filesystem': 'cgroup2' if ' - cgroup2 ' in mountinfo else 'other',\n"
        "  'uid': os.getuid(),\n"
        "  'gid': os.getgid(),\n"
        "}, sort_keys=True))\n"
        "PY\n",
        stage="guest-platform-identity",
    )
    try:
        value = json.loads(output)
    except json.JSONDecodeError as error:
        raise NativeUbuntuEvidenceError("Ubuntu guest identity is invalid") from error
    if (
        value.get("distribution") != "ubuntu"
        or value.get("version") != "26.04"
        or value.get("architecture") != "x86_64"
        or value.get("virtualization") != "kvm"
        or value.get("cgroup_filesystem") != "cgroup2"
        or value.get("uid") != TEST_UID
        or value.get("gid") != TEST_GID
        or KERNEL_RELEASE.fullmatch(str(value.get("kernel_release"))) is None
    ):
        raise NativeUbuntuEvidenceError("Ubuntu KVM guest identity changed")
    return value


def guest_tools(vm: VmHandle) -> list[dict[str, Any]]:
    specifications = json.dumps(TRUSTED_TOOLS)
    output = ssh_script(
        vm,
        "python3 - <<'PY'\n"
        "import hashlib, json, os, re, stat, subprocess\n"
        f"specifications = {specifications}\n"
        "records = []\n"
        "for identity, path, bootstrap_package in specifications:\n"
        "    canonical = os.path.realpath(path)\n"
        "    metadata = os.stat(canonical, follow_symlinks=False)\n"
        "    if (not stat.S_ISREG(metadata.st_mode) or metadata.st_uid != 0\n"
        "            or metadata.st_mode & 0o022 or not metadata.st_mode & 0o111):\n"
        "        raise SystemExit(2)\n"
        "    digest = hashlib.sha256()\n"
        "    with open(canonical, 'rb') as handle:\n"
        "        for block in iter(lambda: handle.read(1024 * 1024), b''):\n"
        "            digest.update(block)\n"
        "    ownership = subprocess.check_output(\n"
        "        ['dpkg-query', '-S', canonical], text=True).splitlines()\n"
        "    if len(ownership) != 1 or ': ' not in ownership[0]:\n"
        "        raise SystemExit(3)\n"
        "    owning_package = ownership[0].split(': ', 1)[0].split(':', 1)[0]\n"
        "    if re.fullmatch(r'[a-z0-9][a-z0-9+.-]{0,127}', owning_package) is None:\n"
        "        raise SystemExit(4)\n"
        "    version = subprocess.check_output(\n"
        "        ['dpkg-query', '-W', '-f=${Version}', owning_package], "
        "text=True).strip()\n"
        "    records.append({\n"
        "        'id': identity,\n"
        "        'declared_path': path,\n"
        "        'canonical_path': canonical,\n"
        "        'bootstrap_package': bootstrap_package,\n"
        "        'owning_package': owning_package,\n"
        "        'version': version,\n"
        "        'owner_uid': metadata.st_uid,\n"
        "        'group_or_world_writable': False,\n"
        "        'sha256': digest.hexdigest(),\n"
        "    })\n"
        "print(json.dumps(records, sort_keys=True))\n"
        "PY\n",
        stage="guest-trusted-tool-identities",
    )
    try:
        records = json.loads(output)
    except json.JSONDecodeError as error:
        raise NativeUbuntuEvidenceError("Ubuntu trusted tool record is invalid") from error
    if not valid_trusted_tools(records):
        raise NativeUbuntuEvidenceError("Ubuntu trusted tool closure changed")
    return records


def initialize_synthetic_keyring(vm: VmHandle) -> None:
    ssh_script(
        vm,
        "set -eu\n"
        "export XDG_RUNTIME_DIR=/run/user/10001\n"
        "export DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/10001/bus\n"
        "systemctl --user stop gnome-keyring-daemon.service "
        "gnome-keyring-daemon.socket 2>/dev/null || true\n"
        "pkill -u 10001 -x gnome-keyring-d 2>/dev/null || true\n"
        "sleep 1\n"
        "if test -d \"$HOME/.local/share/keyrings\"; then\n"
        "  find \"$HOME/.local/share/keyrings\" -mindepth 1 -delete\n"
        "fi\n"
        "install -d -m 700 \"$HOME/.local/share/keyrings\" \"$XDG_RUNTIME_DIR/keyring\"\n"
        "keyring_password=$(od -An -N32 -tx1 /dev/urandom | tr -d ' \\n')\n"
        "printf %s \"$keyring_password\" | gnome-keyring-daemon --daemonize --login "
        "--control-directory=\"$XDG_RUNTIME_DIR/keyring\" >/tmp/agentmage-keyring-env\n"
        "set -a\n"
        ". /tmp/agentmage-keyring-env\n"
        "set +a\n"
        "gnome-keyring-daemon --start --components=secrets "
        ">/tmp/agentmage-keyring-start\n"
        "unset keyring_password\n"
        "unlink /tmp/agentmage-keyring-env\n"
        "unlink /tmp/agentmage-keyring-start\n"
        "busctl --user --no-pager status org.freedesktop.secrets >/dev/null\n",
        timeout=60,
        stage="synthetic-keyring-initialization",
    )


def cleanup_synthetic_keyring(vm: VmHandle) -> dict[str, bool]:
    output = ssh_script(
        vm,
        "set -eu\n"
        "export XDG_RUNTIME_DIR=/run/user/10001\n"
        "export DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/10001/bus\n"
        "matches=$(secret-tool search --all agentmage-schema 1 2>/dev/null || true)\n"
        "test -z \"$matches\"\n"
        "unset matches\n"
        "pkill -u 10001 -x gnome-keyring-d 2>/dev/null || true\n"
        "if test -d \"$HOME/.local/share/keyrings\"; then\n"
        "  find \"$HOME/.local/share/keyrings\" -mindepth 1 -delete\n"
        "fi\n"
        "test -z \"$(find \"$HOME/.local/share/keyrings\" -mindepth 1 -print -quit)\"\n"
        "printf 'agentmage_items_absent=true\\nkeyring_files_absent=true\\n'\n",
        stage="synthetic-keyring-cleanup",
    )
    return {
        "agentmage_items_absent": "agentmage_items_absent=true" in output,
        "synthetic_keyring_files_absent": "keyring_files_absent=true" in output,
    }


def external_network_denied(vm: VmHandle) -> bool:
    output = ssh_script(
        vm,
        "python3 - <<'PY'\n"
        "import socket\n"
        "connection = socket.socket()\n"
        "connection.settimeout(3)\n"
        "result = connection.connect_ex(('1.1.1.1', 443))\n"
        "connection.close()\n"
        "print('denied' if result != 0 else 'connected')\n"
        "PY\n",
        timeout=15,
        stage="external-network-denial",
    ).strip()
    return output == "denied"


def run_native_acceptance(
    revision: str,
    tools: HostTools,
    prepared: dict[str, Any],
) -> dict[str, Any]:
    CACHE_ROOT.mkdir(parents=True, exist_ok=True, mode=0o700)
    result: dict[str, Any] | None = None
    cleanup: dict[str, bool] = {}
    with tempfile.TemporaryDirectory(prefix="agentmage-ubuntu-native-", dir=CACHE_ROOT) as name:
        temporary = Path(name)
        temporary.chmod(0o700)
        build = compile_tests(revision, temporary)
        binaries = {item["id"]: item["path"] for item in build["binaries"]}
        private_key, public_key = generate_ssh_key(temporary)
        seed = create_seed(tools, temporary, public_key, bootstrap=False)
        overlay = temporary / "acceptance.qcow2"
        create_overlay(tools, PREPARED_IMAGE_PATH, overlay)
        vm = start_vm(
            tools,
            overlay,
            seed,
            private_key,
            temporary,
            restricted_network=True,
        )
        vm_cleanup = {"qemu_process_absent": False, "loopback_ssh_port_released": False}
        try:
            wait_for_ssh(vm)
            ssh_script(
                vm,
                "cloud-init status --wait >/dev/null\n",
                timeout=300,
                stage="acceptance-cloud-init",
            )
            ssh_script(
                vm,
                "test -f /var/lib/agentmage-cloud-init-complete\n",
                stage="acceptance-completion-marker",
            )
            ssh_script(
                vm,
                "test \"$(wc -l < \"$HOME/.ssh/authorized_keys\")\" = 1\n",
                stage="acceptance-ephemeral-authorization",
            )
            ssh_script(
                vm,
                f"test \"$(id -u)\" = {TEST_UID}\n"
                f"test \"$(id -g)\" = {TEST_GID}\n",
                stage="acceptance-user-identity",
            )
            ssh_script(
                vm,
                "systemctl --user is-system-running --wait >/dev/null\n",
                stage="acceptance-user-manager",
            )
            identity = guest_identity(vm)
            if not external_network_denied(vm):
                raise NativeUbuntuEvidenceError("Ubuntu acceptance guest reached the Internet")
            trusted_tools = guest_tools(vm)
            for binary_id, path in binaries.items():
                scp_to_guest(vm, path, f"/home/agentmage/acceptance/{binary_id}")
            ssh_script(
                vm,
                "chmod 0500 /home/agentmage/acceptance/platform-tests "
                "/home/agentmage/acceptance/kernel-engine-tests\n",
                stage="acceptance-test-binary-mode",
            )
            initialize_synthetic_keyring(vm)
            sandbox_tests, _ = guest_test(
                vm,
                "platform-tests",
                "sandbox::tests",
                LIVE_SANDBOX_TESTS,
                ignored=True,
            )
            guest_observation_test(
                vm,
                "worker_kernel_status_confirms_no_new_privileges_and_seccomp",
                KERNEL_MARKER,
            )
            guest_observation_test(
                vm,
                "transient_service_terminates_an_unbounded_worker",
                RESOURCE_MARKER,
            )
            ipc_tests, _ = guest_test(
                vm, "platform-tests", "ipc::tests", IPC_TESTS
            )
            secret_tests, _ = guest_test(
                vm,
                "platform-tests",
                "secret_service::tests",
                SECRET_SERVICE_TESTS,
            )
            secret_live_tests, _ = guest_test(
                vm,
                "platform-tests",
                "secret_service::tests::live_",
                SECRET_SERVICE_LIVE_TESTS,
                ignored=True,
            )
            startup_mapping: list[dict[str, str]] = []
            for test in STARTUP_MAPPING_TESTS:
                records, _ = guest_test(vm, "platform-tests", test, (test,))
                startup_mapping.extend(records)
            startup_live, _ = guest_test(
                vm,
                "platform-tests",
                STARTUP_LIVE_TESTS[0],
                STARTUP_LIVE_TESTS,
                ignored=True,
            )
            kernel_startup, _ = guest_test(
                vm,
                "kernel-engine-tests",
                KERNEL_STARTUP_TESTS[0],
                KERNEL_STARTUP_TESTS,
            )
            keyring_cleanup = cleanup_synthetic_keyring(vm)
            if not all(keyring_cleanup.values()):
                raise NativeUbuntuEvidenceError("synthetic Secret Service cleanup failed")
            ssh_script(
                vm,
                "set -eu\n"
                "test -z \"$(systemctl --user --no-legend --state=running "
                "'agentmage-worker-*' 2>/dev/null)\"\n"
                "test -z \"$(find /run/user/10001 -maxdepth 2 -type s "
                "-name 'agentmage*' -print -quit 2>/dev/null)\"\n"
                "unlink /home/agentmage/acceptance/platform-tests\n"
                "unlink /home/agentmage/acceptance/kernel-engine-tests\n",
                stage="acceptance-runtime-residue-cleanup",
            )
            result = {
                "build": {
                    key: value
                    for key, value in build.items()
                    if key != "binaries"
                }
                | {
                    "binaries": [
                        {"id": item["id"], "sha256": item["sha256"]}
                        for item in build["binaries"]
                    ]
                },
                "platform": identity,
                "hypervisor": {
                    "acceleration": "kvm",
                    "launcher_class": tools.launcher_class,
                    "qemu_version": tools.qemu_version,
                    "qemu_sha256": tools.qemu_sha256,
                },
                "network": {
                    "qemu_restrict_mode": True,
                    "host_forward": "loopback-ssh-only",
                    "external_connection_denied": True,
                },
                "trusted_tools": trusted_tools,
                "tests": {
                    "sandbox_live": sandbox_tests,
                    "ipc": ipc_tests,
                    "secret_service_static": secret_tests,
                    "secret_service_live": secret_live_tests,
                    "startup_mapping": startup_mapping,
                    "startup_live": startup_live,
                    "kernel_startup_refusal": kernel_startup,
                },
                "attack_results": [
                    {
                        "attack": attack,
                        "test": test,
                        "ubuntu": "pass-zero-escape-native-kernel",
                    }
                    for attack, test in ATTACK_TESTS.items()
                ],
                "startup_controls": {
                    "control_ids": list(STARTUP_CONTROL_IDS),
                    "mutation_statuses": ["unavailable", "invalid"],
                    "no_degraded_fallback": True,
                },
                "syscall_trace": {
                    "no_new_privileges": 1,
                    "seccomp_mode": 2,
                    "policy_id": "agentmage.linux.worker.deny.v1",
                    "observation_test":
                    "worker_kernel_status_confirms_no_new_privileges_and_seccomp",
                },
                "resource_trace": {
                    "cgroup_filesystem": "cgroup2",
                    "properties": list(CGROUP_PROPERTIES),
                    "runtime_limit_enforced": True,
                    "observation_test": "transient_service_terminates_an_unbounded_worker",
                },
                "secret_service_lifecycle": {
                    "fresh_synthetic_keyring": True,
                    "live_round_trip_count": len(SECRET_SERVICE_LIVE_TESTS),
                    **keyring_cleanup,
                    "host_keyring_touched": False,
                },
                "prepared_image_sha256": prepared["prepared_image_sha256"],
            }
        finally:
            vm_cleanup = stop_vm(vm)
        cleanup = {
            **vm_cleanup,
            "disposable_overlay_removed": False,
            "ephemeral_ssh_key_removed": False,
            "test_binaries_removed_with_temporary_tree": False,
        }
    cleanup.update(
        {
            "disposable_overlay_removed": not overlay.exists(),
            "ephemeral_ssh_key_removed": not private_key.exists()
            and not private_key.with_suffix(".pub").exists(),
            "test_binaries_removed_with_temporary_tree": not temporary.exists(),
        }
    )
    if result is None or not all(cleanup.values()):
        raise NativeUbuntuEvidenceError("native Ubuntu acceptance cleanup failed")
    result["cleanup"] = cleanup
    return result


def valid_test_records(value: Any, expected: tuple[str, ...]) -> bool:
    return (
        isinstance(value, list)
        and [item.get("test") for item in value if isinstance(item, dict)]
        == list(expected)
        and len(value) == len(expected)
        and all(isinstance(item, dict) and item.get("status") == "pass" for item in value)
    )


def valid_trusted_tools(value: Any) -> bool:
    return (
        isinstance(value, list)
        and len(value) == len(TRUSTED_TOOLS)
        and [item.get("id") for item in value if isinstance(item, dict)]
        == [item[0] for item in TRUSTED_TOOLS]
        and all(
            isinstance(item, dict)
            and item.get("declared_path") == specification[1]
            and isinstance(item.get("canonical_path"), str)
            and item["canonical_path"].startswith("/")
            and item.get("bootstrap_package") == specification[2]
            and re.fullmatch(
                r"[a-z0-9][a-z0-9+.-]{0,127}",
                str(item.get("owning_package")),
            )
            is not None
            and isinstance(item.get("version"), str)
            and bool(item["version"])
            and item.get("owner_uid") == 0
            and item.get("group_or_world_writable") is False
            and SHA256.fullmatch(str(item.get("sha256"))) is not None
            for item, specification in zip(value, TRUSTED_TOOLS, strict=True)
        )
    )


def build_report(revision: str, tools: HostTools) -> dict[str, Any]:
    prepared = load_prepared_metadata()
    execution = run_native_acceptance(revision, tools, prepared)
    total_tests = sum(
        len(execution["tests"][name])
        for name in (
            "sandbox_live",
            "ipc",
            "secret_service_static",
            "secret_service_live",
            "startup_mapping",
            "startup_live",
            "kernel_startup_refusal",
        )
    )
    return {
        "schema_version": 1,
        "artifact_id": "linux-native-ubuntu-control-verification",
        "task_ids": list(TASK_IDS),
        "test_ids": list(TEST_IDS),
        "status": "pass-native-ubuntu-kernel-controls",
        "source_revision": revision,
        "sources": source_records(revision),
        "bootstrap": prepared,
        "execution": execution,
        "summary": {
            "native_ubuntu_kernel_controls_verified": True,
            "physical_host_certification": False,
            "test_count": total_tests,
            "attack_class_count": len(ATTACK_TESTS),
            "parity_dimensions_unblocked": 3,
            "cleanup_complete": True,
        },
        "private_values_present": False,
        "macos_evidence_substituted": False,
        "release_claim": "none",
        "limitations": list(LIMITATIONS),
    }


def validate_report(value: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict):
        return ["native Ubuntu control report must be an object"]
    if (
        value.get("schema_version") != 1
        or value.get("artifact_id") != "linux-native-ubuntu-control-verification"
        or value.get("task_ids") != list(TASK_IDS)
        or value.get("test_ids") != list(TEST_IDS)
        or value.get("status") != "pass-native-ubuntu-kernel-controls"
        or REVISION.fullmatch(str(value.get("source_revision"))) is None
    ):
        failures.append("native Ubuntu control report identity changed")
    sources = value.get("sources")
    if (
        not isinstance(sources, list)
        or [item.get("path") for item in sources if isinstance(item, dict)]
        != list(SOURCE_PATHS)
        or len(sources) != len(SOURCE_PATHS)
        or any(SHA256.fullmatch(str(item.get("sha256"))) is None for item in sources)
    ):
        failures.append("native Ubuntu source closure changed")
    bootstrap = value.get("bootstrap")
    package_ids = (
        [item.get("id") for item in bootstrap.get("packages", [])]
        if isinstance(bootstrap, dict)
        else []
    )
    if (
        not isinstance(bootstrap, dict)
        or bootstrap.get("schema_version") != 1
        or bootstrap.get("image_url") != OFFICIAL_IMAGE_URL
        or bootstrap.get("official_image_sha256") != OFFICIAL_IMAGE_SHA256
        or SHA256.fullmatch(str(bootstrap.get("prepared_image_sha256"))) is None
        or bootstrap.get("prepared_virtual_size_bytes")
        != PREPARED_VIRTUAL_SIZE_BYTES
        or package_ids != list(BOOTSTRAP_PACKAGES)
        or any(not item.get("version") for item in bootstrap.get("packages", []))
        or bootstrap.get("bootstrap_network")
        != "qemu-user-network-bootstrap-only"
        or bootstrap.get("cloud_init_cleaned") is not True
        or bootstrap.get("bootstrap_ssh_key_retained") is not False
        or bootstrap.get("private_user_data_used") is not False
        or bootstrap.get("qemu_launcher_class")
        not in {"host-system-path", "fedora-toolbox"}
        or not isinstance(bootstrap.get("qemu_version"), str)
        or not bootstrap.get("qemu_version")
        or SHA256.fullmatch(str(bootstrap.get("qemu_sha256"))) is None
    ):
        failures.append("native Ubuntu bootstrap evidence is incomplete")
    execution = value.get("execution")
    if not isinstance(execution, dict):
        failures.append("native Ubuntu execution evidence is absent")
    else:
        build = execution.get("build", {})
        binaries = build.get("binaries", []) if isinstance(build, dict) else []
        if (
            not isinstance(build, dict)
            or IMAGE_ID.fullmatch(str(build.get("container_image_id"))) is None
            or build.get("network_used") is not False
            or build.get("source_revision") != value.get("source_revision")
            or set(build.get("toolchains", {})) != {"rustc", "cargo"}
            or [item.get("id") for item in binaries]
            != ["platform-tests", "kernel-engine-tests"]
            or any(SHA256.fullmatch(str(item.get("sha256"))) is None for item in binaries)
        ):
            failures.append("native Ubuntu build evidence is incomplete")
        platform_value = execution.get("platform", {})
        if (
            platform_value.get("distribution") != "ubuntu"
            or platform_value.get("version") != "26.04"
            or platform_value.get("architecture") != "x86_64"
            or platform_value.get("virtualization") != "kvm"
            or platform_value.get("cgroup_filesystem") != "cgroup2"
            or platform_value.get("uid") != TEST_UID
            or platform_value.get("gid") != TEST_GID
            or KERNEL_RELEASE.fullmatch(str(platform_value.get("kernel_release"))) is None
        ):
            failures.append("native Ubuntu guest identity changed")
        hypervisor = execution.get("hypervisor", {})
        network = execution.get("network", {})
        if (
            hypervisor.get("acceleration") != "kvm"
            or hypervisor.get("launcher_class")
            not in {"host-system-path", "fedora-toolbox"}
            or not hypervisor.get("qemu_version")
            or SHA256.fullmatch(str(hypervisor.get("qemu_sha256"))) is None
            or network
            != {
                "qemu_restrict_mode": True,
                "host_forward": "loopback-ssh-only",
                "external_connection_denied": True,
            }
        ):
            failures.append("native Ubuntu VM boundary changed")
        if not valid_trusted_tools(execution.get("trusted_tools")):
            failures.append("native Ubuntu trusted tool closure changed")
        tests = execution.get("tests", {})
        expected_test_sets = {
            "sandbox_live": LIVE_SANDBOX_TESTS,
            "ipc": IPC_TESTS,
            "secret_service_static": SECRET_SERVICE_TESTS,
            "secret_service_live": SECRET_SERVICE_LIVE_TESTS,
            "startup_mapping": STARTUP_MAPPING_TESTS,
            "startup_live": STARTUP_LIVE_TESTS,
            "kernel_startup_refusal": KERNEL_STARTUP_TESTS,
        }
        if not isinstance(tests, dict) or any(
            not valid_test_records(tests.get(name), expected)
            for name, expected in expected_test_sets.items()
        ):
            failures.append("native Ubuntu test closure changed")
        expected_attacks = [
            {
                "attack": attack,
                "test": test,
                "ubuntu": "pass-zero-escape-native-kernel",
            }
            for attack, test in ATTACK_TESTS.items()
        ]
        if execution.get("attack_results") != expected_attacks:
            failures.append("native Ubuntu attack closure changed")
        if execution.get("startup_controls") != {
            "control_ids": list(STARTUP_CONTROL_IDS),
            "mutation_statuses": ["unavailable", "invalid"],
            "no_degraded_fallback": True,
        }:
            failures.append("native Ubuntu startup control closure changed")
        if execution.get("syscall_trace") != {
            "no_new_privileges": 1,
            "seccomp_mode": 2,
            "policy_id": "agentmage.linux.worker.deny.v1",
            "observation_test":
            "worker_kernel_status_confirms_no_new_privileges_and_seccomp",
        }:
            failures.append("native Ubuntu syscall trace changed")
        if execution.get("resource_trace") != {
            "cgroup_filesystem": "cgroup2",
            "properties": list(CGROUP_PROPERTIES),
            "runtime_limit_enforced": True,
            "observation_test": "transient_service_terminates_an_unbounded_worker",
        }:
            failures.append("native Ubuntu resource trace changed")
        if execution.get("secret_service_lifecycle") != {
            "fresh_synthetic_keyring": True,
            "live_round_trip_count": len(SECRET_SERVICE_LIVE_TESTS),
            "agentmage_items_absent": True,
            "synthetic_keyring_files_absent": True,
            "host_keyring_touched": False,
        }:
            failures.append("native Ubuntu Secret Service lifecycle changed")
        if (
            not isinstance(bootstrap, dict)
            or execution.get("prepared_image_sha256")
            != bootstrap.get("prepared_image_sha256")
            or execution.get("cleanup")
            != {
                "qemu_process_absent": True,
                "loopback_ssh_port_released": True,
                "disposable_overlay_removed": True,
                "ephemeral_ssh_key_removed": True,
                "test_binaries_removed_with_temporary_tree": True,
            }
        ):
            failures.append("native Ubuntu cleanup or image binding changed")
    if value.get("summary") != {
        "native_ubuntu_kernel_controls_verified": True,
        "physical_host_certification": False,
        "test_count": 30,
        "attack_class_count": len(ATTACK_TESTS),
        "parity_dimensions_unblocked": 3,
        "cleanup_complete": True,
    }:
        failures.append("native Ubuntu summary changed or overclaimed")
    if (
        value.get("private_values_present") is not False
        or value.get("macos_evidence_substituted") is not False
        or value.get("release_claim") != "none"
        or value.get("limitations") != list(LIMITATIONS)
    ):
        failures.append("native Ubuntu report made an unsupported claim")
    return failures


def write_report(revision: str, tools: HostTools) -> None:
    write_atomic(REPORT_PATH, pretty_json(build_report(revision, tools)))


def check_report() -> None:
    try:
        value = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        revision = value["source_revision"]
    except (OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise NativeUbuntuEvidenceError(
            "native Ubuntu control report is unavailable"
        ) from error
    failures = validate_report(value)
    if not isinstance(revision, str):
        failures.append("native Ubuntu source revision is invalid")
    else:
        try:
            expected_sources = source_records(revision)
        except (OSError, UnicodeError, NativeUbuntuEvidenceError) as error:
            failures.append(str(error))
        else:
            if value.get("sources") != expected_sources:
                failures.append("native Ubuntu report is stale")
    if failures:
        raise NativeUbuntuEvidenceError("; ".join(failures))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--bootstrap-image", action="store_true")
    parser.add_argument("--force-bootstrap", action="store_true")
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    parser.add_argument("--toolbox-container", default="fedora-toolbox-44")
    arguments = parser.parse_args()
    try:
        if arguments.force_bootstrap and not arguments.bootstrap_image:
            raise NativeUbuntuEvidenceError(
                "--force-bootstrap requires --bootstrap-image"
            )
        if arguments.bootstrap_image or arguments.write:
            tools = discover_host_tools(arguments.toolbox_container)
        if arguments.bootstrap_image:
            bootstrap_image(tools, force=arguments.force_bootstrap)
        if arguments.write:
            write_report(git_revision(arguments.source_revision), tools)
        if not arguments.bootstrap_image or arguments.write:
            check_report()
    except (
        NativeUbuntuEvidenceError,
        OSError,
        UnicodeError,
        subprocess.SubprocessError,
    ) as error:
        print(f"Native Ubuntu control evidence failed: {error}", file=sys.stderr)
        return 1
    if arguments.bootstrap_image and not arguments.write:
        print("Prepared digest-pinned Ubuntu KVM control image validated")
    else:
        print("Native Ubuntu-kernel control evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
