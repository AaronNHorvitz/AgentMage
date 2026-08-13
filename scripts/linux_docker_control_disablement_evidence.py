#!/usr/bin/env python3
"""Build and validate independent Docker control-disablement evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
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
REPORT_PATH: Final = (
    ROOT
    / "artifacts/sprints/sprint-9/story-9.2/linux-docker-control-disablement.json"
)
SOURCE_PATHS: Final = (
    "docs/decisions/0039-verify-independent-docker-control-disablement.md",
    "docs/support/linux-docker-control-disablement-evidence.md",
    "package.json",
    "platforms/linux-inference/src/docker_live_collector.rs",
    "platforms/linux-inference/src/docker_preflight.rs",
    "scripts/linux_docker_control_disablement_evidence.py",
    "scripts/linux_docker_control_disablement_guest.py",
    "scripts/linux_docker_kvm_guest.py",
    "tests/test_linux_docker_control_disablement_evidence.py",
)
CONTROLS: Final = [
    (
        "daemon-privilege",
        "runtime-user-in-docker-group",
        "docker-preflight.daemon.privilege",
    ),
    (
        "socket-ownership",
        "docker-socket-world-writable",
        "docker-preflight.socket.ownership",
    ),
    (
        "api-binding",
        "private-management-listener",
        "docker-preflight.api.binding",
    ),
    (
        "container-reachability",
        "guard-socket-world-writable",
        "docker-preflight.container.reachability",
    ),
    (
        "image-identity",
        "model-manifest-substitution",
        "docker-preflight.image.identity",
    ),
    (
        "resource-limits",
        "runner-memory-limit-drift",
        "docker-preflight.resources.invalid",
    ),
    ("zero-egress", "ambient-proxy-injection", "docker-preflight.egress.nonzero"),
]
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class ControlDisablementEvidenceError(ValueError):
    """Raised when control-disablement evidence is incomplete or overstated."""


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
        raise ControlDisablementEvidenceError(
            "committed control-disablement source is unavailable"
        )
    return completed.stdout


def sources(revision: str) -> list[dict[str, Any]]:
    return [
        {
            "path": path,
            "bytes": len(data := git_bytes(revision, path)),
            "sha256": hashlib.sha256(data).hexdigest(),
        }
        for path in SOURCE_PATHS
    ]


def run_target(
    tools: vm_support.HostTools,
    target: kvm.Target,
    revision: str,
    package: Path,
) -> dict[str, Any]:
    prepared = kvm.validate_prepared(target)
    with tempfile.TemporaryDirectory(
        prefix=f"agentmage-{target.distribution}-control-disablement-",
        dir=kvm.CACHE_ROOT,
    ) as name:
        temporary = Path(name)
        private_key, public_key = vm_support.generate_ssh_key(temporary)
        seed = kvm.create_seed(tools, target, temporary, public_key, bootstrap=False)
        overlay = temporary / "control-disablement.qcow2"
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
                stage="control-disablement-guest-startup",
            )
            if not kvm.external_network_denied(guest):
                raise ControlDisablementEvidenceError(
                    "control-disablement guest reached the Internet"
                )
            for source in (
                package,
                ROOT / "scripts/linux_docker_kvm_guest.py",
                ROOT / "scripts/linux_docker_control_disablement_guest.py",
            ):
                vm_support.scp_to_guest(
                    guest, source, f"/home/agentmage/{source.name}"
                )
            install = (
                f"sudo rpm -Uvh --replacepkgs --nodeps /home/agentmage/{package.name} >/dev/null"
                if target is kvm.FEDORA
                else f"sudo dpkg -i /home/agentmage/{package.name} >/dev/null"
            )
            script = (
                "set -eu\n"
                + install
                + "\n"
                "sudo chmod 0555 /home/agentmage/linux_docker_*.py\n"
                "cd /home/agentmage && sudo python3 "
                f"linux_docker_control_disablement_guest.py {revision}\n"
            )
            completed = vm_support.run(
                [*vm_support.ssh_argv(guest), "/usr/bin/bash", "-s"],
                input_value=script,
                timeout=1200,
            )
            if completed.returncode != 0:
                detail = completed.stderr.strip().splitlines()
                raise ControlDisablementEvidenceError(
                    detail[-1] if detail else "guest disablement probe refused"
                )
            result = json.loads(completed.stdout)
        finally:
            cleanup = vm_support.stop_vm(guest)
        if result is None or not all(cleanup.values()):
            raise ControlDisablementEvidenceError(
                "control-disablement guest cleanup failed"
            )
    return {
        "target_id": target.target_id,
        "prepared_sha256": prepared["prepared_sha256"],
        "package_sha256": kvm.sha256_file(package),
        "observation": result,
        "host_cleanup": cleanup | {"overlay_absent": True},
    }


def build_report(tools: vm_support.HostTools, revision: str) -> dict[str, Any]:
    with tempfile.TemporaryDirectory(
        prefix="agentmage-control-disablement-packages-"
    ) as name:
        packages = kvm.build_acceptance_packages(Path(name))
        targets = [
            run_target(
                tools,
                target,
                revision,
                packages["rpm" if target is kvm.FEDORA else "deb"],
            )
            for target in kvm.TARGETS
        ]
    return {
        "schema_version": 1,
        "artifact_id": "linux-docker-independent-control-disablement",
        "source_revision": revision,
        "task_ids": ["9.2.2.3"],
        "status": "pass-independent-refusal-no-fallback-no-inference",
        "targets": targets,
        "fallback_selected": False,
        "inference_performed": False,
        "release_support": False,
        "sources": sources(revision),
    }


def validate(value: Any) -> list[str]:
    failures: list[str] = []
    if (
        not isinstance(value, dict)
        or value.get("artifact_id")
        != "linux-docker-independent-control-disablement"
        or value.get("task_ids") != ["9.2.2.3"]
        or REVISION.fullmatch(str(value.get("source_revision"))) is None
    ):
        return ["control-disablement report identity changed"]
    targets = value.get("targets")
    if not isinstance(targets, list) or [
        item.get("target_id") for item in targets
    ] != [item.target_id for item in kvm.TARGETS]:
        failures.append("control-disablement target closure changed")
    else:
        for target in targets:
            observed = target.get("observation", {})
            cases = observed.get("cases", [])
            actual = [
                (
                    item.get("control"),
                    item.get("mutation"),
                    item.get("observed_refusal"),
                )
                for item in cases
            ]
            native = observed.get("native_descriptor", {})
            if (
                actual != CONTROLS
                or observed.get("baseline", {}).get("status") != "admitted"
                or observed.get("fallback_selected") is not False
                or observed.get("inference_performed") is not False
                or native.get("docker_compatibility_available") is not False
                or native.get("inference_available") is not False
                or native.get("enabled_models") != 0
                or any(
                    item.get("expected_refusal") != item.get("observed_refusal")
                    or item.get("docker_admitted") is not False
                    or item.get("native_fallback_selected") is not False
                    or item.get("inference_performed") is not False
                    or not all(item.get("cleanup", {}).values())
                    for item in cases
                )
                or not all(observed.get("baseline", {}).get("cleanup", {}).values())
                or not all(observed.get("final_cleanup", {}).values())
                or not all(target.get("host_cleanup", {}).values())
            ):
                failures.append(
                    "control-disablement observation changed: "
                    f"{target.get('target_id')}"
                )
    if (
        value.get("fallback_selected") is not False
        or value.get("inference_performed") is not False
        or value.get("release_support") is not False
    ):
        failures.append("control-disablement report overclaimed")
    records = value.get("sources")
    if (
        not isinstance(records, list)
        or [item.get("path") for item in records] != list(SOURCE_PATHS)
        or any(
            SHA256.fullmatch(str(item.get("sha256"))) is None
            for item in records or []
        )
    ):
        failures.append("control-disablement source closure changed")
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
                raise ControlDisablementEvidenceError("; ".join(failures))
            kvm.write_atomic(REPORT_PATH, vm_support.pretty_json(report))
        if not REPORT_PATH.is_file():
            raise ControlDisablementEvidenceError(
                "control-disablement report is missing"
            )
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        failures = validate(report)
        if not failures and report["sources"] != sources(report["source_revision"]):
            failures.append("control-disablement source evidence is stale")
        if failures:
            raise ControlDisablementEvidenceError("; ".join(failures))
    except (
        ControlDisablementEvidenceError,
        OSError,
        ValueError,
        subprocess.SubprocessError,
    ) as error:
        print(f"Docker control-disablement evidence failed: {error}", file=sys.stderr)
        return 1
    print("Docker independent control-disablement evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
