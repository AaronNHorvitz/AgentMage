#!/usr/bin/env python3
"""Validate the closed Decision 0040 Linux VM regression catalog."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Final

try:
    from scripts import linux_native_ubuntu_control_evidence as vm_support
except ModuleNotFoundError:
    import linux_native_ubuntu_control_evidence as vm_support


ROOT: Final = Path(__file__).resolve().parents[1]
CATALOG_PATH: Final = ROOT / "architecture/linux-vm-base-images.json"
REPORT_PATH: Final = (
    ROOT / "artifacts/sprints/sprint-9/story-9.1/linux-vm-regression-overlays.json"
)
RUN_ROOT: Final = Path.home() / ".cache/agentmage/linux-vm-regression/runs"
EXPECTED_TARGETS: Final = {
    "fedora-44-x86_64": ("fedora", "44"),
    "ubuntu-26.04-x86_64": ("ubuntu", "26.04"),
}
EXPECTED_SOURCES: Final = {
    "fedora-44-x86_64": {
        "publisher": "Fedora Project",
        "url": "https://download.fedoraproject.org/pub/fedora/linux/releases/44/Cloud/x86_64/images/Fedora-Cloud-Base-Generic-44-1.7.x86_64.qcow2",
        "sha256": "28680fe5b371a5a82ebf43a31926e086a168e59949d03969c5093e7071f90b7f",
        "format": "qcow2",
        "license_disposition": "mixed-distribution-package-licenses",
        "license_source": "https://docs.fedoraproject.org/en-US/legal/",
        "cache_path": ".cache/agentmage/docker-kvm/Fedora-Cloud-Base-Generic-44-1.7.x86_64.qcow2",
    },
    "ubuntu-26.04-x86_64": {
        "publisher": "Canonical",
        "url": "https://cloud-images.ubuntu.com/releases/resolute/release/ubuntu-26.04-server-cloudimg-amd64.img",
        "sha256": "9dc7c5363c0146a08ba0c9aa834d82c2c6dfbb1c471ad9a2f0aba1189e21be05",
        "format": "qcow2",
        "license_disposition": "mixed-distribution-package-licenses",
        "license_source": "https://ubuntu.com/legal/intellectual-property-policy",
        "cache_path": ".cache/agentmage/ubuntu-vm/ubuntu-26.04-server-cloudimg-amd64.img",
    },
}
EXPECTED_HARDWARE: Final = {
    "acceleration": "kvm",
    "machine": "q35",
    "cpu": "host",
    "cpu_count": 4,
    "memory_mib": 8192,
    "firmware": "qemu-machine-default-bios",
    "disk": "virtio-qcow2-cache-none",
    "network": "virtio-user-mode-phase-controlled",
    "rng": "virtio-rng-pci",
    "secure_boot": False,
    "virtual_tpm": False,
}


class LinuxVmRegressionError(ValueError):
    """Raised when the local VM authority contract is not exact."""


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        while block := handle.read(1024 * 1024):
            digest.update(block)
    return digest.hexdigest()


def write_atomic(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    content = (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()
    descriptor, name = tempfile.mkstemp(prefix=".agentmage-linux-vm-", dir=path.parent)
    temporary = Path(name)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
    except BaseException:
        temporary.unlink(missing_ok=True)
        raise


def checked(argv: list[str] | tuple[str, ...], *, timeout: int = 120) -> str:
    completed = subprocess.run(
        list(argv),
        cwd=ROOT,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        timeout=timeout,
        check=False,
    )
    if completed.returncode != 0:
        raise LinuxVmRegressionError(f"host command failed: {Path(argv[0]).name}")
    return completed.stdout


def exact_clean_revision(value: str) -> str:
    revision = checked(["git", "rev-parse", "--verify", f"{value}^{{commit}}"], timeout=30).strip()
    if len(revision) != 40 or any(character not in "0123456789abcdef" for character in revision):
        raise LinuxVmRegressionError("Linux VM source revision is invalid")
    if checked(["git", "status", "--porcelain", "--untracked-files=all"], timeout=30):
        raise LinuxVmRegressionError("Linux VM source worktree is dirty")
    return revision


def load_catalog(path: Path = CATALOG_PATH) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise LinuxVmRegressionError("Linux VM catalog is unreadable") from error


def validate_catalog(value: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict):
        return ["Linux VM catalog must be an object"]
    if value.get("schema_version") != 1:
        failures.append("Linux VM catalog schema version drifted")
    if value.get("record_type") != "linux-vm-base-image-catalog":
        failures.append("Linux VM catalog record type drifted")
    if value.get("decision") != "ADR-0040":
        failures.append("Linux VM catalog decision binding drifted")
    if value.get("status") != "admitted-local-regression-bases":
        failures.append("Linux VM catalog status drifted")
    targets = value.get("targets")
    if not isinstance(targets, list) or len(targets) != 2:
        failures.append("Linux VM target closure drifted")
        targets = []
    observed: set[str] = set()
    for target in targets:
        if not isinstance(target, dict):
            failures.append("Linux VM target must be an object")
            continue
        target_id = target.get("target_id")
        if target_id in observed or target_id not in EXPECTED_TARGETS:
            failures.append("Linux VM target identity drifted")
            continue
        observed.add(target_id)
        distribution, version = EXPECTED_TARGETS[target_id]
        if (
            target.get("distribution") != distribution
            or target.get("version") != version
            or target.get("architecture") != "x86_64"
        ):
            failures.append(f"Linux VM platform identity drifted: {target_id}")
        expected_source = EXPECTED_SOURCES[target_id]
        source = target.get("source")
        if source != {key: value for key, value in expected_source.items() if key != "cache_path"}:
            failures.append(f"Linux VM source or license identity drifted: {target_id}")
        cache = Path(str(target.get("cache_path", "")))
        if (
            str(cache) != expected_source["cache_path"]
            or cache.is_absolute()
            or ".." in cache.parts
            or not cache.parts
        ):
            failures.append(f"Linux VM cache path is unsafe: {target_id}")
        packages = target.get("package_snapshot")
        if not isinstance(packages, list) or not packages:
            failures.append(f"Linux VM package snapshot is absent: {target_id}")
        elif [item.get("id") for item in packages if isinstance(item, dict)] != sorted(
            item.get("id") for item in packages if isinstance(item, dict)
        ) or any(
            not isinstance(item, dict)
            or set(item) != {"id", "version"}
            or not item["id"]
            or not item["version"]
            for item in packages
        ):
            failures.append(f"Linux VM package snapshot drifted: {target_id}")
    if observed != set(EXPECTED_TARGETS):
        failures.append("Linux VM target set is incomplete")
    if value.get("virtual_hardware") != EXPECTED_HARDWARE:
        failures.append("Linux VM virtual hardware profile drifted")
    if value.get("toolchain") != {
        "rust": "1.95.0",
        "node": "24.15.0",
        "npm": "11.12.1",
    }:
        failures.append("Linux VM toolchain profile drifted")
    vscode = value.get("visual_studio_code")
    if vscode != {
        "version": "1.132.0",
        "commit": "df53daabb18cd157bdb08c7f01c34df936cf12f4",
        "url": "https://update.code.visualstudio.com/1.132.0/linux-x64/stable",
        "archive_sha256": "acdaf0fa557bda1720956ff65ca0de0965e92d68f97e2db22341984400937aed",
    }:
        failures.append("Linux VM Visual Studio Code identity drifted")
    if value.get("standard_user") != {
        "name": "agentmage",
        "uid": 10001,
        "gid": 10001,
        "administrator_during_acceptance": False,
    }:
        failures.append("Linux VM standard-user identity drifted")
    if value.get("network_phases") != [
        "dependency-acquisition",
        "connected-adapter",
        "strict-offline",
    ]:
        failures.append("Linux VM network-phase order drifted")
    for field in (
        "repository_credentials_allowed",
        "private_user_data_allowed",
        "release_claim",
    ):
        if value.get(field) is not False:
            failures.append(f"Linux VM prohibited claim changed: {field}")
    return failures


def verify_cached_bases(value: Any, home: Path = Path.home()) -> list[dict[str, Any]]:
    failures = validate_catalog(value)
    if failures:
        raise LinuxVmRegressionError("; ".join(failures))
    records = []
    for target in value["targets"]:
        path = home / target["cache_path"]
        if not path.is_file():
            raise LinuxVmRegressionError(
                f"Linux VM base is unavailable: {target['target_id']}"
            )
        observed = sha256_file(path)
        if observed != target["source"]["sha256"]:
            raise LinuxVmRegressionError(
                f"Linux VM base digest changed: {target['target_id']}"
            )
        records.append(
            {
                "target_id": target["target_id"],
                "bytes": path.stat().st_size,
                "sha256": observed,
                "verified": True,
            }
        )
    return records


def exercise_overlays(
    value: Any,
    *,
    source_revision: str,
    report_path: Path = REPORT_PATH,
    home: Path = Path.home(),
    toolbox_container: str = "fedora-toolbox-44",
) -> dict[str, Any]:
    revision = exact_clean_revision(source_revision)
    bases = verify_cached_bases(value, home)
    tools = vm_support.discover_host_tools(toolbox_container)
    RUN_ROOT.mkdir(parents=True, exist_ok=True, mode=0o700)
    records: list[dict[str, Any]] = []
    with tempfile.TemporaryDirectory(prefix="overlay-", dir=RUN_ROOT) as directory:
        run_directory = Path(directory)
        run_directory.chmod(0o700)
        for target, base_record in zip(value["targets"], bases, strict=True):
            base = home / target["cache_path"]
            overlay = run_directory / f"{target['target_id']}.qcow2"
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
                ]
            )
            checked([*tools.qemu_img, "check", "-q", str(overlay)])
            try:
                info = json.loads(
                    checked([*tools.qemu_img, "info", "--output=json", str(overlay)])
                )
            except json.JSONDecodeError as error:
                raise LinuxVmRegressionError("Linux VM overlay metadata is malformed") from error
            if (
                info.get("format") != "qcow2"
                or Path(str(info.get("backing-filename", ""))).resolve() != base.resolve()
                or not overlay.is_file()
            ):
                raise LinuxVmRegressionError("Linux VM overlay backing identity drifted")
            records.append(
                {
                    "target_id": target["target_id"],
                    "base_bytes": base_record["bytes"],
                    "base_sha256": base_record["sha256"],
                    "overlay_format": "qcow2",
                    "backing_format": "qcow2",
                    "overlay_created": True,
                    "overlay_cleanup_verified": False,
                }
            )
        report = {
            "schema_version": 1,
            "record_type": "linux-vm-regression-overlay-evidence",
            "task_ids": ["9.1.4.1", "9.1.4.2"],
            "source_revision": revision,
            "status": "pre-cleanup-retained",
            "qemu": {
                "launcher_class": tools.launcher_class,
                "version": tools.qemu_version,
                "sha256": tools.qemu_sha256,
                "kvm_accessible": True,
            },
            "targets": records,
            "repository_credentials_injected": False,
            "private_user_data_used": False,
            "guest_started": False,
            "network_used": False,
            "release_claim": False,
        }
        write_atomic(report_path, report)
        for record in records:
            overlay = run_directory / f"{record['target_id']}.qcow2"
            overlay.unlink()
            record["overlay_cleanup_verified"] = not overlay.exists()
        if not all(record["overlay_cleanup_verified"] for record in records):
            raise LinuxVmRegressionError("Linux VM overlay cleanup failed")
        report["status"] = "pass-local-overlay-lifecycle"
        write_atomic(report_path, report)
    if run_directory.exists():
        raise LinuxVmRegressionError("Linux VM transient run directory remains")
    return report


def validate_overlay_report(value: Any, catalog: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict):
        return ["Linux VM overlay report must be an object"]
    if (
        value.get("schema_version") != 1
        or value.get("record_type") != "linux-vm-regression-overlay-evidence"
        or value.get("task_ids") != ["9.1.4.1", "9.1.4.2"]
        or value.get("status") != "pass-local-overlay-lifecycle"
    ):
        failures.append("Linux VM overlay report identity drifted")
    qemu = value.get("qemu", {})
    if (
        qemu.get("launcher_class") not in {"host-system-path", "fedora-toolbox"}
        or not qemu.get("version")
        or len(str(qemu.get("sha256", ""))) != 64
        or qemu.get("kvm_accessible") is not True
    ):
        failures.append("Linux VM QEMU or KVM identity drifted")
    targets = value.get("targets", [])
    if [item.get("target_id") for item in targets if isinstance(item, dict)] != [
        target["target_id"] for target in catalog.get("targets", [])
    ]:
        failures.append("Linux VM overlay target order drifted")
    if any(
        not isinstance(item, dict)
        or item.get("overlay_format") != "qcow2"
        or item.get("backing_format") != "qcow2"
        or item.get("overlay_created") is not True
        or item.get("overlay_cleanup_verified") is not True
        or item.get("base_sha256")
        != catalog["targets"][index]["source"]["sha256"]
        for index, item in enumerate(targets)
    ):
        failures.append("Linux VM overlay lifecycle drifted")
    for field in (
        "repository_credentials_injected",
        "private_user_data_used",
        "guest_started",
        "network_used",
        "release_claim",
    ):
        if value.get(field) is not False:
            failures.append(f"Linux VM overlay prohibited claim changed: {field}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--verify-bases", action="store_true")
    parser.add_argument("--exercise-overlays", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    try:
        catalog = load_catalog()
        failures = validate_catalog(catalog)
        if failures:
            for failure in failures:
                print(f"Linux VM regression contract failed: {failure}", file=sys.stderr)
            return 1
        if arguments.verify_bases:
            verify_cached_bases(catalog)
        if arguments.exercise_overlays:
            exercise_overlays(catalog, source_revision=arguments.source_revision)
        elif REPORT_PATH.is_file():
            report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
            failures = validate_overlay_report(report, catalog)
            if failures:
                raise LinuxVmRegressionError("; ".join(failures))
    except LinuxVmRegressionError as error:
        print(f"Linux VM regression contract failed: {error}", file=sys.stderr)
        return 1
    print("Linux VM regression contract passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
