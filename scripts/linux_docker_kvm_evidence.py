#!/usr/bin/env python3
"""Prepare and validate clean Fedora/Ubuntu KVM guests for live Docker evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Final

try:
    from scripts import linux_native_ubuntu_control_evidence as vm_support
    from scripts.package_candidate import build_all
except ModuleNotFoundError:  # Direct execution places only scripts/ on sys.path.
    import linux_native_ubuntu_control_evidence as vm_support
    from package_candidate import build_all


ROOT: Final = Path(__file__).resolve().parents[1]
CACHE_ROOT: Final = Path.home() / ".cache/agentmage/docker-kvm"
REPORT_PATH: Final = (
    ROOT
    / "artifacts/sprints/sprint-9/story-9.2/linux-docker-kvm-topology.json"
)
GUEST_PROBE_PATH: Final = ROOT / "scripts/linux_docker_kvm_guest.py"
SOURCE_PATHS: Final = (
    "docs/decisions/0036-observe-network-namespace-through-procfs.md",
    "docs/decisions/0037-admit-canonical-runner-digest-reference.md",
    "docs/support/linux-docker-kvm-evidence.md",
    "package.json",
    "platforms/linux-inference/src/docker_live_collector.rs",
    "platforms/linux-inference/src/docker_linux_observer.rs",
    "scripts/linux_docker_kvm_evidence.py",
    "scripts/linux_docker_kvm_guest.py",
    "scripts/package_candidate.py",
    "tests/test_linux_docker_kvm_evidence.py",
)
LIMITATIONS: Final = [
    "The Fedora and Ubuntu observations run under native distribution kernels in hardware-accelerated disposable KVM guests, not on physical reference hosts.",
    "The held runtime peer proves exact process, cgroup, socket, and guard topology only; the product kernel-to-guard inference path remains unimplemented.",
    "The native adapter is packaged and self-checked but inactive; native llama.cpp execution and all model inference remain assigned to Sprint 13.",
    "No inference, model quality evaluation, private workspace, credential, supported-release claim, or macOS substitution is included.",
]
GUEST_CLEANUP_KEYS: Final = {
    "bootstrap_secret_absent",
    "guard_process_absent",
    "host_raw_listener_absent",
    "runner_absent",
    "runtime_unit_inactive",
}
HOST_CLEANUP_KEYS: Final = {
    "disposable_overlay_absent",
    "loopback_ssh_listener_absent",
    "qemu_process_absent",
}
PRECOLLECTOR_CHECKS: Final = [
    "daemon-peer-pid",
    "daemon-peer-root",
    "docker-socket-group-nonzero",
    "runtime-identity",
    "guard-identity",
    "guard-no-capabilities",
    "guard-no-new-privileges",
    "shared-private-network-namespace",
    "distinct-guard-mount-namespace",
    "guard-parent-owner",
    "guard-parent-group",
    "guard-parent-mode",
    "private-interface-count",
    "private-loopback-up",
    "private-route-count",
    "private-raw-listener-count",
    "private-ipv4-wildcard-count",
    "private-ipv6-wildcard-count",
    "private-non-loopback-listener-count",
    "private-management-listener-count",
    "host-raw-listener-count",
]
RUNNER_DIGEST: Final = (
    "sha256:bd94095bbc1ddc4266c3a88f582a92562c6b63eceb175572c9a60045663727c9"
)
MODEL_DIGEST: Final = (
    "08fa7b1d44f255be48cfc12359211725bfd659742612ed4b221cd5be90d14444"
)
MODEL_FILES: Final = {
    "gemma-4-E4B-it-Q4_K_M.gguf": (
        4_977_171_584,
        "85a896a047553e842f25297ee5b031d64ff30147d9c4af17b1e4b394cd1fab87",
    ),
    "mmproj-F16.gguf": (
        990_372_672,
        "ddf46c21d7078e95338cfc22306b19b276a29a5ad089023449dd54d4b6170a51",
    ),
}
MODEL_SOURCE_ROOT: Final = (
    Path.home()
    / ".local/share/containers/storage/volumes/agentmage-dmr-models-v126/_data"
    / "bundles"
    / "sha256"
    / MODEL_DIGEST
    / "model"
)
VIRTUAL_SIZE_BYTES: Final = 32 * 1024 * 1024 * 1024
TEST_UID: Final = 10001
TEST_GID: Final = 10001
SHA256 = re.compile(r"^[0-9a-f]{64}$")
REVISION = re.compile(r"^[0-9a-f]{40}$")


class DockerKvmEvidenceError(ValueError):
    """Raised when a reusable Docker evidence guest is not exact."""


@dataclass(frozen=True)
class Target:
    """One exact distribution bootstrap specification."""

    target_id: str
    distribution: str
    version: str
    official_url: str
    official_sha256: str
    official_path: Path
    prepared_path: Path
    metadata_path: Path
    packages: tuple[str, ...]
    admin_group: str
    ssh_service: str
    docker_packages: tuple[str, ...]


FEDORA = Target(
    target_id="fedora-44-x86_64",
    distribution="fedora",
    version="44",
    official_url=(
        "https://download.fedoraproject.org/pub/fedora/linux/releases/44/Cloud/"
        "x86_64/images/Fedora-Cloud-Base-Generic-44-1.7.x86_64.qcow2"
    ),
    official_sha256=(
        "28680fe5b371a5a82ebf43a31926e086a168e59949d03969c5093e7071f90b7f"
    ),
    official_path=CACHE_ROOT / "Fedora-Cloud-Base-Generic-44-1.7.x86_64.qcow2",
    prepared_path=CACHE_ROOT / "fedora-44-docker-evidence.qcow2",
    metadata_path=CACHE_ROOT / "fedora-44-docker-evidence.json",
    packages=(
        "ca-certificates",
        "docker-cli",
        "iproute",
        "jq",
        "moby-engine",
        "openssh-server",
        "procps-ng",
        "python3",
        "shadow-utils",
        "sudo",
        "util-linux",
    ),
    admin_group="wheel",
    ssh_service="sshd.service",
    docker_packages=("docker-cli", "moby-engine"),
)

UBUNTU = Target(
    target_id="ubuntu-26.04-x86_64",
    distribution="ubuntu",
    version="26.04",
    official_url=vm_support.OFFICIAL_IMAGE_URL,
    official_sha256=vm_support.OFFICIAL_IMAGE_SHA256,
    official_path=vm_support.OFFICIAL_IMAGE_PATH,
    prepared_path=CACHE_ROOT / "ubuntu-26.04-docker-evidence.qcow2",
    metadata_path=CACHE_ROOT / "ubuntu-26.04-docker-evidence.json",
    packages=(
        "ca-certificates",
        "docker.io",
        "iproute2",
        "jq",
        "openssh-server",
        "procps",
        "python3",
        "sudo",
        "util-linux",
    ),
    admin_group="sudo",
    ssh_service="ssh.service",
    docker_packages=("docker.io",),
)

TARGETS: Final = (FEDORA, UBUNTU)


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        while block := handle.read(1024 * 1024):
            digest.update(block)
    return digest.hexdigest()


def checked(argv: list[str], *, timeout: int = 1800) -> str:
    completed = subprocess.run(
        argv,
        cwd=ROOT,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        timeout=timeout,
        check=False,
    )
    if completed.returncode != 0:
        raise DockerKvmEvidenceError(f"host command failed: {Path(argv[0]).name}")
    return completed.stdout


def git_bytes(revision: str, path: str) -> bytes:
    completed = subprocess.run(
        ["git", "show", f"{revision}:{path}"],
        cwd=ROOT,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=60,
        check=False,
    )
    if completed.returncode != 0 or not completed.stdout:
        raise DockerKvmEvidenceError("committed evidence source is unavailable")
    return completed.stdout


def git_revision(value: str) -> str:
    revision = checked(["git", "rev-parse", "--verify", f"{value}^{{commit}}"], timeout=60).strip()
    if REVISION.fullmatch(revision) is None:
        raise DockerKvmEvidenceError("source revision is invalid")
    for path in SOURCE_PATHS:
        git_bytes(revision, path)
    return revision


def source_records(revision: str) -> list[dict[str, Any]]:
    return [
        {
            "path": path,
            "bytes": len(content := git_bytes(revision, path)),
            "sha256": hashlib.sha256(content).hexdigest(),
        }
        for path in SOURCE_PATHS
    ]


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(prefix=".agentmage-docker-kvm-", dir=path.parent)
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


def validate_model_source() -> list[dict[str, Any]]:
    records = []
    for name, (size, digest) in MODEL_FILES.items():
        path = MODEL_SOURCE_ROOT / name
        if not path.is_file() or path.stat().st_size != size or sha256_file(path) != digest:
            raise DockerKvmEvidenceError("pinned local model input is unavailable")
        records.append({"name": name, "bytes": size, "sha256": digest})
    return records


def download_official(target: Target, *, force: bool) -> None:
    CACHE_ROOT.mkdir(parents=True, exist_ok=True, mode=0o700)
    if target is UBUNTU:
        vm_support.download_official_image(force=force)
        return
    if target.official_path.is_file():
        if sha256_file(target.official_path) == target.official_sha256:
            return
        if not force:
            raise DockerKvmEvidenceError("cached official image digest changed")
        target.official_path.unlink()
    partial = target.official_path.with_suffix(".partial")
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
                target.official_url,
            ]
        )
        if sha256_file(partial) != target.official_sha256:
            raise DockerKvmEvidenceError("official image digest mismatch")
        partial.chmod(0o600)
        os.replace(partial, target.official_path)
    finally:
        partial.unlink(missing_ok=True)


def render_seed(target: Target, public_key: str, *, bootstrap: bool) -> str:
    packages = "\n".join(f"  - {name}" for name in target.packages)
    package_section = (
        f"package_update: true\npackage_upgrade: false\npackages:\n{packages}\n"
        if bootstrap
        else "package_update: false\npackage_upgrade: false\n"
    )
    return (
        "#cloud-config\n"
        f"hostname: agentmage-{target.distribution}-docker\n"
        "manage_etc_hosts: true\n"
        "disable_root: true\n"
        "ssh_pwauth: false\n"
        "users:\n"
        "  - name: agentmage\n"
        f"    uid: {TEST_UID}\n"
        "    shell: /bin/bash\n"
        f"    groups: [{target.admin_group}]\n"
        "    sudo: ALL=(ALL) NOPASSWD:ALL\n"
        "    lock_passwd: true\n"
        "    ssh_authorized_keys:\n"
        f"      - {public_key}\n"
        f"{package_section}"
        "runcmd:\n"
        f"  - [systemctl, enable, --now, {target.ssh_service}]\n"
        "  - [systemctl, enable, --now, docker.service]\n"
        "  - [touch, /var/lib/agentmage-docker-cloud-init-complete]\n"
    )


def create_seed(
    tools: vm_support.HostTools,
    target: Target,
    directory: Path,
    public_key: str,
    *,
    bootstrap: bool,
) -> Path:
    user_data = directory / "user-data"
    meta_data = directory / "meta-data"
    seed = directory / "seed.iso"
    user_data.write_text(render_seed(target, public_key, bootstrap=bootstrap), encoding="ascii")
    meta_data.write_text(
        f"instance-id: agentmage-{target.target_id}-{os.getpid()}\n"
        f"local-hostname: agentmage-{target.distribution}-docker\n",
        encoding="ascii",
    )
    user_data.chmod(0o600)
    meta_data.chmod(0o600)
    vm_support.checked(
        [*tools.cloud_localds, str(seed), str(user_data), str(meta_data)], timeout=60
    )
    return seed


def scp_model(vm: vm_support.VmHandle, source: Path) -> None:
    argv = [
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
        f"agentmage@127.0.0.1:/home/agentmage/{source.name}",
    ]
    checked(argv, timeout=1800)


def package_versions(target: Target, guest: vm_support.VmHandle) -> list[dict[str, str]]:
    if target.distribution == "fedora":
        command = (
            "rpm -q --qf '%{NAME}\\t%{EPOCHNUM}:%{VERSION}-%{RELEASE}\\n' "
            + " ".join(target.packages)
        )
    else:
        command = (
            "dpkg-query -W -f='${binary:Package}\\t${Version}\\n' "
            + " ".join(target.packages)
        )
    output = vm_support.ssh_script(guest, f"set -eu\n{command}\n", stage="package-versions")
    observed: dict[str, str] = {}
    for line in output.splitlines():
        fields = line.split("\t", 1)
        if len(fields) == 2:
            observed[fields[0].split(":", 1)[0]] = fields[1]
    if set(observed) != set(target.packages):
        raise DockerKvmEvidenceError("bootstrap package closure changed")
    return [{"id": name, "version": observed[name]} for name in target.packages]


def bootstrap_target(
    tools: vm_support.HostTools, target: Target, *, force: bool
) -> None:
    if target.prepared_path.exists() or target.metadata_path.exists():
        if not force:
            validate_prepared(target)
            return
        target.prepared_path.unlink(missing_ok=True)
        target.metadata_path.unlink(missing_ok=True)
    download_official(target, force=force)
    model_records = validate_model_source()
    with tempfile.TemporaryDirectory(
        prefix=f"agentmage-{target.distribution}-docker-bootstrap-", dir=CACHE_ROOT
    ) as name:
        temporary = Path(name)
        private_key, public_key = vm_support.generate_ssh_key(temporary)
        seed = create_seed(tools, target, temporary, public_key, bootstrap=True)
        building = temporary / "prepared.qcow2"
        vm_support.create_overlay(tools, target.official_path, building)
        vm_support.checked(
            [*tools.qemu_img, "resize", "-q", str(building), str(VIRTUAL_SIZE_BYTES)],
            timeout=60,
        )
        guest = vm_support.start_vm(
            tools,
            building,
            seed,
            private_key,
            temporary,
            restricted_network=False,
            cpu_count=8,
            memory_mib=8192,
        )
        shutdown = {"qemu_process_absent": False, "loopback_ssh_listener_absent": False}
        try:
            vm_support.wait_for_ssh(guest, timeout=300)
            vm_support.ssh_script(
                guest,
                "cloud-init status --wait >/dev/null\n"
                "test -f /var/lib/agentmage-docker-cloud-init-complete\n"
                "sudo systemctl is-active --quiet docker.service\n",
                timeout=1800,
                stage="bootstrap-cloud-init",
            )
            packages = package_versions(target, guest)
            vm_support.ssh_script(
                guest,
                f"sudo docker pull docker.io/docker/model-runner@{RUNNER_DIGEST}\n"
                "sudo docker volume create agentmage-models >/dev/null\n",
                timeout=1800,
                stage="bootstrap-runner-image",
            )
            for model_name in MODEL_FILES:
                scp_model(guest, MODEL_SOURCE_ROOT / model_name)
            expected = "".join(
                f'test "$(sudo sha256sum "$target/{model_name}" | cut -d" " -f1)" = "{digest}"\n'
                for model_name, (_, digest) in MODEL_FILES.items()
            )
            vm_support.ssh_script(
                guest,
                "set -eu\n"
                "root=$(sudo docker volume inspect agentmage-models --format '{{.Mountpoint}}')\n"
                f"target=$root/bundles/sha256/{MODEL_DIGEST}/model\n"
                "sudo install -d -o root -g root -m 0755 \"$target\"\n"
                + "".join(
                    f"sudo install -o root -g root -m 0444 /home/agentmage/{model_name} \"$target/{model_name}\"\n"
                    f"unlink /home/agentmage/{model_name}\n"
                    for model_name in MODEL_FILES
                )
                + expected
                + "sudo docker image inspect docker.io/docker/model-runner@"
                f"{RUNNER_DIGEST} >/dev/null\n",
                timeout=1800,
                stage="bootstrap-model-verification",
            )
            vm_support.ssh_script(
                guest,
                "set -eu\n"
                "sudo unlink /home/agentmage/.ssh/authorized_keys\n"
                "sudo cloud-init clean --logs --machine-id\n"
                "sync\n"
                "sudo systemctl poweroff\n",
                timeout=30,
                check=False,
                stage="bootstrap-cleanup",
            )
            vm_support.wait_for_vm_exit(guest.pid, 120)
        finally:
            shutdown = vm_support.stop_vm(guest)
        if not all(shutdown.values()):
            raise DockerKvmEvidenceError("bootstrap guest cleanup failed")
        vm_support.checked([*tools.qemu_img, "check", "-q", str(building)], timeout=120)
        prepared_sha = sha256_file(building)
        building.chmod(0o600)
        os.replace(building, target.prepared_path)
        metadata = {
            "schema_version": 1,
            "target_id": target.target_id,
            "distribution": target.distribution,
            "version": target.version,
            "official_url": target.official_url,
            "official_sha256": target.official_sha256,
            "prepared_sha256": prepared_sha,
            "virtual_size_bytes": VIRTUAL_SIZE_BYTES,
            "packages": packages,
            "docker_packages": list(target.docker_packages),
            "runner_digest": RUNNER_DIGEST,
            "model_manifest_digest": f"sha256:{MODEL_DIGEST}",
            "model_files": model_records,
            "bootstrap_network": "qemu-user-network-bootstrap-only",
            "cloud_init_cleaned": True,
            "bootstrap_ssh_key_retained": False,
            "private_values_used": False,
        }
        vm_support.write_atomic(target.metadata_path, vm_support.pretty_json(metadata), mode=0o600)
    validate_prepared(target)


def validate_prepared(target: Target) -> dict[str, Any]:
    try:
        value = json.loads(target.metadata_path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise DockerKvmEvidenceError("prepared metadata is unavailable") from error
    package_ids = [item.get("id") for item in value.get("packages", [])]
    if (
        value.get("schema_version") != 1
        or value.get("target_id") != target.target_id
        or value.get("official_url") != target.official_url
        or value.get("official_sha256") != target.official_sha256
        or value.get("virtual_size_bytes") != VIRTUAL_SIZE_BYTES
        or package_ids != list(target.packages)
        or value.get("docker_packages") != list(target.docker_packages)
        or value.get("runner_digest") != RUNNER_DIGEST
        or value.get("model_manifest_digest") != f"sha256:{MODEL_DIGEST}"
        or value.get("model_files") != validate_model_source()
        or value.get("bootstrap_network") != "qemu-user-network-bootstrap-only"
        or value.get("cloud_init_cleaned") is not True
        or value.get("bootstrap_ssh_key_retained") is not False
        or value.get("private_values_used") is not False
        or SHA256.fullmatch(str(value.get("prepared_sha256"))) is None
    ):
        raise DockerKvmEvidenceError("prepared metadata changed")
    if not target.prepared_path.is_file() or sha256_file(target.prepared_path) != value["prepared_sha256"]:
        raise DockerKvmEvidenceError("prepared image digest changed")
    return value


def build_acceptance_packages(output: Path) -> dict[str, Path]:
    checked(["cargo", "build", "--workspace", "--release", "--locked"], timeout=1800)
    checked(
        ["npm", "run", "build", "--workspace", "@agentmage/vscode-shell"],
        timeout=600,
    )
    packages = build_all(output)
    if sorted(packages) != ["deb", "rpm", "vsix"] or any(
        not path.is_file() or path.stat().st_size <= 0 for path in packages.values()
    ):
        raise DockerKvmEvidenceError("Linux acceptance package build is incomplete")
    return packages


def external_network_denied(guest: vm_support.VmHandle) -> bool:
    output = vm_support.ssh_script(
        guest,
        "python3 - <<'PY'\n"
        "import socket\n"
        "connection = socket.socket()\n"
        "connection.settimeout(3)\n"
        "result = connection.connect_ex(('1.1.1.1', 443))\n"
        "connection.close()\n"
        "print('denied' if result != 0 else 'connected')\n"
        "PY\n",
        timeout=15,
        stage="docker-kvm-external-network-denial",
    )
    return output.strip() == "denied"


def run_target_acceptance(
    tools: vm_support.HostTools,
    target: Target,
    revision: str,
    package: Path,
) -> dict[str, Any]:
    prepared = validate_prepared(target)
    result: dict[str, Any] | None = None
    with tempfile.TemporaryDirectory(
        prefix=f"agentmage-{target.distribution}-docker-acceptance-", dir=CACHE_ROOT
    ) as name:
        temporary = Path(name)
        temporary.chmod(0o700)
        private_key, public_key = vm_support.generate_ssh_key(temporary)
        seed = create_seed(tools, target, temporary, public_key, bootstrap=False)
        overlay = temporary / "acceptance.qcow2"
        vm_support.create_overlay(tools, target.prepared_path, overlay)
        guest = vm_support.start_vm(
            tools,
            overlay,
            seed,
            private_key,
            temporary,
            restricted_network=True,
            cpu_count=32,
            memory_mib=8192,
        )
        vm_cleanup = {"qemu_process_absent": False, "loopback_ssh_listener_absent": False}
        try:
            vm_support.wait_for_ssh(guest, timeout=300)
            vm_support.ssh_script(
                guest,
                "cloud-init status --wait >/dev/null\n"
                "test -f /var/lib/agentmage-docker-cloud-init-complete\n"
                "sudo systemctl is-active --quiet docker.service\n"
                "test \"$(id -u)\" = 10001\n"
                "test \"$(id -g)\" = 10001\n",
                timeout=300,
                stage="docker-kvm-acceptance-startup",
            )
            if not external_network_denied(guest):
                raise DockerKvmEvidenceError("restricted acceptance guest reached the Internet")
            vm_support.scp_to_guest(guest, package, f"/home/agentmage/{package.name}")
            vm_support.scp_to_guest(
                guest, GUEST_PROBE_PATH, "/home/agentmage/linux_docker_kvm_guest.py"
            )
            install = (
                f"sudo rpm -Uvh --replacepkgs --nodeps /home/agentmage/{package.name} >/dev/null"
                if target is FEDORA
                else f"sudo dpkg -i /home/agentmage/{package.name} >/dev/null"
            )
            probe_script = (
                "set -eu\n"
                f"{install}\n"
                "sudo chmod 0500 /home/agentmage/linux_docker_kvm_guest.py\n"
                f"sudo python3 /home/agentmage/linux_docker_kvm_guest.py {revision}\n"
            )
            completed = vm_support.run(
                [*vm_support.ssh_argv(guest), "/usr/bin/bash", "-s"],
                timeout=900,
                input_value=probe_script,
            )
            if completed.returncode != 0:
                diagnostic = completed.stderr.strip().splitlines()
                detail = diagnostic[-1] if diagnostic else "guest probe refused"
                if len(detail) > 240 or re.fullmatch(r"[A-Za-z0-9 ./_:=@()-]*", detail) is None:
                    detail = "guest probe refused"
                raise DockerKvmEvidenceError(f"Docker KVM live topology failed: {detail}")
            output = completed.stdout
            try:
                result = json.loads(output)
            except json.JSONDecodeError as error:
                raise DockerKvmEvidenceError("guest topology output is invalid") from error
            vm_support.ssh_script(
                guest,
                "set -eu\n"
                f"sudo rm -f /home/agentmage/{package.name} "
                "/home/agentmage/linux_docker_kvm_guest.py\n"
                "sudo systemctl poweroff\n",
                timeout=30,
                check=False,
                stage="docker-kvm-acceptance-shutdown",
            )
            vm_support.wait_for_vm_exit(guest.pid, 120)
        finally:
            vm_cleanup = vm_support.stop_vm(guest)
        if result is None or not all(vm_cleanup.values()):
            raise DockerKvmEvidenceError("Docker KVM acceptance cleanup failed")
    if overlay.exists():
        raise DockerKvmEvidenceError("disposable acceptance overlay was retained")
    return {
        "target_id": target.target_id,
        "prepared_image": {
            "official_sha256": prepared["official_sha256"],
            "prepared_sha256": prepared["prepared_sha256"],
            "packages": prepared["packages"],
        },
        "acceptance_package": {
            "filename": package.name,
            "bytes": package.stat().st_size,
            "sha256": sha256_file(package),
        },
        "hypervisor": {
            "acceleration": "kvm",
            "cpu_count": 32,
            "memory_mib": 8192,
            "launcher_class": tools.launcher_class,
            "qemu_version": tools.qemu_version,
            "qemu_sha256": tools.qemu_sha256,
        },
        "network": {
            "qemu_restrict_mode": True,
            "host_forward": "loopback-ssh-only",
            "external_connection_denied": True,
        },
        "observation": result,
        "host_cleanup": vm_cleanup | {"disposable_overlay_absent": True},
    }


def build_report(tools: vm_support.HostTools, revision: str) -> dict[str, Any]:
    with tempfile.TemporaryDirectory(prefix="agentmage-docker-kvm-packages-") as name:
        packages = build_acceptance_packages(Path(name))
        targets = [
            run_target_acceptance(
                tools,
                target,
                revision,
                packages["rpm" if target is FEDORA else "deb"],
            )
            for target in TARGETS
        ]
        return {
            "schema_version": 1,
            "artifact_id": "linux-docker-kvm-topology",
            "source_revision": revision,
            "task_ids": ["9.2.2.1"],
            "status": "pass-live-topology-no-inference",
            "targets": targets,
            "claims": {
                "docker_engine_directly_tested": True,
                "live_topology_inspected": True,
                "native_adapter_live_inference": False,
                "docker_inference_performed": False,
                "model_quality_evaluated": False,
                "release_support": False,
            },
            "limitations": LIMITATIONS,
            "sources": source_records(revision),
        }


def validate_report(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["Docker KVM topology report must be an object"]
    failures: list[str] = []
    if (
        value.get("schema_version") != 1
        or value.get("artifact_id") != "linux-docker-kvm-topology"
        or value.get("task_ids") != ["9.2.2.1"]
        or value.get("status") != "pass-live-topology-no-inference"
        or REVISION.fullmatch(str(value.get("source_revision"))) is None
    ):
        failures.append("Docker KVM topology identity changed")
    targets = value.get("targets")
    if (
        not isinstance(targets, list)
        or any(not isinstance(item, dict) for item in targets)
        or [item.get("target_id") for item in targets] != [
            target.target_id for target in TARGETS
        ]
    ):
        failures.append("Docker KVM target closure changed")
    else:
        for specification, target in zip(TARGETS, targets, strict=True):
            observation = target.get("observation", {})
            if not isinstance(observation, dict):
                failures.append(f"Docker KVM live observation changed: {target.get('target_id')}")
                continue
            docker = observation.get("docker", {})
            collector = docker.get("collector", {})
            native = observation.get("native", {})
            processes = docker.get("processes", {})
            guest_cleanup = observation.get("cleanup", {})
            host_cleanup = target.get("host_cleanup", {})
            if (
                observation.get("platform", {}).get("distribution")
                != specification.distribution
                or target.get("hypervisor", {}).get("acceleration") != "kvm"
                or target.get("hypervisor", {}).get("cpu_count") != 32
                or target.get("network", {}).get("qemu_restrict_mode") is not True
                or target.get("network", {}).get("external_connection_denied") is not True
                or observation.get("platform", {}).get("virtualization") != "kvm"
                or observation.get("platform", {}).get("cgroup_filesystem") != "cgroup2"
                or native.get("mode") != "packaged-inactive-self-check-only"
                or native.get("inference_performed") is not False
                or native.get("host_listener_count_before") != 0
                or native.get("host_listener_count_after") != 0
                or docker.get("runner_image_digest") != RUNNER_DIGEST
                or docker.get("model_manifest_digest") != f"sha256:{MODEL_DIGEST}"
                or docker.get("daemon_configuration", {}).get("listener")
                != "direct-unix-socket"
                or docker.get("daemon_configuration", {}).get("socket_activation_active")
                is not False
                or SHA256.fullmatch(
                    str(docker.get("daemon_configuration", {}).get("service_unit_sha256"))
                )
                is None
                or docker.get("precollector_checks") != PRECOLLECTOR_CHECKS
                or collector.get("record_type") != "agentmage_docker_live_topology_observation"
                or collector.get("admission", {}).get("status") != "admitted"
                or collector.get("source_revision") != value.get("source_revision")
                or set(guest_cleanup) != GUEST_CLEANUP_KEYS
                or any(item is not True for item in guest_cleanup.values())
                or set(host_cleanup) != HOST_CLEANUP_KEYS
                or any(item is not True for item in host_cleanup.values())
            ):
                failures.append(f"Docker KVM live observation changed: {target.get('target_id')}")
                continue
            expected_processes = {
                "daemon": (0, None),
                "runtime_acceptance_peer": (TEST_UID, TEST_GID),
                "guard": (10002, TEST_GID),
                "runner": (None, None),
            }
            for name, (uid, gid) in expected_processes.items():
                record = processes.get(name, {})
                if (
                    not isinstance(record.get("pid"), int)
                    or record.get("pid", 0) <= 1
                    or (uid is not None and record.get("uid") != uid)
                    or (gid is not None and record.get("gid") != gid)
                    or SHA256.fullmatch(str(record.get("executable_sha256"))) is None
                    or SHA256.fullmatch(str(record.get("cgroup_sha256"))) is None
                    or SHA256.fullmatch(str(record.get("network_namespace_sha256"))) is None
                    or SHA256.fullmatch(str(record.get("mount_namespace_sha256"))) is None
                ):
                    failures.append(
                        f"Docker KVM process observation changed: {target.get('target_id')}:{name}"
                    )
    claims = value.get("claims", {})
    if (
        claims.get("docker_engine_directly_tested") is not True
        or claims.get("live_topology_inspected") is not True
        or any(
            claims.get(name) is not False
            for name in (
                "native_adapter_live_inference",
                "docker_inference_performed",
                "model_quality_evaluated",
                "release_support",
            )
        )
    ):
        failures.append("Docker KVM report made an evidence overclaim")
    if value.get("limitations") != LIMITATIONS:
        failures.append("Docker KVM limitations changed")
    sources = value.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(SOURCE_PATHS):
        failures.append("Docker KVM source closure changed")
    elif any(
        not isinstance(item.get("bytes"), int)
        or item.get("bytes", 0) <= 0
        or SHA256.fullmatch(str(item.get("sha256"))) is None
        for item in sources
    ):
        failures.append("Docker KVM source identity is invalid")
    return failures


def check_report() -> list[str]:
    if not REPORT_PATH.is_file():
        return ["Docker KVM topology report is missing"]
    try:
        value = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError):
        return ["Docker KVM topology report is unreadable"]
    failures = validate_report(value)
    if failures:
        return failures
    if value["sources"] != source_records(value["source_revision"]):
        return ["Docker KVM source evidence is stale"]
    return []


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--bootstrap-images", action="store_true")
    parser.add_argument("--force-bootstrap", action="store_true")
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    parser.add_argument("--toolbox-container", default="fedora-toolbox-44")
    arguments = parser.parse_args()
    try:
        if arguments.force_bootstrap and not arguments.bootstrap_images:
            raise DockerKvmEvidenceError("--force-bootstrap requires --bootstrap-images")
        if arguments.bootstrap_images and arguments.write:
            raise DockerKvmEvidenceError("bootstrap and acceptance writes are separate operations")
        tools = vm_support.discover_host_tools(arguments.toolbox_container)
        if arguments.bootstrap_images:
            for target in TARGETS:
                bootstrap_target(tools, target, force=arguments.force_bootstrap)
        else:
            for target in TARGETS:
                validate_prepared(target)
            if arguments.write:
                revision = git_revision(arguments.source_revision)
                report = build_report(tools, revision)
                failures = validate_report(report)
                if failures:
                    raise DockerKvmEvidenceError("; ".join(failures))
                write_atomic(REPORT_PATH, vm_support.pretty_json(report))
            failures = check_report()
            if failures:
                raise DockerKvmEvidenceError("; ".join(failures))
    except (
        DockerKvmEvidenceError,
        vm_support.NativeUbuntuEvidenceError,
        OSError,
        UnicodeError,
        subprocess.SubprocessError,
    ) as error:
        print(f"Linux Docker KVM evidence failed: {error}", file=sys.stderr)
        return 1
    print("Fedora and Ubuntu Docker KVM topology evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
