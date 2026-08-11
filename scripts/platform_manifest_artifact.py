#!/usr/bin/env python3
"""Validate platform API/manifests and emit bounded Sprint 7 evidence."""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import os
import re
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
REPORT_PATH = ROOT / "artifacts/sprints/sprint-7/story-7.1/platform-contract-report.json"
MANIFEST_PATHS = (
    "release/platform-manifests/v1/fedora-x86_64.json",
    "release/platform-manifests/v1/ubuntu-x86_64.json",
)
SOURCE_PATHS = (
    "docs/architecture/platform-adapter-contract.md",
    "kernel/contracts/src/lib.rs",
    "kernel/contracts/src/platform.rs",
    "kernel/engine/src/lib.rs",
    "kernel/engine/src/platform_startup.rs",
    "kernel/engine/tests/platform_adapter_conformance.rs",
    "release/platform-manifests/README.md",
    *MANIFEST_PATHS,
    "scripts/platform_manifest_artifact.py",
    "tests/test_platform_manifest_artifact.py",
)
UBUNTU_IMAGE = "localhost/agentmage-clean-build:ubuntu-x86_64"
CAPABILITIES = (
    "workspace-authorization",
    "secure-path-resolution",
    "tool-confinement",
    "secret-storage",
    "process-limits",
    "local-inference",
    "model-installation",
    "packaging",
    "updates",
    "network-isolation",
)
TOP_LEVEL_FIELDS = {
    "schema_version",
    "record_type",
    "manifest_id",
    "status",
    "platform",
    "adapter_api_version",
    "runtime_identities",
    "capabilities",
    "dependency_classes",
    "credential_values_present",
    "private_environment_values_present",
    "macos_evidence_substituted",
    "release_claim",
}
EXPECTED_PLATFORMS = {
    "fedora-x86_64.json": ("fedora", "44", "rpm"),
    "ubuntu-x86_64.json": ("ubuntu", "26.04", "deb"),
}
HASH = re.compile(r"^[0-9a-f]{64}$")
REVISION = re.compile(r"^[0-9a-f]{40}$")
FORBIDDEN_KEYS = {
    "api_key",
    "home_directory",
    "hostname",
    "password",
    "private_key",
    "secret",
    "token",
    "username",
}


class PlatformManifestError(ValueError):
    """Raised when a platform contract is stale or overclaimed."""


def pretty_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, separators=(",", ":"), sort_keys=True) + "\n").encode()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def nested_keys(value: Any) -> set[str]:
    keys: set[str] = set()
    if isinstance(value, dict):
        for key, item in value.items():
            keys.add(key)
            keys.update(nested_keys(item))
    elif isinstance(value, list):
        for item in value:
            keys.update(nested_keys(item))
    return keys


def validate_manifest(value: Any, filename: str) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict):
        return [f"{filename}: manifest must be an object"]
    if set(value) != TOP_LEVEL_FIELDS:
        return [f"{filename}: top-level fields changed"]
    expected = EXPECTED_PLATFORMS.get(filename)
    if expected is None:
        return [f"{filename}: platform fixture is undeclared"]
    family, version, package_format = expected
    if (
        value["schema_version"] != 1
        or value["record_type"] != "platform-release-manifest"
        or value["status"] != "contract-fixture"
        or value["adapter_api_version"] != 1
    ):
        failures.append(f"{filename}: manifest identity changed")
    platform = value["platform"]
    if not isinstance(platform, dict) or set(platform) != {
        "family", "architecture", "supported_distributions", "os_build"
    }:
        failures.append(f"{filename}: platform identity fields changed")
    else:
        expected_distribution = [{"id": family, "version": version}]
        if (
            platform["family"] != family
            or platform["architecture"] != "x86_64"
            or platform["supported_distributions"] != expected_distribution
            or not isinstance(platform["os_build"], dict)
            or set(platform["os_build"]) != {"id", "sha256"}
            or not HASH.fullmatch(str(platform["os_build"].get("sha256")))
        ):
            failures.append(f"{filename}: platform identity is invalid")
    runtime = value["runtime_identities"]
    if not isinstance(runtime, dict) or set(runtime) != {
        "product_toolchain", "vscode", "package"
    }:
        failures.append(f"{filename}: runtime identity fields changed")
    else:
        if set(runtime["product_toolchain"]) != {"id", "sha256"}:
            failures.append(f"{filename}: toolchain identity is malformed")
        if set(runtime["vscode"]) != {"version", "commit", "sha256"}:
            failures.append(f"{filename}: VS Code identity is malformed")
        package = runtime["package"]
        if (
            set(package) != {"format", "sha256", "identity_class"}
            or package["format"] != package_format
            or package["identity_class"] != "synthetic-contract-fixture"
            or not HASH.fullmatch(str(package["sha256"]))
        ):
            failures.append(f"{filename}: package identity is invalid")
        for record_name, record in runtime.items():
            if not isinstance(record, dict) or not HASH.fullmatch(str(record.get("sha256"))):
                failures.append(f"{filename}: runtime hash is invalid: {record_name}")
    capabilities = value["capabilities"]
    names = [item.get("capability") for item in capabilities] if isinstance(capabilities, list) else []
    if tuple(names) != CAPABILITIES:
        failures.append(f"{filename}: capability closure or order changed")
    else:
        for item in capabilities:
            if (
                set(item) != {"capability", "mechanism", "sha256"}
                or not isinstance(item["mechanism"], str)
                or not item["mechanism"]
                or not HASH.fullmatch(str(item["sha256"]))
            ):
                failures.append(f"{filename}: capability evidence is malformed")
        packaging = capabilities[CAPABILITIES.index("packaging")]
        if packaging["sha256"] != runtime["package"]["sha256"]:
            failures.append(f"{filename}: packaging capability is not package-bound")
    dependency_classes = value["dependency_classes"]
    if not isinstance(dependency_classes, dict) or set(dependency_classes) != {
        "maintainer_release", "end_user_runtime", "excluded_end_user"
    }:
        failures.append(f"{filename}: dependency classes changed")
    else:
        for class_name, entries in dependency_classes.items():
            if (
                not isinstance(entries, list)
                or not entries
                or entries != sorted(set(entries))
            ):
                failures.append(f"{filename}: dependency class is not a sorted set: {class_name}")
        if set(dependency_classes["maintainer_release"]) & set(
            dependency_classes["end_user_runtime"]
        ):
            failures.append(f"{filename}: maintainer and end-user dependencies overlap")
        if set(dependency_classes["end_user_runtime"]) & set(
            dependency_classes["excluded_end_user"]
        ):
            failures.append(f"{filename}: excluded dependency is required at runtime")
    if nested_keys(value) & FORBIDDEN_KEYS:
        failures.append(f"{filename}: manifest contains a forbidden private field")
    if (
        value["credential_values_present"] is not False
        or value["private_environment_values_present"] is not False
        or value["macos_evidence_substituted"] is not False
        or value["release_claim"] != "none"
    ):
        failures.append(f"{filename}: manifest made an unsupported or private-data claim")
    return failures


def run_command(argv: list[str], root: Path = ROOT, timeout: int = 240) -> str:
    completed = subprocess.run(
        argv,
        cwd=root,
        check=False,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=timeout,
    )
    if completed.returncode != 0:
        raise PlatformManifestError(f"platform contract command failed: {argv[0]}")
    return completed.stdout


def git_revision(candidate: str = "HEAD", root: Path = ROOT) -> str:
    completed = subprocess.run(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"],
        cwd=root,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=10,
    )
    revision = completed.stdout.decode("ascii", "strict").strip()
    if completed.returncode != 0 or REVISION.fullmatch(revision) is None:
        raise PlatformManifestError("platform contract source revision is unavailable")
    return revision


def git_file(revision: str, relative: str, root: Path = ROOT) -> bytes:
    completed = subprocess.run(
        ["git", "show", f"{revision}:{relative}"],
        cwd=root,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=10,
    )
    if completed.returncode != 0:
        raise PlatformManifestError(f"platform contract source is absent: {relative}")
    return completed.stdout


def git_archive(revision: str, destination: Path, root: Path = ROOT) -> None:
    completed = subprocess.run(
        ["git", "archive", "--format=tar", revision],
        cwd=root,
        check=False,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=60,
    )
    if completed.returncode != 0:
        raise PlatformManifestError("could not archive platform contract source")
    with tarfile.open(fileobj=io.BytesIO(completed.stdout), mode="r:") as archive:
        archive.extractall(destination, filter="data")


def image_identity(root: Path = ROOT) -> str:
    value = run_command(
        ["podman", "image", "inspect", UBUNTU_IMAGE, "--format", "{{.Id}}"],
        root,
        60,
    ).strip()
    if HASH.fullmatch(value):
        value = f"sha256:{value}"
    if re.fullmatch(r"sha256:[0-9a-f]{64}", value) is None:
        raise PlatformManifestError("Ubuntu image identity is invalid")
    return value


def run_host_contract(root: Path = ROOT) -> None:
    run_command(
        [
            "cargo", "test", "--locked", "--offline", "-p", "agentmage-kernel-engine",
            "--test", "platform_adapter_conformance", "--", "--test-threads=1",
        ],
        root,
    )


def run_ubuntu_contract(revision: str, root: Path = ROOT) -> str:
    registry = (Path.home() / ".cargo/registry").resolve()
    if not registry.is_dir():
        raise PlatformManifestError("read-only Cargo dependency cache is unavailable")
    with tempfile.TemporaryDirectory(prefix="agentmage-ubuntu-platform-") as temporary:
        source = Path(temporary) / "source"
        source.mkdir()
        source.chmod(0o755)
        git_archive(revision, source, root)
        run_command(
            [
                "podman", "run", "--rm", "--network", "none", "--read-only",
                "--cap-drop=all", "--security-opt=no-new-privileges",
                "--security-opt=label=disable", "--pids-limit=256", "--memory=2g",
                "--tmpfs=/tmp:rw,exec,nosuid,nodev,size=2147483648",
                "--mount", f"type=bind,src={source},dst=/source,ro=true",
                "--mount", f"type=bind,src={registry},dst=/registry,ro=true",
                "--user=10001:10001", UBUNTU_IMAGE, "sh", "-lc",
                "mkdir -p /tmp/cargo /tmp/work /tmp/target"
                " && cp -a /registry /tmp/cargo/registry"
                " && cp -a /source/. /tmp/work/"
                " && cd /tmp/work"
                " && CARGO_HOME=/tmp/cargo CARGO_TARGET_DIR=/tmp/target"
                " CARGO_NET_OFFLINE=true cargo test --locked --offline"
                " -p agentmage-kernel-engine --test platform_adapter_conformance"
                " -- --test-threads=1",
            ],
            root,
        )
    return image_identity(root)


def manifest_summary(path: Path) -> dict[str, Any]:
    value = read_json(path)
    failures = validate_manifest(value, path.name)
    if failures:
        raise PlatformManifestError("; ".join(failures))
    canonical_first = canonical_json(value)
    canonical_second = canonical_json(read_json(path))
    if canonical_first != canonical_second:
        raise PlatformManifestError(f"manifest canonicalization changed: {path.name}")
    dependencies = value["dependency_classes"]
    return {
        "path": path.relative_to(ROOT).as_posix(),
        "platform": value["platform"]["family"],
        "architecture": value["platform"]["architecture"],
        "manifest_sha256": sha256_bytes(canonical_first),
        "repeat_manifest_sha256": sha256_bytes(canonical_second),
        "package_sha256": value["runtime_identities"]["package"]["sha256"],
        "capability_count": len(value["capabilities"]),
        "maintainer_dependency_count": len(dependencies["maintainer_release"]),
        "end_user_dependency_count": len(dependencies["end_user_runtime"]),
        "excluded_end_user_dependency_count": len(dependencies["excluded_end_user"]),
        "release_claim": value["release_claim"],
    }


def build_report(reference_revision: str, root: Path = ROOT) -> dict[str, Any]:
    ancestor = subprocess.run(
        ["git", "merge-base", "--is-ancestor", reference_revision, "HEAD"],
        cwd=root,
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        timeout=10,
    )
    if ancestor.returncode != 0:
        raise PlatformManifestError("platform contract revision is not an ancestor of HEAD")
    sources = []
    for relative in SOURCE_PATHS:
        committed = git_file(reference_revision, relative, root)
        if committed != (root / relative).read_bytes():
            raise PlatformManifestError(f"source differs from reference revision: {relative}")
        sources.append({"path": relative, "sha256": sha256_bytes(committed)})
    summaries = [manifest_summary(root / relative) for relative in MANIFEST_PATHS]
    run_host_contract(root)
    ubuntu_image_id = run_ubuntu_contract(reference_revision, root)
    return {
        "schema_version": 1,
        "task_id": "7.1.2",
        "artifact_id": "platform-adapter-contract-and-manifest-evidence",
        "status": "pass-all-available-non-macos-contract-scope",
        "reference_revision": reference_revision,
        "sources": sources,
        "api": {
            "version": 1,
            "required_capability_count": len(CAPABILITIES),
            "startup_failure_class_count": 11,
            "operating_system_branches_in_kernel_selector": 0,
        },
        "manifests": summaries,
        "conformance": {
            "deterministic_fake": "verified",
            "fedora_contract": "verified-local",
            "ubuntu_contract": "verified-no-network-container",
            "ubuntu_image": UBUNTU_IMAGE,
            "ubuntu_image_id": ubuntu_image_id,
            "macos": "blocked-macos",
            "equivalent_available_contract_result_count": 3,
            "networked_test_count": 0,
        },
        "maintainer_credentials_recorded": False,
        "private_environment_values_recorded": False,
        "macos_evidence_substituted": False,
        "release_claim": "none",
        "limitations": [
            "Linux fixtures validate manifest and shared startup semantics, not native mechanism readiness",
            "macOS implementation, package identity, and execution remain blocked",
        ],
    }


def validate_report(value: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict):
        return ["platform contract report must be an object"]
    if (
        value.get("schema_version") != 1
        or value.get("task_id") != "7.1.2"
        or value.get("artifact_id") != "platform-adapter-contract-and-manifest-evidence"
        or value.get("status") != "pass-all-available-non-macos-contract-scope"
        or REVISION.fullmatch(str(value.get("reference_revision"))) is None
    ):
        failures.append("platform contract report identity changed")
    api = value.get("api", {})
    if api != {
        "version": 1,
        "required_capability_count": 10,
        "startup_failure_class_count": 11,
        "operating_system_branches_in_kernel_selector": 0,
    }:
        failures.append("platform API evidence changed")
    manifests = value.get("manifests")
    if (
        not isinstance(manifests, list)
        or len(manifests) != 2
        or {item.get("platform") for item in manifests} != {"fedora", "ubuntu"}
        or any(item.get("capability_count") != 10 for item in manifests)
        or any(item.get("manifest_sha256") != item.get("repeat_manifest_sha256") for item in manifests)
        or any(item.get("release_claim") != "none" for item in manifests)
    ):
        failures.append("platform manifest evidence is incomplete")
    conformance = value.get("conformance", {})
    if (
        conformance.get("deterministic_fake") != "verified"
        or conformance.get("fedora_contract") != "verified-local"
        or conformance.get("ubuntu_contract") != "verified-no-network-container"
        or conformance.get("macos") != "blocked-macos"
        or conformance.get("equivalent_available_contract_result_count") != 3
        or conformance.get("networked_test_count") != 0
    ):
        failures.append("platform conformance evidence changed")
    if (
        value.get("maintainer_credentials_recorded") is not False
        or value.get("private_environment_values_recorded") is not False
        or value.get("macos_evidence_substituted") is not False
        or value.get("release_claim") != "none"
    ):
        failures.append("platform contract report made an unsupported claim")
    if not isinstance(value.get("limitations"), list) or len(value["limitations"]) != 2:
        failures.append("platform contract limitations are incomplete")
    return failures


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(prefix=".agentmage-platform-", dir=path.parent)
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


def write_report(reference_revision: str, root: Path = ROOT) -> None:
    write_atomic(REPORT_PATH, pretty_json(build_report(reference_revision, root)))


def check_report(root: Path = ROOT) -> None:
    try:
        actual = read_json(REPORT_PATH)
        revision = actual["reference_revision"]
    except (OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise PlatformManifestError(f"cannot read platform contract report: {error}") from error
    failures = validate_report(actual)
    if not isinstance(revision, str) or actual != build_report(revision, root):
        failures.append("platform contract report is stale or malformed")
    if failures:
        raise PlatformManifestError("; ".join(failures))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    args = parser.parse_args()
    try:
        if args.write:
            write_report(git_revision(args.source_revision))
        check_report()
    except (OSError, UnicodeError, PlatformManifestError, subprocess.SubprocessError) as error:
        print(f"Platform contract evidence failed: {error}", file=sys.stderr)
        return 1
    print("Sprint 7 platform contract passed on every available non-macOS contract target")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
