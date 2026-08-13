#!/usr/bin/env python3
"""Build and validate Sprint 9 native llama.cpp package-boundary evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import shutil
import stat
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path
from typing import Any, Final

try:
    from scripts.native_llama_runtime import (
        PROFILE_PATH,
        build_runtime_package,
        load_profile,
        read_source_payload,
        verify_runtime_package,
    )
except ModuleNotFoundError:
    from native_llama_runtime import (
        PROFILE_PATH,
        build_runtime_package,
        load_profile,
        read_source_payload,
        verify_runtime_package,
    )


ROOT: Final = Path(__file__).resolve().parents[1]
REPORT_PATH: Final = (
    ROOT
    / "artifacts/sprints/sprint-9/story-9.2/linux-native-runtime-package.json"
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
    "model-profiles/runtimes/docker-model-runner-v1.2.6-linux-x86_64.json",
    "model-profiles/runtimes/llama-cpp-b10333-linux-x86_64.json",
    "package.json",
    "platforms/linux-inference/Cargo.toml",
    "platforms/linux-inference/README.md",
    "platforms/linux-inference/src/lib.rs",
    "platforms/linux-inference/src/main.rs",
    "platforms/linux-inference/src/docker_runtime.rs",
    "platforms/linux-inference/src/native_runtime.rs",
    "platforms/linux-inference/tests/process_boundary.rs",
    "scripts/build_contract.py",
    "scripts/component_inventory.py",
    "scripts/linux_native_runtime_evidence.py",
    "scripts/native_llama_runtime.py",
    "tests/test_component_inventory.py",
    "tests/test_linux_native_runtime_evidence.py",
    "tests/test_native_llama_runtime.py",
)
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
ALLOWED_DYNAMIC_LIBRARIES: Final = {
    "ld-linux-x86-64.so.2",
    "libc.so.6",
    "libgcc_s.so.1",
    "libggml-base.so.0",
    "libggml.so.0",
    "libgomp.so.1",
    "libm.so.6",
    "libstdc++.so.6",
}
EXPECTED_DESCRIPTOR: Final = {
    "accepted_operation": "self-check-only",
    "authority_inputs": [],
    "component_id": "platform-linux-native-inference",
    "contract": "authenticated-local-endpoint-v1",
    "docker_compatibility_available": False,
    "docker_model_artifact_digest": "sha256:08fa7b1d44f255be48cfc12359211725bfd659742612ed4b221cd5be90d14444",
    "docker_model_runner_image_digest": "sha256:bd94095bbc1ddc4266c3a88f582a92562c6b63eceb175572c9a60045663727c9",
    "docker_runtime_profile_sha256": "ab8cde6bc1440f8a0013390aa2e291a315cdebcfe44aa1d339f0aa0b1d70899c",
    "enabled_models": 0,
    "inference_available": False,
    "native_runtime_package": "agentmage-llama-cpp-b10333-cpu-linux-x86_64",
    "native_runtime_profile_sha256": "21346c06fb86b418706326b186609f8e1f690d6b57b53e73d02b4ad8e28e53ea",
    "network_listener": False,
    "process_boundary_version": 3,
}
LIMITATIONS: Final = [
    "The package is a pinned CPU-library input for the inactive AgentMage adapter; no model is enabled and inference is not implemented.",
    "The evidence builds on native Fedora 44 x86_64; clean native Ubuntu package execution remains owned by Sub-task 9.2.2.1.",
    "Vulkan, CUDA, Docker Model Runner, model-family codecs, LocalModelRuntime, streaming, and cancellation are outside this sub-task.",
    "The runtime package is not a supported release and carries no model, platform, performance, or release approval.",
]


class LinuxNativeRuntimeEvidenceError(ValueError):
    """Raised when native runtime evidence is missing, stale, or overclaimed."""


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(
        prefix=".agentmage-linux-native-runtime-", dir=path.parent
    )
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


def run_command(
    arguments: list[str], timeout: int = 900
) -> subprocess.CompletedProcess[str]:
    environment = os.environ.copy()
    environment["CARGO_NET_OFFLINE"] = "true"
    return subprocess.run(
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


def require_command(arguments: list[str], timeout: int = 900) -> str:
    result = run_command(arguments, timeout)
    if result.returncode != 0:
        raise LinuxNativeRuntimeEvidenceError(
            f"verification command failed: {Path(arguments[0]).name}"
        )
    return result.stdout


def git_revision(candidate: str) -> str:
    result = run_command(["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"], 20)
    revision = result.stdout.strip()
    if result.returncode != 0 or REVISION.fullmatch(revision) is None:
        raise LinuxNativeRuntimeEvidenceError("source revision is unavailable")
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
        raise LinuxNativeRuntimeEvidenceError(f"committed source absent: {relative}")
    return result.stdout


def source_records(revision: str) -> list[dict[str, Any]]:
    records = []
    for relative in SOURCE_PATHS:
        committed = committed_file(revision, relative)
        current = ROOT / relative
        if not current.is_file() or current.read_bytes() != committed:
            raise LinuxNativeRuntimeEvidenceError(
                f"source differs from revision: {relative}"
            )
        records.append(
            {"bytes": len(committed), "path": relative, "sha256": sha256_bytes(committed)}
        )
    return records


def host_identity() -> dict[str, str]:
    values: dict[str, str] = {}
    for line in Path("/etc/os-release").read_text(encoding="utf-8").splitlines():
        if "=" in line:
            key, value = line.split("=", 1)
            values[key] = value.strip('"')
    identity = {
        "architecture": platform.machine(),
        "distribution": values.get("ID", "unknown"),
        "version": values.get("VERSION_ID", "unknown"),
    }
    if identity != {
        "architecture": "x86_64",
        "distribution": "fedora",
        "version": "44",
    }:
        raise LinuxNativeRuntimeEvidenceError("host is not Fedora 44 x86_64")
    return identity


def inspect_dynamic_libraries(
    archive: Path, profile: dict[str, Any], directory: Path
) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    readelf = shutil.which("readelf")
    if readelf is None:
        raise LinuxNativeRuntimeEvidenceError("readelf is unavailable")
    executable = Path(readelf).resolve()
    version = require_command([str(executable), "--version"], 20).splitlines()[0]
    payload = read_source_payload(archive, profile)
    directory.mkdir(parents=True, exist_ok=False)
    records = []
    for item in profile["source_files"]:
        if item["type"] != "file" or not item["destination"].startswith("lib/"):
            continue
        content = payload[item["source"]]
        if not isinstance(content, bytes):
            raise LinuxNativeRuntimeEvidenceError("library payload type changed")
        path = directory / Path(item["destination"]).name
        path.write_bytes(content)
        path.chmod(0o444)
        header = require_command([str(executable), "-h", str(path)], 20)
        dynamic = require_command([str(executable), "-d", str(path)], 20)
        if "ELF64" not in header or "DYN (Shared object file)" not in header:
            raise LinuxNativeRuntimeEvidenceError("retained library is not an ELF64 shared object")
        needed = sorted(
            set(re.findall(r"Shared library: \[([^]]+)\]", dynamic))
        )
        if not set(needed).issubset(ALLOWED_DYNAMIC_LIBRARIES):
            raise LinuxNativeRuntimeEvidenceError("retained library dependency is undeclared")
        records.append(
            {
                "destination": item["destination"],
                "needed": needed,
                "sha256": item["sha256"],
                "size_bytes": item["size_bytes"],
            }
        )
    return records, {
        "executable_sha256": sha256_file(executable),
        "path_class": "maintainer-host-tool",
        "version": version,
    }


def package_members(package: Path) -> list[dict[str, Any]]:
    records = []
    with tarfile.open(package, mode="r:gz") as archive:
        for member in archive.getmembers():
            records.append(
                {
                    "mode": member.mode,
                    "name": member.name,
                    "target": member.linkname if member.issym() else None,
                    "type": (
                        "directory"
                        if member.isdir()
                        else "symlink"
                        if member.issym()
                        else "file"
                    ),
                }
            )
    return records


def build_report(revision: str, archive: Path) -> dict[str, Any]:
    require_command(["python3", "-m", "unittest", "tests.test_native_llama_runtime"])
    require_command(
        ["cargo", "test", "-p", "agentmage-platform-linux-inference", "--locked"]
    )
    require_command(
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
    require_command(["python3", "scripts/build_contract.py"])
    profile, profile_sha256 = load_profile(PROFILE_PATH)
    sources = source_records(revision)
    with tempfile.TemporaryDirectory(prefix="agentmage-native-runtime-evidence-") as name:
        temporary = Path(name)
        first = temporary / "first.tar.gz"
        second = temporary / "second.tar.gz"
        first_result = build_runtime_package(archive, first, PROFILE_PATH)
        second_result = build_runtime_package(archive, second, PROFILE_PATH)
        if first.read_bytes() != second.read_bytes():
            raise LinuxNativeRuntimeEvidenceError("runtime package is not deterministic")
        verified = verify_runtime_package(first, PROFILE_PATH)
        libraries, readelf = inspect_dynamic_libraries(
            archive, profile, temporary / "elf"
        )
        members = package_members(first)
    if first_result["sha256"] != second_result["sha256"] or first_result["sha256"] != verified["sha256"]:
        raise LinuxNativeRuntimeEvidenceError("runtime package identity changed")
    adapter = ROOT / "target/debug/agentmage-native-inference"
    descriptor = json.loads(require_command([str(adapter), "--self-check"], 20))
    if descriptor != EXPECTED_DESCRIPTOR:
        raise LinuxNativeRuntimeEvidenceError("adapter runtime identity changed")
    if descriptor["native_runtime_profile_sha256"] != profile_sha256:
        raise LinuxNativeRuntimeEvidenceError("adapter and package profile identity differ")
    model_state = json.loads(
        (ROOT / "evidence/current/model-activation-report.json").read_text(encoding="utf-8")
    )
    if model_state.get("enabled_profile_count") != 0 or model_state.get("enabled_profiles") != []:
        raise LinuxNativeRuntimeEvidenceError("a model was enabled")
    return {
        "artifact_id": "linux-native-llama-runtime-package",
        "authority": profile["authority"],
        "deterministic_rebuild_count": 2,
        "dynamic_libraries": libraries,
        "enabled_models": 0,
        "host": host_identity(),
        "inference_started": False,
        "limitations": LIMITATIONS,
        "model_store": profile["model_store"],
        "network_used": False,
        "package": {
            "bytes": verified["bytes"],
            "file_count": verified["file_count"],
            "members": members,
            "package_id": verified["package_id"],
            "sha256": verified["sha256"],
        },
        "platform_status": {
            "fedora_44_x86_64": "verified-native-package-build",
            "ubuntu_26_04_x86_64": "not-run-this-subtask",
        },
        "process_boundary": descriptor,
        "profile_sha256": profile_sha256,
        "readelf": readelf,
        "release_claim": "none",
        "resource_ceiling": profile["resource_ceiling"],
        "retained_regular_library_count": len(libraries),
        "schema_version": 1,
        "source_archive": {
            "bytes": archive.stat().st_size,
            "name": profile["source_archive"]["name"],
            "sha256": sha256_file(archive),
        },
        "source_revision": revision,
        "sources": sources,
        "status": "pass-fedora-native-package-boundary",
        "task_ids": ["9.2.1.1"],
        "upstream_entrypoints_included": [],
    }


def validate_report(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["Linux native runtime report must be an object"]
    failures = []
    if (
        value.get("schema_version") != 1
        or value.get("artifact_id") != "linux-native-llama-runtime-package"
        or value.get("task_ids") != ["9.2.1.1"]
        or value.get("status") != "pass-fedora-native-package-boundary"
        or REVISION.fullmatch(str(value.get("source_revision"))) is None
    ):
        failures.append("Linux native runtime report identity changed")
    if value.get("authority") != dict.fromkeys(
        ("credential", "grant", "network", "tool", "workspace"), False
    ):
        failures.append("runtime authority closure changed")
    if (
        value.get("enabled_models") != 0
        or value.get("inference_started") is not False
        or value.get("network_used") is not False
        or value.get("release_claim") != "none"
        or value.get("upstream_entrypoints_included") != []
    ):
        failures.append("runtime state or authority was overclaimed")
    if value.get("deterministic_rebuild_count") != 2:
        failures.append("deterministic rebuild evidence is incomplete")
    if value.get("process_boundary") != EXPECTED_DESCRIPTOR:
        failures.append("adapter process boundary changed")
    package = value.get("package", {})
    members = package.get("members")
    if (
        package.get("package_id")
        != "agentmage-llama-cpp-b10333-cpu-linux-x86_64"
        or SHA256.fullmatch(str(package.get("sha256"))) is None
        or not isinstance(package.get("bytes"), int)
        or package.get("bytes", 0) <= 0
        or package.get("file_count") != 28
        or not isinstance(members, list)
        or len(members) != 28
        or any(
            term in str(item.get("name", "")).lower()
            for item in members
            for term in ("server", "rpc", "cli", "vulkan", "download")
        )
    ):
        failures.append("runtime package closure is invalid")
    libraries = value.get("dynamic_libraries")
    if (
        value.get("retained_regular_library_count") != 17
        or not isinstance(libraries, list)
        or len(libraries) != 17
        or any(
            SHA256.fullmatch(str(item.get("sha256"))) is None
            or not set(item.get("needed", [])).issubset(ALLOWED_DYNAMIC_LIBRARIES)
            for item in libraries
        )
    ):
        failures.append("retained dynamic-library closure is invalid")
    if value.get("model_store") != {
        "directory_enumeration": False,
        "directory_mode": 0o700,
        "input": "kernel-held-read-only-model-descriptors",
        "path_input": False,
        "replacement": "refuse-existing-or-drifted-identity",
        "required_owner": "invoking-standard-user",
        "runtime_write": False,
    }:
        failures.append("model-store boundary changed")
    if value.get("resource_ceiling") != {
        "cpu_percent": 3200,
        "memory_bytes": 64 * 1024 * 1024 * 1024,
        "output_bytes": 16 * 1024 * 1024,
        "parallel_slots": 1,
        "runtime_seconds": 3600,
        "swap_bytes": 0,
        "tasks": 64,
    }:
        failures.append("resource ceiling changed")
    if value.get("platform_status") != {
        "fedora_44_x86_64": "verified-native-package-build",
        "ubuntu_26_04_x86_64": "not-run-this-subtask",
    }:
        failures.append("platform status changed")
    sources = value.get("sources")
    if (
        not isinstance(sources, list)
        or [item.get("path") for item in sources] != list(SOURCE_PATHS)
        or any(
            SHA256.fullmatch(str(item.get("sha256"))) is None
            or not isinstance(item.get("bytes"), int)
            or item.get("bytes", -1) < 0
            for item in sources
        )
    ):
        failures.append("source closure is invalid")
    source_archive = value.get("source_archive", {})
    if (
        source_archive.get("name") != "llama-b10333-bin-ubuntu-vulkan-x64.tar.gz"
        or source_archive.get("bytes") != 32521550
        or source_archive.get("sha256")
        != "f14e312fbee33ce60d2eed7036de5debe31c1d7f4d8f0e37920eb0a2de0854a5"
    ):
        failures.append("source archive identity changed")
    if SHA256.fullmatch(str(value.get("profile_sha256"))) is None:
        failures.append("profile identity is invalid")
    readelf = value.get("readelf", {})
    if (
        readelf.get("path_class") != "maintainer-host-tool"
        or SHA256.fullmatch(str(readelf.get("executable_sha256"))) is None
        or not isinstance(readelf.get("version"), str)
        or not readelf.get("version")
    ):
        failures.append("ELF inspection tool identity is invalid")
    if value.get("limitations") != LIMITATIONS:
        failures.append("native runtime limitations changed")
    return failures


def check_report(archive: Path) -> None:
    try:
        actual = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        revision = actual["source_revision"]
    except (OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise LinuxNativeRuntimeEvidenceError(
            f"cannot read Linux native runtime report: {error}"
        ) from error
    failures = validate_report(actual)
    if not isinstance(revision, str) or actual != build_report(revision, archive):
        failures.append("Linux native runtime report is stale or malformed")
    if failures:
        raise LinuxNativeRuntimeEvidenceError("; ".join(failures))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, required=True)
    parser.add_argument("--source-revision", default="HEAD")
    parser.add_argument("--write", action="store_true")
    arguments = parser.parse_args(argv)
    archive = arguments.archive.resolve()
    try:
        if arguments.write:
            revision = git_revision(arguments.source_revision)
            write_atomic(REPORT_PATH, canonical_json(build_report(revision, archive)))
        check_report(archive)
    except (
        LinuxNativeRuntimeEvidenceError,
        OSError,
        UnicodeError,
        json.JSONDecodeError,
        subprocess.SubprocessError,
        tarfile.TarError,
    ) as error:
        print(f"Linux native runtime evidence failed: {error}", file=sys.stderr)
        return 1
    print("Linux native llama.cpp runtime package evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
