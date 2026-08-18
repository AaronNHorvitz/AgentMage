#!/usr/bin/env python3
"""Execute Decision 0040 Linux VM acquisition, adapter, and offline phases."""

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
    from scripts.linux_vm_regression import load_catalog, validate_catalog, write_atomic
except ModuleNotFoundError:
    import linux_docker_kvm_evidence as docker_vm
    import linux_native_ubuntu_control_evidence as vm_support
    from linux_vm_regression import load_catalog, validate_catalog, write_atomic


ROOT: Final = Path(__file__).resolve().parents[1]
REPORT_PATH: Final = (
    ROOT / "artifacts/sprints/sprint-9/story-9.1/linux-vm-regression-phases.json"
)
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")
PHASES: Final = ["dependency-acquisition", "connected-adapter", "strict-offline"]
SOURCE_PATHS: Final = (
    "TASKS.md",
    "architecture/linux-vm-base-images.json",
    "docs/decisions/0040-local-platform-validation-and-manual-macos.md",
    "docs/support/linux-vm-regression.md",
    "package.json",
    "scripts/linux_vm_regression.py",
    "scripts/linux_vm_regression_phases.py",
    "tests/test_linux_vm_regression.py",
    "tests/test_linux_vm_regression_phases.py",
)


class LinuxVmPhaseError(ValueError):
    """Raised when a Linux VM phase cannot produce exact evidence."""


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
        raise LinuxVmPhaseError(f"host command failed: {Path(argv[0]).name}")
    return completed.stdout


def exact_clean_revision(value: str) -> str:
    revision = checked(["git", "rev-parse", "--verify", f"{value}^{{commit}}"], timeout=30).strip()
    if REVISION.fullmatch(revision) is None:
        raise LinuxVmPhaseError("Linux VM phase source revision is invalid")
    if checked(["git", "status", "--porcelain", "--untracked-files=all"], timeout=30):
        raise LinuxVmPhaseError("Linux VM phase source worktree is dirty")
    for path in SOURCE_PATHS:
        checked(["git", "cat-file", "-e", f"{revision}:{path}"], timeout=30)
    return revision


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        while block := handle.read(1024 * 1024):
            digest.update(block)
    return digest.hexdigest()


def source_archive(revision: str, destination: Path) -> str:
    with destination.open("wb") as output:
        completed = subprocess.run(
            ["git", "archive", "--format=tar", revision],
            cwd=ROOT,
            stdin=subprocess.DEVNULL,
            stdout=output,
            stderr=subprocess.PIPE,
            timeout=120,
            check=False,
        )
    if completed.returncode != 0:
        raise LinuxVmPhaseError("Linux VM source archive failed")
    return sha256_file(destination)


def guest_hash(vm: vm_support.VmHandle, command: str, stage: str) -> dict[str, Any]:
    output = vm_support.ssh_script(vm, f"set -eu\n{command}\n", stage=stage)
    return {
        "line_count": len(output.splitlines()),
        "sha256": hashlib.sha256(output.encode()).hexdigest(),
    }


def start_guest(
    tools: vm_support.HostTools,
    overlay: Path,
    directory: Path,
    private_key: Path,
    seed: Path,
    *,
    restricted: bool,
) -> vm_support.VmHandle:
    return vm_support.start_vm(
        tools,
        overlay,
        seed,
        private_key,
        directory,
        restricted_network=restricted,
        cpu_count=4,
        memory_mib=8192,
    )


def shutdown(vm: vm_support.VmHandle) -> dict[str, bool]:
    vm_support.ssh_script(
        vm,
        "sudo systemctl poweroff\n",
        timeout=30,
        check=False,
        stage="phase-shutdown",
    )
    vm_support.wait_for_vm_exit(vm.pid, 120)
    return vm_support.stop_vm(vm)


def run_target(
    tools: vm_support.HostTools,
    target: docker_vm.Target,
    revision: str,
    archive: Path,
    archive_sha256: str,
    report_path: Path,
) -> dict[str, Any]:
    prepared = docker_vm.validate_prepared(target)
    with tempfile.TemporaryDirectory(
        prefix=f"agentmage-{target.distribution}-regression-", dir=docker_vm.CACHE_ROOT
    ) as name:
        temporary = Path(name)
        temporary.chmod(0o700)
        overlay = temporary / "regression.qcow2"
        vm_support.create_overlay(tools, target.prepared_path, overlay)
        credential_dir = temporary / "credentials"
        credential_dir.mkdir(mode=0o700)
        private_key, public_key = vm_support.generate_ssh_key(credential_dir)
        seed = docker_vm.create_seed(
            tools, target, credential_dir, public_key, bootstrap=False
        )
        connected_dir = temporary / "connected"
        connected_dir.mkdir(mode=0o700)
        connected = start_guest(
            tools,
            overlay,
            connected_dir,
            private_key,
            seed,
            restricted=False,
        )
        connected_cleanup = {"qemu_process_absent": False, "loopback_ssh_listener_absent": False}
        try:
            vm_support.wait_for_ssh(connected, timeout=300)
            vm_support.ssh_script(
                connected,
                "cloud-init status --wait >/dev/null\n"
                "test \"$(id -u)\" = 10001\n"
                "test \"$(id -g)\" = 10001\n"
                "sudo systemctl is-active --quiet docker.service\n",
                timeout=300,
                stage="connected-startup",
            )
            install = (
                "sudo dnf -y -q install tcpdump >/dev/null"
                if target.distribution == "fedora"
                else "sudo apt-get -qq update >/dev/null && sudo apt-get -qq -y install tcpdump >/dev/null"
            )
            acquisition = guest_hash(connected, install + "\nprintf 'dependency-acquisition-complete\\n'", "dependency-acquisition")
            adapter = guest_hash(
                connected,
                "sudo docker version --format '{{.Server.Version}}'\n"
                "sudo docker info --format '{{.Driver}}'\n"
                "sudo ss -H -lntup\n",
                "connected-adapter",
            )
            vm_support.scp_to_guest(connected, archive, "/home/agentmage/source.tar")
            vm_support.ssh_script(
                connected,
                f"test \"$(sha256sum /home/agentmage/source.tar | cut -d' ' -f1)\" = {archive_sha256}\n",
                stage="source-transfer",
            )
            connected_cleanup = shutdown(connected)
        finally:
            fallback = vm_support.stop_vm(connected)
            connected_cleanup = {
                key: connected_cleanup.get(key, False) or value for key, value in fallback.items()
            }
        if not all(connected_cleanup.values()):
            raise LinuxVmPhaseError("connected Linux VM cleanup failed")

        offline_dir = temporary / "offline"
        offline_dir.mkdir(mode=0o700)
        offline = start_guest(
            tools,
            overlay,
            offline_dir,
            private_key,
            seed,
            restricted=True,
        )
        offline_cleanup = {"qemu_process_absent": False, "loopback_ssh_listener_absent": False}
        try:
            vm_support.wait_for_ssh(offline, timeout=300)
            vm_support.ssh_script(
                offline,
                "cloud-init status --wait >/dev/null\n"
                "test \"$(id -u)\" = 10001\n"
                "test \"$(id -g)\" = 10001\n",
                timeout=300,
                stage="offline-startup",
            )
            if not docker_vm.external_network_denied(offline):
                raise LinuxVmPhaseError("strict-offline guest reached an external peer")
            vm_support.ssh_script(
                offline,
                "set -eu\n"
                "rm -rf /home/agentmage/source\n"
                "mkdir /home/agentmage/source\n"
                "tar -xf /home/agentmage/source.tar -C /home/agentmage/source\n"
                "cd /home/agentmage/source\n"
                "python3 scripts/linux_vm_regression.py\n"
                "python3 -m unittest tests.test_linux_vm_regression tests.test_linux_vm_regression_phases\n",
                timeout=300,
                stage="strict-offline-tests",
            )
            process = guest_hash(offline, "ps -eo uid,pid,ppid,comm --sort=uid,pid", "process-evidence")
            sockets = guest_hash(offline, "sudo ss -H -a -n -p", "socket-evidence")
            packages = docker_vm.package_versions(target, offline)
            packet = guest_hash(
                offline,
                "sudo timeout 3 tcpdump -n -i any -c 1 2>/dev/null || true\n"
                "printf 'strict-offline-probe-complete\\n'",
                "packet-evidence",
            )
            tests = {"command_count": 2, "status": "pass", "network_used": False}
            vm_support.ssh_script(
                offline,
                "rm -rf /home/agentmage/source /home/agentmage/source.tar\n"
                "sudo cloud-init clean --logs --machine-id\n",
                stage="guest-cleanup",
            )
            offline_cleanup = shutdown(offline)
        finally:
            fallback = vm_support.stop_vm(offline)
            offline_cleanup = {
                key: offline_cleanup.get(key, False) or value for key, value in fallback.items()
            }
        if not all(offline_cleanup.values()):
            raise LinuxVmPhaseError("offline Linux VM cleanup failed")
        result = {
            "target_id": target.target_id,
            "source_revision": revision,
            "source_archive_sha256": archive_sha256,
            "prepared_base_sha256": prepared["prepared_sha256"],
            "phases": {
                "dependency-acquisition": acquisition,
                "connected-adapter": adapter,
                "strict-offline": {
                    "external_connection_denied": True,
                    "process": process,
                    "sockets": sockets,
                    "packet": packet,
                    "packages": packages,
                    "tests": tests,
                },
            },
            "guest_cleanup": {
                "connected": connected_cleanup,
                "offline": offline_cleanup,
                "transient_source_absent": True,
            },
            "overlay_cleanup_verified": False,
        }
        partial = build_report(revision, tools, [result], status="pre-cleanup-retained")
        write_atomic(report_path, partial)
        overlay.unlink()
        result["overlay_cleanup_verified"] = not overlay.exists()
        if not result["overlay_cleanup_verified"]:
            raise LinuxVmPhaseError("Linux VM phase overlay cleanup failed")
        return result


def source_records(revision: str) -> list[dict[str, Any]]:
    records = []
    for path in SOURCE_PATHS:
        content = subprocess.run(
            ["git", "show", f"{revision}:{path}"], cwd=ROOT, stdout=subprocess.PIPE, check=True
        ).stdout
        records.append({"path": path, "bytes": len(content), "sha256": hashlib.sha256(content).hexdigest()})
    return records


def build_report(
    revision: str,
    tools: vm_support.HostTools,
    targets: list[dict[str, Any]],
    *,
    status: str = "pass-three-phase-local-kvm",
) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "linux-vm-regression-phase-evidence",
        "task_id": "9.1.4.3",
        "source_revision": revision,
        "status": status,
        "phase_order": PHASES,
        "qemu": {"launcher_class": tools.launcher_class, "version": tools.qemu_version, "sha256": tools.qemu_sha256},
        "targets": targets,
        "repository_credentials_injected": False,
        "private_host_data_retained": False,
        "release_claim": False,
        "sources": source_records(revision),
    }


def validate_report(value: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict):
        return ["Linux VM phase report must be an object"]
    if (
        value.get("schema_version") != 1
        or value.get("record_type") != "linux-vm-regression-phase-evidence"
        or value.get("task_id") != "9.1.4.3"
        or value.get("status") != "pass-three-phase-local-kvm"
        or value.get("phase_order") != PHASES
        or REVISION.fullmatch(str(value.get("source_revision", ""))) is None
    ):
        failures.append("Linux VM phase report identity drifted")
    targets = value.get("targets")
    if not isinstance(targets, list) or [item.get("target_id") for item in targets] != [target.target_id for target in docker_vm.TARGETS]:
        failures.append("Linux VM phase target closure drifted")
        targets = []
    for target in targets:
        phases = target.get("phases", {})
        offline = phases.get("strict-offline", {})
        if (
            list(phases) != PHASES
            or offline.get("external_connection_denied") is not True
            or offline.get("tests") != {"command_count": 2, "status": "pass", "network_used": False}
            or target.get("overlay_cleanup_verified") is not True
            or not all(target.get("guest_cleanup", {}).get("connected", {}).values())
            or not all(target.get("guest_cleanup", {}).get("offline", {}).values())
            or target.get("guest_cleanup", {}).get("transient_source_absent") is not True
        ):
            failures.append(f"Linux VM phase or cleanup drifted: {target.get('target_id')}")
        for section in (phases.get("dependency-acquisition", {}), phases.get("connected-adapter", {}), offline.get("process", {}), offline.get("sockets", {}), offline.get("packet", {})):
            if SHA256.fullmatch(str(section.get("sha256", ""))) is None:
                failures.append(f"Linux VM phase digest is absent: {target.get('target_id')}")
    for field in ("repository_credentials_injected", "private_host_data_retained", "release_claim"):
        if value.get(field) is not False:
            failures.append(f"Linux VM phase prohibited claim changed: {field}")
    return failures


def execute(revision_value: str) -> dict[str, Any]:
    revision = exact_clean_revision(revision_value)
    catalog = load_catalog()
    if failures := validate_catalog(catalog):
        raise LinuxVmPhaseError("; ".join(failures))
    tools = vm_support.discover_host_tools("fedora-toolbox-44")
    with tempfile.TemporaryDirectory(prefix="agentmage-linux-vm-source-") as name:
        archive = Path(name) / "source.tar"
        archive_sha256 = source_archive(revision, archive)
        targets = [run_target(tools, target, revision, archive, archive_sha256, REPORT_PATH) for target in docker_vm.TARGETS]
    report = build_report(revision, tools, targets)
    write_atomic(REPORT_PATH, report)
    return report


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    try:
        if arguments.write:
            report = execute(arguments.source_revision)
        else:
            report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        failures = validate_report(report)
    except (OSError, json.JSONDecodeError, ValueError) as error:
        print(f"Linux VM regression phases failed: {error}", file=sys.stderr)
        return 1
    for failure in failures:
        print(f"Linux VM regression phases failed: {failure}", file=sys.stderr)
    if failures:
        return 1
    print("Linux VM regression phases passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
