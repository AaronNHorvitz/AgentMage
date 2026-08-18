#!/usr/bin/env python3
"""Build and verify installed Sprint 16 workers in disposable Linux KVM guests."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Final

try:
    from scripts import linux_docker_kvm_evidence as docker_vm
    from scripts import linux_native_ubuntu_control_evidence as vm_support
    from scripts import linux_vm_promoted_matrix as promoted
    from scripts.linux_vm_regression import write_atomic
except ModuleNotFoundError:
    import linux_docker_kvm_evidence as docker_vm
    import linux_native_ubuntu_control_evidence as vm_support
    import linux_vm_promoted_matrix as promoted
    from linux_vm_regression import write_atomic


ROOT: Final = Path(__file__).resolve().parents[1]
REPORT_PATH: Final = (
    ROOT / "artifacts/sprints/sprint-16/installed-linux-worker-matrix.json"
)
SOURCE_PATHS: Final = (
    "capabilities/read-only/src/bin/agentmage-read-only-worker.rs",
    "docs/architecture/read-only-tool-protocol.md",
    "packaging/linux/agentmage.spec.in",
    "packaging/linux/debian-control.in",
    "scripts/package_candidate.py",
    "scripts/sprint_16_linux_worker_evidence.py",
    "scripts/sprint_16_linux_worker_guest.py",
    "shells/host/src/linux_read.rs",
    "shells/host/src/package_verify.rs",
    "tests/test_sprint_16_linux_worker_evidence.py",
)
SHA256: Final = promoted.SHA256
REVISION: Final = promoted.REVISION


class WorkerEvidenceError(ValueError):
    """Raised when installed-worker VM evidence cannot be produced."""


def source_records(revision: str) -> list[dict[str, Any]]:
    records = []
    for path in SOURCE_PATHS:
        content = subprocess.run(
            ["git", "show", f"{revision}:{path}"],
            cwd=ROOT,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=True,
        ).stdout
        records.append(
            {
                "path": path,
                "bytes": len(content),
                "sha256": hashlib.sha256(content).hexdigest(),
            }
        )
    return records


def shutdown(vm: vm_support.VmHandle) -> dict[str, bool]:
    vm_support.ssh_script(
        vm,
        "sudo systemctl poweroff\n",
        timeout=30,
        check=False,
        stage="sprint16-shutdown",
    )
    vm_support.wait_for_vm_exit(vm.pid, 120)
    return vm_support.stop_vm(vm)


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
        tools,
        overlay,
        seed,
        key,
        directory,
        restricted_network=restricted,
        cpu_count=8,
        memory_mib=12288,
    )


def merge_cleanup(primary: dict[str, bool], fallback: dict[str, bool]) -> dict[str, bool]:
    return {
        key: primary.get(key, False) or fallback.get(key, False)
        for key in set(primary) | set(fallback)
    }


def run_target(
    tools: vm_support.HostTools,
    target: docker_vm.Target,
    revision: str,
    bundle: Path,
    bundle_sha256: str,
) -> dict[str, Any]:
    prepared = docker_vm.validate_prepared(target)
    with tempfile.TemporaryDirectory(
        prefix=f"agentmage-{target.distribution}-sprint16-",
        dir=docker_vm.CACHE_ROOT,
    ) as name:
        temporary = Path(name)
        temporary.chmod(0o700)
        overlay = temporary / "worker.qcow2"
        vm_support.create_overlay(tools, target.prepared_path, overlay)
        credentials = temporary / "credentials"
        credentials.mkdir(mode=0o700)
        private_key, public_key = vm_support.generate_ssh_key(credentials)
        seed = docker_vm.create_seed(tools, target, credentials, public_key, bootstrap=False)

        connected_directory = temporary / "connected"
        connected_directory.mkdir(mode=0o700)
        connected = start_guest(
            tools,
            overlay,
            seed,
            private_key,
            connected_directory,
            restricted=False,
        )
        connected_cleanup = {
            "qemu_process_absent": False,
            "loopback_ssh_listener_absent": False,
        }
        try:
            vm_support.wait_for_ssh(connected, timeout=300)
            vm_support.ssh_script(
                connected,
                "cloud-init status --wait >/dev/null\n"
                "test \"$(id -u)\" = 10001\n"
                "rm -rf /home/agentmage/source\n",
                timeout=300,
                stage="sprint16-connected-startup",
            )
            vm_support.scp_to_guest(connected, bundle, "/home/agentmage/source.bundle")
            promoted.diagnostic_guest_script(
                connected,
                "set -eu\n"
                f"test \"$(sha256sum /home/agentmage/source.bundle | cut -d' ' -f1)\" = {bundle_sha256}\n"
                + promoted.system_dependencies_script(target)
                + "\n"
                "mkdir /home/agentmage/source\n"
                "git -C /home/agentmage/source init --quiet\n"
                "git -C /home/agentmage/source fetch --quiet /home/agentmage/source.bundle HEAD\n"
                "git -C /home/agentmage/source checkout --quiet --detach FETCH_HEAD\n"
                f"test \"$(git -C /home/agentmage/source rev-parse HEAD)\" = {revision}\n"
                "test -z \"$(git -C /home/agentmage/source status --porcelain --untracked-files=all)\"\n"
                + promoted.toolchain_dependencies_script(),
                stage="sprint16-dependency-acquisition",
                timeout=7200,
            )
            connected_cleanup = shutdown(connected)
        finally:
            connected_cleanup = merge_cleanup(
                connected_cleanup, vm_support.stop_vm(connected)
            )
        if not all(connected_cleanup.values()):
            raise WorkerEvidenceError("connected guest cleanup failed")

        offline_directory = temporary / "offline"
        offline_directory.mkdir(mode=0o700)
        offline = start_guest(
            tools,
            overlay,
            seed,
            private_key,
            offline_directory,
            restricted=True,
        )
        offline_cleanup = {
            "qemu_process_absent": False,
            "loopback_ssh_listener_absent": False,
        }
        guest_result_path = temporary / "guest-result.json"
        try:
            vm_support.wait_for_ssh(offline, timeout=300)
            if not docker_vm.external_network_denied(offline):
                raise WorkerEvidenceError("offline guest reached an external peer")
            promoted.diagnostic_guest_script(
                offline,
                "set -eu\n"
                "export PATH=/opt/node/bin:/opt/cargo/bin:$PATH "
                "CARGO_HOME=/opt/cargo RUSTUP_HOME=/opt/rustup RUSTUP_NO_UPDATE_CHECK=1\n"
                "cd /home/agentmage/source\n"
                f"python3 scripts/sprint_16_linux_worker_guest.py --distribution {target.distribution} "
                "--output /home/agentmage/sprint16-worker-result.json\n",
                stage="sprint16-offline-worker",
                timeout=7200,
            )
            promoted.scp_from_guest(
                offline,
                "/home/agentmage/sprint16-worker-result.json",
                guest_result_path,
            )
            guest_result = json.loads(guest_result_path.read_text(encoding="utf-8"))
            process_sha256 = hashlib.sha256(
                vm_support.ssh_script(
                    offline,
                    "ps -eo uid,pid,ppid,comm --sort=uid,pid\n",
                    stage="sprint16-processes",
                ).encode()
            ).hexdigest()
            socket_sha256 = hashlib.sha256(
                vm_support.ssh_script(
                    offline,
                    "sudo ss -H -a -n -p\n",
                    stage="sprint16-sockets",
                ).encode()
            ).hexdigest()
            vm_support.ssh_script(
                offline,
                "rm -rf /home/agentmage/source /home/agentmage/source.bundle "
                "/home/agentmage/sprint16-worker-result.json\n"
                "sudo cloud-init clean --logs --machine-id\n",
                stage="sprint16-guest-cleanup",
            )
            offline_cleanup = shutdown(offline)
        finally:
            offline_cleanup = merge_cleanup(offline_cleanup, vm_support.stop_vm(offline))
        if not all(offline_cleanup.values()):
            raise WorkerEvidenceError("offline guest cleanup failed")
        overlay.unlink()
        if overlay.exists():
            raise WorkerEvidenceError("guest overlay cleanup failed")
        return {
            "target_id": target.target_id,
            "source_revision": revision,
            "prepared_base_sha256": prepared["prepared_sha256"],
            "source_bundle_sha256": bundle_sha256,
            "strict_offline": True,
            "guest_result": guest_result,
            "observation_sha256": {
                "processes": process_sha256,
                "sockets": socket_sha256,
            },
            "cleanup": {
                "connected": connected_cleanup,
                "offline": offline_cleanup,
                "overlay_absent": True,
                "transient_source_absent": True,
                "credential_material_absent": True,
            },
        }


def build_report(
    revision: str,
    tools: vm_support.HostTools,
    targets: list[dict[str, Any]],
) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "sprint-16-installed-linux-worker-matrix",
        "task_ids": ["16.1.1.5", "16.1.2.3"],
        "source_revision": revision,
        "status": "pass-installed-linux-worker-subset",
        "qemu": {
            "launcher_class": tools.launcher_class,
            "version": tools.qemu_version,
            "sha256": tools.qemu_sha256,
        },
        "targets": targets,
        "verified_operations": ["agentmage.workspace.search-text"],
        "complete_ten_tool_matrix": False,
        "attack_matrix_complete": False,
        "cleanup_campaign_complete": False,
        "macos_evidence_substituted": False,
        "private_host_data_used": False,
        "repository_credentials_injected": False,
        "release_claim": False,
        "sources": source_records(revision),
    }


def validate_report(value: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict):
        return ["installed worker matrix must be an object"]
    if (
        value.get("schema_version") != 1
        or value.get("record_type") != "sprint-16-installed-linux-worker-matrix"
        or value.get("task_ids") != ["16.1.1.5", "16.1.2.3"]
        or value.get("status") != "pass-installed-linux-worker-subset"
        or REVISION.fullmatch(str(value.get("source_revision", ""))) is None
    ):
        failures.append("installed worker matrix identity drifted")
    qemu = value.get("qemu", {})
    if (
        not isinstance(qemu.get("launcher_class"), str)
        or not qemu.get("launcher_class")
        or not isinstance(qemu.get("version"), str)
        or not qemu.get("version")
        or SHA256.fullmatch(str(qemu.get("sha256", ""))) is None
    ):
        failures.append("installed worker QEMU identity drifted")
    targets = value.get("targets")
    expected_targets = [target.target_id for target in docker_vm.TARGETS]
    if not isinstance(targets, list) or [item.get("target_id") for item in targets] != expected_targets:
        failures.append("installed worker target closure drifted")
        targets = []
    for target in targets:
        guest = target.get("guest_result", {})
        worker = guest.get("worker", {})
        commands = guest.get("commands")
        cleanup = target.get("cleanup", {})
        if (
            target.get("strict_offline") is not True
            or target.get("source_revision") != value.get("source_revision")
            or any(
                SHA256.fullmatch(str(item)) is None
                for item in (
                    target.get("prepared_base_sha256"),
                    target.get("source_bundle_sha256"),
                    *target.get("observation_sha256", {}).values(),
                )
            )
            or guest.get("status") != "pass"
            or guest.get("strict_offline") is not True
            or guest.get("verified_operation") != "agentmage.workspace.search-text"
            or guest.get("receipt_count") != 1
            or guest.get("workspace_invariant") is not True
            or any(
                guest.get(field) is not False
                for field in (
                    "worker_process_residue",
                    "transient_unit_residue",
                    "package_residue",
                    "network_used_during_execution",
                    "private_data_used",
                )
            )
            or worker.get("path_class") != "root-owned-package-libexec"
            or worker.get("mode") != "0755"
            or SHA256.fullmatch(str(worker.get("sha256", ""))) is None
            or SHA256.fullmatch(str(guest.get("package_sha256", ""))) is None
            or not isinstance(commands, list)
            or not commands
            or any(
                command.get("exit_code") != 0
                or SHA256.fullmatch(str(command.get("output_sha256", ""))) is None
                for command in commands
            )
        ):
            failures.append(f"installed worker guest result drifted: {target.get('target_id')}")
        if (
            not all(cleanup.get("connected", {}).values())
            or not all(cleanup.get("offline", {}).values())
            or any(
                cleanup.get(field) is not True
                for field in (
                    "overlay_absent",
                    "transient_source_absent",
                    "credential_material_absent",
                )
            )
        ):
            failures.append(f"installed worker cleanup drifted: {target.get('target_id')}")
    if value.get("verified_operations") != ["agentmage.workspace.search-text"]:
        failures.append("installed worker operation claim drifted")
    for field in (
        "complete_ten_tool_matrix",
        "attack_matrix_complete",
        "cleanup_campaign_complete",
        "macos_evidence_substituted",
        "private_host_data_used",
        "repository_credentials_injected",
        "release_claim",
    ):
        if value.get(field) is not False:
            failures.append(f"installed worker prohibited claim changed: {field}")
    sources = value.get("sources")
    if (
        not isinstance(sources, list)
        or [item.get("path") for item in sources] != list(SOURCE_PATHS)
        or any(
            SHA256.fullmatch(str(item.get("sha256", ""))) is None
            or not isinstance(item.get("bytes"), int)
            or item.get("bytes", 0) <= 0
            for item in sources
        )
    ):
        failures.append("installed worker source closure drifted")
    return failures


def execute(revision_value: str) -> dict[str, Any]:
    revision = promoted.clean_revision(revision_value)
    tools = vm_support.discover_host_tools("fedora-toolbox-44")
    with tempfile.TemporaryDirectory(prefix="agentmage-sprint16-source-") as name:
        bundle = Path(name) / "source.bundle"
        bundle_sha256 = promoted.source_bundle(revision, bundle)
        targets = [
            run_target(tools, target, revision, bundle, bundle_sha256)
            for target in docker_vm.TARGETS
        ]
    report = build_report(revision, tools, targets)
    failures = validate_report(report)
    if failures:
        raise WorkerEvidenceError("; ".join(failures))
    write_atomic(REPORT_PATH, report)
    return report


def execute_diagnostic_target(revision_value: str, target_id: str) -> None:
    revision = promoted.clean_revision(revision_value)
    tools = vm_support.discover_host_tools("fedora-toolbox-44")
    matching = [target for target in docker_vm.TARGETS if target.target_id == target_id]
    if len(matching) != 1:
        raise WorkerEvidenceError("diagnostic target identity drifted")
    with tempfile.TemporaryDirectory(prefix="agentmage-sprint16-source-") as name:
        bundle = Path(name) / "source.bundle"
        bundle_sha256 = promoted.source_bundle(revision, bundle)
        run_target(tools, matching[0], revision, bundle, bundle_sha256)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    parser.add_argument(
        "--diagnostic-target",
        choices=tuple(target.target_id for target in docker_vm.TARGETS),
    )
    arguments = parser.parse_args()
    try:
        if arguments.write and arguments.diagnostic_target:
            raise WorkerEvidenceError("write and diagnostic modes are mutually exclusive")
        if arguments.diagnostic_target:
            execute_diagnostic_target(
                arguments.source_revision, arguments.diagnostic_target
            )
            print("Sprint 16 installed worker diagnostic target passed")
            return 0
        if arguments.write:
            report = execute(arguments.source_revision)
        else:
            report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        failures = validate_report(report)
    except (OSError, ValueError, json.JSONDecodeError, subprocess.SubprocessError) as error:
        print(f"Sprint 16 installed worker evidence failed: {error}", file=sys.stderr)
        return 1
    for failure in failures:
        print(f"Sprint 16 installed worker evidence failed: {failure}", file=sys.stderr)
    if failures:
        return 1
    print("Sprint 16 installed Linux worker matrix passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
