#!/usr/bin/env python3
"""Build and validate hostile-position Docker reachability evidence."""

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
    from scripts import linux_docker_kvm_evidence as kvm
    from scripts import linux_native_ubuntu_control_evidence as vm_support
except ModuleNotFoundError:
    import linux_docker_kvm_evidence as kvm
    import linux_native_ubuntu_control_evidence as vm_support


ROOT: Final = Path(__file__).resolve().parents[1]
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-9/story-9.2/linux-docker-reachability.json"
SOURCE_PATHS: Final = (
    "docs/decisions/0038-bind-cross-uid-guard-peer-through-challenge.md",
    "docs/support/linux-docker-reachability-evidence.md",
    "package.json",
    "platforms/linux-inference/src/docker_guard_service.rs",
    "scripts/linux_docker_guard_peer.py",
    "scripts/linux_docker_kvm_guest.py",
    "scripts/linux_docker_reachability_evidence.py",
    "scripts/linux_docker_reachability_guest.py",
    "tests/test_linux_docker_reachability_evidence.py",
)
POSITIONS: Final = [
    "host",
    "same-user",
    "vscode-extension",
    "tool-worker",
    "separate-namespace",
    "lan",
    "ordinary-container",
]
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class ReachabilityEvidenceError(ValueError):
    """Raised when hostile reachability evidence is incomplete."""


def git_bytes(revision: str, path: str) -> bytes:
    completed = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL, timeout=60, check=False,
    )
    if completed.returncode != 0 or not completed.stdout:
        raise ReachabilityEvidenceError("committed reachability source is unavailable")
    return completed.stdout


def sources(revision: str) -> list[dict[str, Any]]:
    return [
        {"path": path, "bytes": len(data := git_bytes(revision, path)),
         "sha256": hashlib.sha256(data).hexdigest()}
        for path in SOURCE_PATHS
    ]


def run_target(
    tools: vm_support.HostTools, target: kvm.Target, revision: str, package: Path
) -> dict[str, Any]:
    prepared = kvm.validate_prepared(target)
    with tempfile.TemporaryDirectory(
        prefix=f"agentmage-{target.distribution}-reachability-", dir=kvm.CACHE_ROOT
    ) as name:
        temporary = Path(name)
        private_key, public_key = vm_support.generate_ssh_key(temporary)
        seed = kvm.create_seed(tools, target, temporary, public_key, bootstrap=False)
        overlay = temporary / "reachability.qcow2"
        vm_support.create_overlay(tools, target.prepared_path, overlay)
        guest = vm_support.start_vm(
            tools, overlay, seed, private_key, temporary, restricted_network=True,
            cpu_count=32, memory_mib=8192,
        )
        result = None
        cleanup = {"qemu_process_absent": False, "loopback_ssh_listener_absent": False}
        try:
            vm_support.wait_for_ssh(guest, timeout=300)
            vm_support.ssh_script(
                guest,
                "cloud-init status --wait >/dev/null\n"
                "test -f /var/lib/agentmage-docker-cloud-init-complete\n"
                "sudo systemctl is-active --quiet docker.service\n",
                timeout=300,
                stage="reachability-guest-startup",
            )
            if not kvm.external_network_denied(guest):
                raise ReachabilityEvidenceError("reachability guest reached the Internet")
            for source in (
                package,
                ROOT / "scripts/linux_docker_kvm_guest.py",
                ROOT / "scripts/linux_docker_guard_peer.py",
                ROOT / "scripts/linux_docker_reachability_guest.py",
            ):
                vm_support.scp_to_guest(guest, source, f"/home/agentmage/{source.name}")
            install = (
                f"sudo rpm -Uvh --replacepkgs --nodeps /home/agentmage/{package.name} >/dev/null"
                if target is kvm.FEDORA else
                f"sudo dpkg -i /home/agentmage/{package.name} >/dev/null"
            )
            script = (
                "set -eu\n" + install + "\n"
                "sudo chmod 0555 /home/agentmage/linux_docker_*.py\n"
                f"cd /home/agentmage && sudo python3 linux_docker_reachability_guest.py {revision}\n"
            )
            completed = vm_support.run(
                [*vm_support.ssh_argv(guest), "/usr/bin/bash", "-s"],
                input_value=script, timeout=900,
            )
            if completed.returncode != 0:
                detail = completed.stderr.strip().splitlines()
                raise ReachabilityEvidenceError(detail[-1] if detail else "guest probe refused")
            result = json.loads(completed.stdout)
        finally:
            cleanup = vm_support.stop_vm(guest)
        if result is None or not all(cleanup.values()):
            raise ReachabilityEvidenceError("reachability guest cleanup failed")
    return {
        "target_id": target.target_id,
        "prepared_sha256": prepared["prepared_sha256"],
        "package_sha256": kvm.sha256_file(package),
        "observation": result,
        "host_cleanup": cleanup | {"overlay_absent": True},
    }


def build_report(tools: vm_support.HostTools, revision: str) -> dict[str, Any]:
    with tempfile.TemporaryDirectory(prefix="agentmage-reachability-packages-") as name:
        packages = kvm.build_acceptance_packages(Path(name))
        targets = [
            run_target(tools, target, revision, packages["rpm" if target is kvm.FEDORA else "deb"])
            for target in kvm.TARGETS
        ]
    return {
        "schema_version": 1,
        "artifact_id": "linux-docker-hostile-reachability",
        "source_revision": revision,
        "task_ids": ["9.2.2.2"],
        "status": "pass-hostile-denial-authenticated-guard-control-no-inference",
        "targets": targets,
        "inference_performed": False,
        "release_support": False,
        "sources": sources(revision),
    }


def validate(value: Any) -> list[str]:
    failures = []
    if not isinstance(value, dict) or value.get("artifact_id") != "linux-docker-hostile-reachability" or value.get("task_ids") != ["9.2.2.2"] or REVISION.fullmatch(str(value.get("source_revision"))) is None:
        return ["reachability report identity changed"]
    targets = value.get("targets")
    if not isinstance(targets, list) or [item.get("target_id") for item in targets] != [item.target_id for item in kvm.TARGETS]:
        failures.append("reachability target closure changed")
    else:
        for target in targets:
            observed = target.get("observation", {})
            probes = observed.get("probes", [])
            control = observed.get("positive_control", {})
            if [item.get("position") for item in probes] != POSITIONS or any(item.get("status") != "denied" or item.get("raw_tcp_connected") is not False for item in probes) or control.get("challenge_authenticated") is not True or control.get("terminal_guard_status") != "docker-guard.service.request-policy" or control.get("inference_request_sent") is not False or observed.get("inference_performed") is not False or any(item is not True for item in observed.get("cleanup", {}).values()) or any(item is not True for item in target.get("host_cleanup", {}).values()):
                failures.append(f"reachability observation changed: {target.get('target_id')}")
    if value.get("inference_performed") is not False or value.get("release_support") is not False:
        failures.append("reachability report overclaimed")
    records = value.get("sources")
    if not isinstance(records, list) or [item.get("path") for item in records] != list(SOURCE_PATHS) or any(SHA256.fullmatch(str(item.get("sha256"))) is None for item in records or []):
        failures.append("reachability source closure changed")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    args = parser.parse_args()
    try:
        tools = vm_support.discover_host_tools("fedora-toolbox-44")
        if args.write:
            revision = kvm.git_revision(args.source_revision)
            report = build_report(tools, revision)
            failures = validate(report)
            if failures:
                raise ReachabilityEvidenceError("; ".join(failures))
            kvm.write_atomic(REPORT_PATH, vm_support.pretty_json(report))
        if not REPORT_PATH.is_file():
            raise ReachabilityEvidenceError("reachability report is missing")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        failures = validate(report)
        if not failures and report["sources"] != sources(report["source_revision"]):
            failures.append("reachability source evidence is stale")
        if failures:
            raise ReachabilityEvidenceError("; ".join(failures))
    except (ReachabilityEvidenceError, OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"Docker reachability evidence failed: {error}", file=sys.stderr)
        return 1
    print("Docker hostile reachability evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
