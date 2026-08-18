#!/usr/bin/env python3
"""Run and verify the complete promoted Linux matrix in disposable KVM guests."""

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
    from scripts import linux_docker_kvm_evidence as docker_vm
    from scripts import linux_native_ubuntu_control_evidence as vm_support
    from scripts.linux_vm_promoted_guest import LANE_ORDER
    from scripts.linux_vm_regression import write_atomic
except ModuleNotFoundError:
    import linux_docker_kvm_evidence as docker_vm
    import linux_native_ubuntu_control_evidence as vm_support
    from linux_vm_promoted_guest import LANE_ORDER
    from linux_vm_regression import write_atomic


ROOT: Final = Path(__file__).resolve().parents[1]
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-9/story-9.1/linux-vm-promoted-matrix.json"
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")
NODE_SHA256: Final = "472655581fb851559730c48763e0c9d3bc25975c59d518003fc0849d3e4ba0f6"
RUSTUP_SHA256: Final = "4acc9acc76d5079515b46346a485974457b5a79893cfb01112423c89aeb5aa10"
SOURCE_PATHS: Final = (
    "TASKS.md",
    "architecture/clean-build-policy.json",
    "architecture/linux-vm-base-images.json",
    "docs/decisions/0040-local-platform-validation-and-manual-macos.md",
    "docs/support/linux-vm-regression.md",
    "package.json",
    "scripts/linux_vm_promoted_guest.py",
    "scripts/linux_vm_promoted_matrix.py",
    "tests/test_linux_vm_promoted_matrix.py",
)


class PromotedMatrixError(ValueError):
    """Raised when promoted Linux KVM evidence cannot be produced."""


def checked(argv: list[str], *, timeout: int = 300) -> str:
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
        raise PromotedMatrixError(f"host command failed: {Path(argv[0]).name}")
    return completed.stdout


def clean_revision(value: str) -> str:
    revision = checked(["git", "rev-parse", "--verify", f"{value}^{{commit}}"], timeout=30).strip()
    if REVISION.fullmatch(revision) is None:
        raise PromotedMatrixError("promoted matrix source revision is invalid")
    if checked(["git", "status", "--porcelain", "--untracked-files=all"], timeout=30):
        raise PromotedMatrixError("promoted matrix source worktree is dirty")
    for path in SOURCE_PATHS:
        checked(["git", "cat-file", "-e", f"{revision}:{path}"], timeout=30)
    return revision


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        while block := stream.read(1024 * 1024):
            digest.update(block)
    return digest.hexdigest()


def source_bundle(revision: str, destination: Path) -> str:
    completed = subprocess.run(
        ["git", "bundle", "create", str(destination), "HEAD"],
        cwd=ROOT,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=120,
        check=False,
    )
    if completed.returncode != 0 or not destination.is_file():
        raise PromotedMatrixError("promoted matrix source bundle failed")
    checked(["git", "bundle", "verify", str(destination)], timeout=120)
    return sha256_file(destination)


def scp_from_guest(vm: vm_support.VmHandle, source: str, destination: Path) -> None:
    checked(
        [
            "scp", "-q", "-o", "BatchMode=yes", "-o", "StrictHostKeyChecking=no",
            "-o", "UserKnownHostsFile=/dev/null", "-i", str(vm.private_key), "-P", str(vm.port),
            f"agentmage@127.0.0.1:{source}", str(destination),
        ],
        timeout=300,
    )


def bounded_diagnostic(output: str) -> str:
    sanitized = output.replace("/home/agentmage/source", "<GUEST_SOURCE>")
    sanitized = sanitized.replace("/home/agentmage", "<GUEST_HOME>")
    lines = [line[-240:] for line in sanitized.splitlines()[-12:]]
    return " | ".join(lines)[:2400] or "no-output"


def acquisition_script(vm: vm_support.VmHandle, script: str) -> None:
    completed = vm_support.run(
        [*vm_support.ssh_argv(vm), "/usr/bin/bash", "-s"],
        timeout=7200,
        input_value=script,
    )
    if completed.returncode != 0:
        raise PromotedMatrixError(
            "promoted dependency acquisition failed: "
            + bounded_diagnostic(completed.stdout + "\n" + completed.stderr)
        )


def install_script(target: docker_vm.Target) -> str:
    if target.distribution == "fedora":
        packages = (
            "bash bubblewrap ca-certificates cpio curl gcc gcc-c++ git glibc-devel gnupg2 "
            "gnupg2-gpgconf libsecret make openssl-devel python3 rpm-build systemd xz "
            "atk cups-libs dbus-libs mesa-libgbm glib2 gtk3 nspr nss "
            "libX11-xcb libXcomposite libXdamage libXfixes libXrandr libxkbcommon"
        )
        install = f"sudo dnf -y -q install {packages} >/dev/null"
    else:
        packages = (
            "bash bubblewrap build-essential ca-certificates cpio curl git gnupg libsecret-tools "
            "libssl-dev python3 rpm systemd xz-utils libasound2t64 libatk-bridge2.0-0 "
            "libatk1.0-0 libcups2 libdbus-1-3 libdrm2 libgbm1 libglib2.0-0t64 "
            "libgtk-3-0t64 libnspr4 libnss3 libx11-xcb1 libxcomposite1 libxdamage1 "
            "libxfixes3 libxkbcommon0 libxrandr2"
        )
        install = (
            "sudo apt-get -qq update >/dev/null && "
            f"sudo DEBIAN_FRONTEND=noninteractive apt-get -qq -y install {packages} >/dev/null"
        )
    return f"""set -eu
{install}
sudo install -d -m 0755 /opt/node /opt/cargo /opt/rustup
curl --fail --location --silent --show-error https://nodejs.org/dist/v24.15.0/node-v24.15.0-linux-x64.tar.xz -o /tmp/node.tar.xz
echo '{NODE_SHA256}  /tmp/node.tar.xz' | sha256sum --check --strict
sudo tar -xJf /tmp/node.tar.xz -C /opt/node --strip-components=1
sudo env PATH=/opt/node/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
  /opt/node/bin/npm install --global --ignore-scripts --no-audit --no-fund npm@11.12.1 >/dev/null
curl --fail --location --silent --show-error https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init -o /tmp/rustup-init
echo '{RUSTUP_SHA256}  /tmp/rustup-init' | sha256sum --check --strict
chmod 0755 /tmp/rustup-init
sudo env CARGO_HOME=/opt/cargo RUSTUP_HOME=/opt/rustup /tmp/rustup-init -y --no-modify-path --profile minimal --default-toolchain 1.95.0 --component clippy,rustfmt >/dev/null
sudo chown -R 10001:10001 /opt/cargo
rm -f /tmp/node.tar.xz /tmp/rustup-init
export PATH=/opt/node/bin:/opt/cargo/bin:$PATH CARGO_HOME=/opt/cargo RUSTUP_HOME=/opt/rustup RUSTUP_NO_UPDATE_CHECK=1
test "$(node --version)" = v24.15.0
test "$(npm --version)" = 11.12.1
test "$(rustc --version | cut -d' ' -f2)" = 1.95.0
cd /home/agentmage/source
npm ci --ignore-scripts --no-audit --no-fund
cargo fetch --locked
npx --no-install puppeteer browsers install chrome-headless-shell
"""


def start_guest(
    tools: vm_support.HostTools,
    overlay: Path,
    seed: Path,
    key: Path,
    directory: Path,
    *,
    restricted: bool,
) -> vm_support.VmHandle:
    return vm_support.start_vm(
        tools, overlay, seed, key, directory,
        restricted_network=restricted, cpu_count=8, memory_mib=12288,
    )


def shutdown(vm: vm_support.VmHandle) -> dict[str, bool]:
    vm_support.ssh_script(vm, "sudo systemctl poweroff\n", timeout=30, check=False, stage="matrix-shutdown")
    vm_support.wait_for_vm_exit(vm.pid, 120)
    return vm_support.stop_vm(vm)


def run_target(
    tools: vm_support.HostTools,
    target: docker_vm.Target,
    revision: str,
    bundle: Path,
    bundle_sha256: str,
    native_archive: Path,
) -> dict[str, Any]:
    prepared = docker_vm.validate_prepared(target)
    with tempfile.TemporaryDirectory(
        prefix=f"agentmage-{target.distribution}-promoted-", dir=docker_vm.CACHE_ROOT
    ) as name:
        temporary = Path(name)
        temporary.chmod(0o700)
        overlay = temporary / "matrix.qcow2"
        vm_support.create_overlay(tools, target.prepared_path, overlay)
        credential_dir = temporary / "credentials"
        credential_dir.mkdir(mode=0o700)
        private_key, public_key = vm_support.generate_ssh_key(credential_dir)
        seed = docker_vm.create_seed(tools, target, credential_dir, public_key, bootstrap=False)

        connected_dir = temporary / "connected"
        connected_dir.mkdir(mode=0o700)
        connected = start_guest(tools, overlay, seed, private_key, connected_dir, restricted=False)
        connected_cleanup = {"qemu_process_absent": False, "loopback_ssh_listener_absent": False}
        try:
            vm_support.wait_for_ssh(connected, timeout=300)
            vm_support.ssh_script(
                connected,
                "cloud-init status --wait >/dev/null\n"
                "test \"$(id -u)\" = 10001\n"
                "rm -rf /home/agentmage/source\n",
                timeout=300,
                stage="matrix-connected-startup",
            )
            vm_support.scp_to_guest(connected, bundle, "/home/agentmage/source.bundle")
            vm_support.scp_to_guest(connected, native_archive, "/home/agentmage/native-runtime.tar.gz")
            acquisition_script(
                connected,
                f"test \"$(sha256sum /home/agentmage/source.bundle | cut -d' ' -f1)\" = {bundle_sha256}\n"
                f"test \"$(sha256sum /home/agentmage/native-runtime.tar.gz | cut -d' ' -f1)\" = {sha256_file(native_archive)}\n"
                "git clone --quiet /home/agentmage/source.bundle /home/agentmage/source\n"
                f"git -C /home/agentmage/source checkout --quiet --detach {revision}\n"
                f"test \"$(git -C /home/agentmage/source rev-parse HEAD)\" = {revision}\n"
                "test -z \"$(git -C /home/agentmage/source status --porcelain --untracked-files=all)\"\n"
                + install_script(target),
            )
            connected_cleanup = shutdown(connected)
        finally:
            fallback = vm_support.stop_vm(connected)
            connected_cleanup = {key: connected_cleanup.get(key, False) or value for key, value in fallback.items()}
        if not all(connected_cleanup.values()):
            raise PromotedMatrixError("promoted connected guest cleanup failed")

        offline_dir = temporary / "offline"
        offline_dir.mkdir(mode=0o700)
        offline = start_guest(tools, overlay, seed, private_key, offline_dir, restricted=True)
        offline_cleanup = {"qemu_process_absent": False, "loopback_ssh_listener_absent": False}
        result_path = temporary / "guest-result.json"
        try:
            vm_support.wait_for_ssh(offline, timeout=300)
            if not docker_vm.external_network_denied(offline):
                raise PromotedMatrixError("promoted guest reached an external peer")
            vm_support.ssh_script(
                offline,
                "set -eu\n"
                "export PATH=/opt/node/bin:/opt/cargo/bin:$PATH CARGO_HOME=/opt/cargo RUSTUP_HOME=/opt/rustup RUSTUP_NO_UPDATE_CHECK=1\n"
                "cd /home/agentmage/source\n"
                f"python3 scripts/linux_vm_promoted_guest.py --distribution {target.distribution} "
                "--native-archive /home/agentmage/native-runtime.tar.gz --output /home/agentmage/promoted-result.json\n",
                timeout=14400,
                stage="matrix-offline-execution",
            )
            scp_from_guest(offline, "/home/agentmage/promoted-result.json", result_path)
            guest_result = json.loads(result_path.read_text(encoding="utf-8"))
            process_sha = hashlib.sha256(
                vm_support.ssh_script(offline, "ps -eo uid,pid,ppid,comm --sort=uid,pid\n", stage="matrix-processes").encode()
            ).hexdigest()
            socket_sha = hashlib.sha256(
                vm_support.ssh_script(offline, "sudo ss -H -a -n -p\n", stage="matrix-sockets").encode()
            ).hexdigest()
            packages = docker_vm.package_versions(target, offline)
            vm_support.ssh_script(
                offline,
                "rm -rf /home/agentmage/source /home/agentmage/source.bundle "
                "/home/agentmage/native-runtime.tar.gz /home/agentmage/promoted-result.json\n"
                "sudo cloud-init clean --logs --machine-id\n",
                stage="matrix-guest-cleanup",
            )
            offline_cleanup = shutdown(offline)
        finally:
            fallback = vm_support.stop_vm(offline)
            offline_cleanup = {key: offline_cleanup.get(key, False) or value for key, value in fallback.items()}
        if not all(offline_cleanup.values()):
            raise PromotedMatrixError("promoted offline guest cleanup failed")
        overlay.unlink()
        if overlay.exists():
            raise PromotedMatrixError("promoted guest overlay cleanup failed")
        return {
            "target_id": target.target_id,
            "source_revision": revision,
            "source_bundle_sha256": bundle_sha256,
            "prepared_base_sha256": prepared["prepared_sha256"],
            "strict_offline": True,
            "guest_result": guest_result,
            "observation_sha256": {"processes": process_sha, "sockets": socket_sha},
            "packages": packages,
            "cleanup": {
                "connected": connected_cleanup,
                "offline": offline_cleanup,
                "overlay_absent": True,
                "transient_source_absent": True,
                "credential_material_absent": True,
            },
        }


def source_records(revision: str) -> list[dict[str, Any]]:
    records = []
    for path in SOURCE_PATHS:
        content = subprocess.run(
            ["git", "show", f"{revision}:{path}"], cwd=ROOT, stdout=subprocess.PIPE, check=True
        ).stdout
        records.append({"path": path, "bytes": len(content), "sha256": hashlib.sha256(content).hexdigest()})
    return records


def build_report(revision: str, tools: vm_support.HostTools, targets: list[dict[str, Any]]) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "linux-vm-promoted-matrix-evidence",
        "task_id": "9.1.4.4",
        "source_revision": revision,
        "status": "pass-independent-fedora-ubuntu-matrices",
        "lane_order": list(LANE_ORDER),
        "qemu": {"launcher_class": tools.launcher_class, "version": tools.qemu_version, "sha256": tools.qemu_sha256},
        "targets": targets,
        "target_results_interchangeable": False,
        "adapter_results_interchangeable": False,
        "repository_credentials_injected": False,
        "private_host_data_retained": False,
        "model_inference_claim": False,
        "human_accessibility_review_claim": False,
        "physical_host_claim": False,
        "release_claim": False,
        "sources": source_records(revision),
    }


def validate_report(value: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict):
        return ["promoted Linux matrix must be an object"]
    if (
        value.get("schema_version") != 1
        or value.get("record_type") != "linux-vm-promoted-matrix-evidence"
        or value.get("task_id") != "9.1.4.4"
        or value.get("status") != "pass-independent-fedora-ubuntu-matrices"
        or value.get("lane_order") != list(LANE_ORDER)
        or REVISION.fullmatch(str(value.get("source_revision", ""))) is None
    ):
        failures.append("promoted Linux matrix identity drifted")
    targets = value.get("targets")
    expected_targets = [target.target_id for target in docker_vm.TARGETS]
    if not isinstance(targets, list) or [item.get("target_id") for item in targets] != expected_targets:
        failures.append("promoted Linux target closure drifted")
        targets = []
    source_revisions: set[str] = set()
    for target in targets:
        guest = target.get("guest_result", {})
        lanes = guest.get("lanes", [])
        source_revisions.add(str(target.get("source_revision")))
        expected_distribution = "fedora" if target.get("target_id") == "fedora-44-x86_64" else "ubuntu"
        if (
            target.get("strict_offline") is not True
            or guest.get("status") != "pass"
            or guest.get("distribution") != expected_distribution
            or guest.get("network_used") is not False
            or [item.get("id") for item in lanes] != list(LANE_ORDER)
            or any(item.get("status") != "pass" for item in lanes)
            or guest.get("native_and_docker_results_distinct") is not True
            or guest.get("model_inference_executed") is not False
            or guest.get("human_accessibility_review_claim") is not False
            or guest.get("release_claim") is not False
        ):
            failures.append(f"promoted guest result drifted: {target.get('target_id')}")
        receipts = guest.get("command_receipts")
        if (
            not isinstance(receipts, list)
            or not receipts
            or any(
                receipt.get("lane") not in LANE_ORDER
                or receipt.get("observed_exit") != receipt.get("expected_exit")
                or SHA256.fullmatch(str(receipt.get("output_sha256", ""))) is None
                for receipt in receipts
            )
        ):
            failures.append(f"promoted guest command receipts drifted: {target.get('target_id')}")
        lane_map = {item.get("id"): item for item in lanes}
        if (
            lane_map.get("native-runtime", {}).get("adapter") != "native-llama-cpp"
            or lane_map.get("docker-compatibility", {}).get("adapter") != "docker-model-runner-compatibility"
            or lane_map.get("docker-compatibility", {}).get("separate_from_native") is not True
            or lane_map.get("accessibility", {}).get("human_review") is not False
        ):
            failures.append(f"promoted adapter or accessibility scope drifted: {target.get('target_id')}")
        cleanup = target.get("cleanup", {})
        if (
            not all(cleanup.get("connected", {}).values())
            or not all(cleanup.get("offline", {}).values())
            or any(cleanup.get(key) is not True for key in ("overlay_absent", "transient_source_absent", "credential_material_absent"))
            or any(SHA256.fullmatch(str(item)) is None for item in target.get("observation_sha256", {}).values())
        ):
            failures.append(f"promoted guest cleanup drifted: {target.get('target_id')}")
    if source_revisions and source_revisions != {value.get("source_revision")}:
        failures.append("promoted guest source revisions are not exact")
    for field in (
        "target_results_interchangeable", "adapter_results_interchangeable", "repository_credentials_injected",
        "private_host_data_retained", "model_inference_claim", "human_accessibility_review_claim",
        "physical_host_claim", "release_claim",
    ):
        if value.get(field) is not False:
            failures.append(f"promoted Linux prohibited claim changed: {field}")
    return failures


def execute(revision_value: str, native_archive: Path) -> dict[str, Any]:
    revision = clean_revision(revision_value)
    if not native_archive.is_file() or sha256_file(native_archive) != "f14e312fbee33ce60d2eed7036de5debe31c1d7f4d8f0e37920eb0a2de0854a5":
        raise PromotedMatrixError("exact native runtime input is unavailable")
    tools = vm_support.discover_host_tools("fedora-toolbox-44")
    with tempfile.TemporaryDirectory(prefix="agentmage-promoted-source-") as name:
        bundle = Path(name) / "source.bundle"
        bundle_sha256 = source_bundle(revision, bundle)
        targets = [
            run_target(tools, target, revision, bundle, bundle_sha256, native_archive)
            for target in docker_vm.TARGETS
        ]
    report = build_report(revision, tools, targets)
    write_atomic(REPORT_PATH, report)
    return report


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    parser.add_argument("--native-archive", type=Path)
    arguments = parser.parse_args()
    try:
        if arguments.write:
            if arguments.native_archive is None:
                raise PromotedMatrixError("--native-archive is required with --write")
            report = execute(arguments.source_revision, arguments.native_archive.resolve())
        else:
            report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        failures = validate_report(report)
    except (OSError, ValueError, json.JSONDecodeError, subprocess.SubprocessError) as error:
        print(f"promoted Linux VM matrix failed: {error}", file=sys.stderr)
        return 1
    for failure in failures:
        print(f"promoted Linux VM matrix failed: {failure}", file=sys.stderr)
    if failures:
        return 1
    print("promoted Linux VM matrix passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
