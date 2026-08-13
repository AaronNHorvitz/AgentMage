#!/usr/bin/env python3
"""Build and validate the Sprint 9 inactive Linux inference boundary."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import stat
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path
from typing import Any

try:
    from scripts.package_candidate import build_all
    from scripts.package_lifecycle import verify_extracted_candidates
except ModuleNotFoundError:
    from package_candidate import build_all
    from package_lifecycle import verify_extracted_candidates


ROOT = Path(__file__).resolve().parents[1]
REPORT_PATH = (
    ROOT
    / "artifacts"
    / "sprints"
    / "sprint-9"
    / "story-9.1"
    / "linux-native-inference-boundary.json"
)
ADAPTER_MANIFEST = "platforms/linux-inference/Cargo.toml"
ADAPTER_BINARY = "target/release/agentmage-native-inference"
SOURCE_PATHS = (
    "Cargo.lock",
    "Cargo.toml",
    "architecture/build-contract.json",
    "architecture/dependency-rules.json",
    "architecture/module-inventory.json",
    "docs/architecture/linux-platform-lifecycle.md",
    "docs/decisions/0020-package-candidates-and-windows-increment.md",
    "docs/decisions/0022-detached-package-signing-boundary.md",
    "docs/decisions/0023-package-verified-linux-bootstrap-transport.md",
    "docs/decisions/0028-isolated-linux-inference-package-boundary.md",
    "docs/decisions/0029-closed-linux-native-llama-runtime-package.md",
    "docs/decisions/0030-closed-linux-docker-model-runner-compatibility-profile.md",
    "docs/decisions/0031-private-docker-model-runner-endpoint-guard.md",
    "docs/decisions/0032-fail-closed-docker-topology-preflight.md",
    "IMPLEMENTATION-PLAN.md",
    "package.json",
    "packaging/linux/README.md",
    "packaging/linux/agentmage-release.spec.in",
    "packaging/linux/agentmage.spec.in",
    "model-profiles/runtimes/llama-cpp-b10333-linux-x86_64.json",
    "model-profiles/runtimes/docker-model-runner-v1.2.6-linux-x86_64.json",
    "model-profiles/runtimes/docker-model-runner-guard-v1-linux-x86_64.json",
    "platforms/linux-inference/Cargo.toml",
    "platforms/linux-inference/README.md",
    "platforms/linux-inference/src/lib.rs",
    "platforms/linux-inference/src/main.rs",
    "platforms/linux-inference/src/docker_runtime.rs",
    "platforms/linux-inference/src/docker_guard.rs",
    "platforms/linux-inference/src/docker_preflight.rs",
    "platforms/linux-inference/src/native_runtime.rs",
    "platforms/linux-inference/tests/process_boundary.rs",
    "platforms/linux/README.md",
    "platforms/linux/src/platform.rs",
    "release/README.md",
    "scripts/build_contract.py",
    "scripts/dependency_classes.py",
    "scripts/dependency_rules.py",
    "scripts/linux_inference_boundary_evidence.py",
    "scripts/module_inventory.py",
    "scripts/package_candidate.py",
    "scripts/package_lifecycle.py",
    "scripts/package_release_lifecycle.py",
    "shells/host/src/linux_bootstrap.rs",
    "shells/host/src/package_verify.rs",
    "supply-chain/dependency-hashes.sha256",
    "supply-chain/dependency-provenance.json",
    "supply-chain/sbom.cdx.json",
    "tests/test_linux_inference_boundary_evidence.py",
    "tests/test_package_candidate.py",
)
EXPECTED_DESCRIPTOR = {
    "accepted_operation": "self-check-only",
    "authority_inputs": [],
    "component_id": "platform-linux-native-inference",
    "contract": "authenticated-local-endpoint-v1",
    "docker_compatibility_available": False,
    "docker_guard_profile_sha256": "a744eb31f4949ec7d99dfae8f62eccb531269c3549ee51f3977e0ea62a8f88c8",
    "docker_model_artifact_digest": "sha256:08fa7b1d44f255be48cfc12359211725bfd659742612ed4b221cd5be90d14444",
    "docker_model_runner_image_digest": "sha256:bd94095bbc1ddc4266c3a88f582a92562c6b63eceb175572c9a60045663727c9",
    "docker_preflight_contract_version": 2,
    "docker_runtime_profile_sha256": "eef3e99df6ab418412bccc219a3ea4cffff18aaa73e3ee83e1ef60a0d615a99a",
    "enabled_models": 0,
    "inference_available": False,
    "native_runtime_package": "agentmage-llama-cpp-b10333-cpu-linux-x86_64",
    "native_runtime_profile_sha256": "21346c06fb86b418706326b186609f8e1f690d6b57b53e73d02b4ad8e28e53ea",
    "network_listener": False,
    "process_boundary_version": 6,
}
EXPECTED_DEPENDENCIES = {"agentmage-kernel-contracts"}
FORBIDDEN_COMPILE_REFERENCES = (
    "agentmage_capability_read_only",
    "agentmage_kernel_engine",
    "agentmage_platform_linux::",
    "agentmage_host",
)
LIMITATIONS = [
    "The candidate-neutral LocalModelRuntime, model-family codecs, profiles, streaming, cancellation, and resource protocol remain assigned to Sprint 13.",
    "No llama.cpp runtime library or model artifact is included in the core RPM/DEB package increment; the separate Sprint 9.2 runtime input remains inactive.",
    "This boundary report builds and extracts the DEB on Fedora; clean Ubuntu installation and execution are recorded separately under Sub-task 9.1.1.7.",
    "No supported package, enabled model, inference result, macOS result, or release claim is made.",
]
SHA256 = re.compile(r"^[0-9a-f]{64}$")
REVISION = re.compile(r"^[0-9a-f]{40}$")


class LinuxInferenceBoundaryEvidenceError(ValueError):
    """Raised when package/process evidence is missing, stale, or overclaimed."""


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
        prefix=".agentmage-linux-inference-", dir=path.parent
    )
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


def run_command(arguments: list[str], timeout: int = 900) -> subprocess.CompletedProcess[str]:
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
        raise LinuxInferenceBoundaryEvidenceError(
            f"verification command failed: {Path(arguments[0]).name}"
        )
    return result.stdout


def git_revision(candidate: str) -> str:
    result = run_command(["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"], 20)
    revision = result.stdout.strip()
    if result.returncode != 0 or REVISION.fullmatch(revision) is None:
        raise LinuxInferenceBoundaryEvidenceError("source revision is unavailable")
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
        raise LinuxInferenceBoundaryEvidenceError(f"committed source absent: {relative}")
    return result.stdout


def source_records(revision: str) -> list[dict[str, Any]]:
    records = []
    for relative in SOURCE_PATHS:
        committed = committed_file(revision, relative)
        current = ROOT / relative
        if not current.is_file() or current.read_bytes() != committed:
            raise LinuxInferenceBoundaryEvidenceError(
                f"source differs from revision: {relative}"
            )
        records.append(
            {"path": relative, "sha256": sha256_bytes(committed), "bytes": len(committed)}
        )
    return records


def validate_adapter_manifest(manifest: Any) -> list[str]:
    if not isinstance(manifest, dict):
        return ["adapter Cargo manifest must be an object"]
    failures = []
    package = manifest.get("package", {})
    if package.get("name") != "agentmage-platform-linux-inference":
        failures.append("adapter package identity changed")
    dependencies = manifest.get("dependencies", {})
    if set(dependencies) != EXPECTED_DEPENDENCIES or dependencies.get(
        "agentmage-kernel-contracts"
    ) != {"workspace": True}:
        failures.append("adapter compile dependency closure changed")
    if manifest.get("dev-dependencies") or manifest.get("build-dependencies"):
        failures.append("adapter gained a development or build dependency")
    binaries = manifest.get("bin")
    if binaries != [{"name": "agentmage-native-inference", "path": "src/main.rs"}]:
        failures.append("adapter executable identity changed")
    return failures


def validate_source_boundary(root: Path = ROOT) -> list[str]:
    try:
        manifest = tomllib.loads((root / ADAPTER_MANIFEST).read_text(encoding="utf-8"))
        source = "\n".join(
            (root / relative).read_text(encoding="utf-8")
            for relative in (
                "platforms/linux-inference/src/lib.rs",
                "platforms/linux-inference/src/main.rs",
                "platforms/linux-inference/src/native_runtime.rs",
            )
        )
    except (OSError, UnicodeError, tomllib.TOMLDecodeError) as error:
        return [f"cannot read adapter source boundary: {error}"]
    failures = validate_adapter_manifest(manifest)
    for prohibited in FORBIDDEN_COMPILE_REFERENCES:
        if prohibited in source:
            failures.append(f"adapter gained forbidden compile reference: {prohibited}")
    return failures


def host_identity() -> dict[str, str]:
    values: dict[str, str] = {}
    for line in Path("/etc/os-release").read_text(encoding="utf-8").splitlines():
        if "=" in line:
            key, value = line.split("=", 1)
            values[key] = value.strip('"')
    identity = {
        "distribution": values.get("ID", "unknown"),
        "version": values.get("VERSION_ID", "unknown"),
        "architecture": platform.machine(),
    }
    if identity != {
        "distribution": "fedora",
        "version": "44",
        "architecture": "x86_64",
    }:
        raise LinuxInferenceBoundaryEvidenceError("host is not Fedora 44 x86_64")
    return identity


def package_records(output: Path) -> tuple[list[dict[str, Any]], dict[str, str]]:
    first = build_all(output, "0.0.0")
    second = build_all(output, "0.0.1")
    lifecycle = verify_extracted_candidates(first, second)
    records = []
    for kind in ("rpm", "deb", "vsix"):
        artifact = first[kind]
        records.append(
            {
                "format": kind,
                "sha256": sha256_file(artifact),
                "bytes": artifact.stat().st_size,
            }
        )
    return records, lifecycle


def build_report(revision: str) -> dict[str, Any]:
    boundary_failures = validate_source_boundary()
    if boundary_failures:
        raise LinuxInferenceBoundaryEvidenceError("; ".join(boundary_failures))
    require_command(
        ["cargo", "test", "-p", "agentmage-platform-linux-inference", "--locked"]
    )
    require_command(
        ["cargo", "test", "-p", "agentmage-host", "package_verify", "--locked"]
    )
    require_command(["python3", "-m", "unittest", "tests.test_package_candidate"])
    require_command(
        ["npm", "run", "build", "--workspace", "@agentmage/vscode-shell"]
    )
    require_command(["cargo", "build", "--release", "-p", "agentmage-host", "--locked"])
    require_command(
        [
            "cargo",
            "build",
            "--release",
            "-p",
            "agentmage-platform-linux-inference",
            "--bin",
            "agentmage-native-inference",
            "--locked",
        ]
    )
    adapter = ROOT / ADAPTER_BINARY
    adapter_metadata = adapter.stat()
    if not stat.S_ISREG(adapter_metadata.st_mode):
        raise LinuxInferenceBoundaryEvidenceError("adapter build output is not regular")
    descriptor_output = require_command([str(adapter), "--self-check"], 30)
    descriptor = json.loads(descriptor_output)
    if descriptor != EXPECTED_DESCRIPTOR:
        raise LinuxInferenceBoundaryEvidenceError("adapter descriptor changed")
    refused = run_command([str(adapter), "--serve"], 30)
    if refused.returncode == 0 or refused.stdout != (
        "agentmage.native-inference.operation-unavailable-no-admitted-profile\n"
    ):
        raise LinuxInferenceBoundaryEvidenceError("adapter did not refuse inference")
    with tempfile.TemporaryDirectory(prefix="agentmage-inference-package-") as directory:
        packages, lifecycle = package_records(Path(directory))
    model_state = json.loads(
        (ROOT / "evidence/current/model-activation-report.json").read_text(encoding="utf-8")
    )
    if model_state.get("enabled_profile_count") != 0 or model_state.get(
        "enabled_profiles"
    ) != []:
        raise LinuxInferenceBoundaryEvidenceError("a model was enabled")
    return {
        "schema_version": 1,
        "artifact_id": "linux-native-inference-package-boundary",
        "task_ids": ["9.1.1.6"],
        "status": "pass-fedora-package-boundary",
        "source_revision": revision,
        "sources": source_records(revision),
        "host": host_identity(),
        "compile_boundary": {
            "allowed_dependencies": sorted(EXPECTED_DEPENDENCIES),
            "kernel_engine": False,
            "workspace_authority": False,
            "tool_authority": False,
            "grant_authority": False,
            "credential_authority": False,
        },
        "process_boundary": descriptor,
        "adapter_binary": {
            "path_class": "package-owned-libexec",
            "sha256": sha256_file(adapter),
            "bytes": adapter_metadata.st_size,
            "mode": stat.S_IMODE(adapter_metadata.st_mode),
        },
        "packages": packages,
        "package_verification": lifecycle,
        "platform_status": {
            "fedora_44_x86_64": "verified-local",
            "ubuntu_26_04_x86_64": "format-built-and-extracted-not-clean-run",
            "macos": "blocked-macos",
        },
        "enabled_models": 0,
        "model_artifacts_packaged": False,
        "inference_started": False,
        "network_used": False,
        "private_values_present": False,
        "release_claim": "none",
        "limitations": LIMITATIONS,
    }


def validate_report(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["Linux inference boundary report must be an object"]
    failures = []
    if (
        value.get("schema_version") != 1
        or value.get("artifact_id") != "linux-native-inference-package-boundary"
        or value.get("task_ids") != ["9.1.1.6"]
        or value.get("status") != "pass-fedora-package-boundary"
        or REVISION.fullmatch(str(value.get("source_revision"))) is None
    ):
        failures.append("Linux inference boundary report identity changed")
    if value.get("compile_boundary") != {
        "allowed_dependencies": sorted(EXPECTED_DEPENDENCIES),
        "kernel_engine": False,
        "workspace_authority": False,
        "tool_authority": False,
        "grant_authority": False,
        "credential_authority": False,
    }:
        failures.append("adapter authority boundary changed")
    if value.get("process_boundary") != EXPECTED_DESCRIPTOR:
        failures.append("adapter process boundary changed")
    binary = value.get("adapter_binary", {})
    if (
        binary.get("path_class") != "package-owned-libexec"
        or SHA256.fullmatch(str(binary.get("sha256"))) is None
        or not isinstance(binary.get("bytes"), int)
        or binary.get("bytes", 0) <= 0
        or binary.get("mode") != 0o755
    ):
        failures.append("adapter binary identity is invalid")
    packages = value.get("packages")
    if (
        not isinstance(packages, list)
        or [item.get("format") for item in packages] != ["rpm", "deb", "vsix"]
        or any(
            SHA256.fullmatch(str(item.get("sha256"))) is None
            or not isinstance(item.get("bytes"), int)
            or item.get("bytes", 0) <= 0
            for item in packages
        )
    ):
        failures.append("package identity closure is invalid")
    if value.get("package_verification") != {
        "extracted_payloads": "pass",
        "mutation_refusal": "pass",
    }:
        failures.append("package verification closure changed")
    if value.get("platform_status") != {
        "fedora_44_x86_64": "verified-local",
        "ubuntu_26_04_x86_64": "format-built-and-extracted-not-clean-run",
        "macos": "blocked-macos",
    }:
        failures.append("platform evidence status changed")
    if (
        value.get("enabled_models") != 0
        or value.get("model_artifacts_packaged") is not False
        or value.get("inference_started") is not False
    ):
        failures.append("model or inference state was overclaimed")
    if (
        value.get("network_used") is not False
        or value.get("private_values_present") is not False
        or value.get("release_claim") != "none"
    ):
        failures.append("privacy, network, or release state was overclaimed")
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
    if value.get("limitations") != LIMITATIONS:
        failures.append("Linux inference limitations changed")
    return failures


def check_report() -> None:
    try:
        actual = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        revision = actual["source_revision"]
    except (OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise LinuxInferenceBoundaryEvidenceError(
            f"cannot read Linux inference boundary report: {error}"
        ) from error
    failures = validate_report(actual)
    if not isinstance(revision, str) or actual != build_report(revision):
        failures.append("Linux inference boundary report is stale or malformed")
    if failures:
        raise LinuxInferenceBoundaryEvidenceError("; ".join(failures))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    try:
        if arguments.write:
            revision = git_revision(arguments.source_revision)
            write_atomic(REPORT_PATH, canonical_json(build_report(revision)))
        check_report()
    except (
        OSError,
        UnicodeError,
        json.JSONDecodeError,
        LinuxInferenceBoundaryEvidenceError,
        subprocess.SubprocessError,
    ) as error:
        print(f"Linux inference boundary evidence failed: {error}", file=sys.stderr)
        return 1
    print("Linux native inference package boundary evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
