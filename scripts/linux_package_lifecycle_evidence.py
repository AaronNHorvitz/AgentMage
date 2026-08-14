#!/usr/bin/env python3
"""Build and validate clean Fedora and Ubuntu package lifecycle evidence."""

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
    from scripts.clean_build_evidence import validate_report as validate_clean_build_report
    from scripts.package_candidate import (
        MANIFEST_PATH,
        PAYLOAD_FILES,
        build_all,
        canonical_json,
        verify_payload,
    )
    from scripts.package_lifecycle import (
        extract_deb,
        extract_rpm,
        validate_container_lifecycle,
        verify_container_lifecycle,
        verify_extracted_candidates,
    )
except ModuleNotFoundError:
    from clean_build_evidence import validate_report as validate_clean_build_report
    from package_candidate import (
        MANIFEST_PATH,
        PAYLOAD_FILES,
        build_all,
        canonical_json,
        verify_payload,
    )
    from package_lifecycle import (
        extract_deb,
        extract_rpm,
        validate_container_lifecycle,
        verify_container_lifecycle,
        verify_extracted_candidates,
    )


ROOT: Final = Path(__file__).resolve().parents[1]
REPORT_PATH: Final = (
    ROOT
    / "artifacts"
    / "sprints"
    / "sprint-9"
    / "story-9.1"
    / "linux-clean-package-lifecycle.json"
)
CLEAN_BUILD_REPORT: Final = (
    ROOT
    / "artifacts"
    / "sprints"
    / "sprint-1"
    / "story-1.1"
    / "clean-build-report.json"
)
CLEAN_BUILD_POLICY: Final = ROOT / "architecture/clean-build-policy.json"
SOURCE_EXACT: Final = {
    ".cargo/config.toml",
    "Cargo.lock",
    "Cargo.toml",
    "LICENSE",
    "architecture/clean-build-policy.json",
    "package-lock.json",
    "package.json",
    "release/clean-build/Containerfile.linux",
    "rust-toolchain.toml",
    "rustfmt.toml",
    "scripts/clean_build_evidence.py",
    "scripts/clean_standard_build.py",
    "scripts/linux_package_lifecycle_evidence.py",
    "scripts/package_candidate.py",
    "scripts/package_lifecycle.py",
    "tests/test_clean_build_evidence.py",
    "tests/test_linux_package_lifecycle_evidence.py",
    "tests/test_package_candidate.py",
    "tests/test_package_lifecycle.py",
}
SOURCE_PREFIXES: Final = (
    "capabilities/read-only/",
    "kernel/contracts/",
    "kernel/engine/",
    "packaging/linux/",
    "platforms/linux/",
    "platforms/linux-inference/",
    "shells/host/",
    "shells/vscode/",
)
EXPECTED_PLATFORMS: Final = ("fedora-x86_64", "ubuntu-x86_64")
EXPECTED_FORMATS: Final = ("deb", "rpm", "vsix")
REVISION = re.compile(r"^[0-9a-f]{40}$")
OBJECT_ID = re.compile(r"^[0-9a-f]{40,64}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")
IMAGE_ID = re.compile(r"^sha256:[0-9a-f]{64}$")
LIMITATIONS: Final = [
    "The packages are unsigned pre-release candidates and make no supported-release claim.",
    "Clean Linux source builds and minimal package lifecycle runs are separate evidence stages bound to the same source revision.",
    "The minimal package environments launch the native host verifier and inactive inference boundary; they do not launch a graphical Visual Studio Code session.",
    "No model is packaged or enabled, and no inference operation is available in this increment.",
    "macOS package execution remains blocked on native Apple hardware under Decision 0003.",
]


class LinuxPackageLifecycleEvidenceError(ValueError):
    """Raised when Linux package lifecycle evidence is stale or overclaimed."""


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def run_command(
    arguments: list[str], timeout: int = 1800
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


def require_command(arguments: list[str], timeout: int = 1800) -> str:
    result = run_command(arguments, timeout)
    if result.returncode != 0:
        raise LinuxPackageLifecycleEvidenceError(
            f"verification command failed: {Path(arguments[0]).name}"
        )
    return result.stdout


def git_revision(candidate: str) -> str:
    result = run_command(["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"], 30)
    revision = result.stdout.strip()
    if result.returncode != 0 or REVISION.fullmatch(revision) is None:
        raise LinuxPackageLifecycleEvidenceError("source revision is unavailable")
    return revision


def committed_file(revision: str, relative: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{relative}"],
        cwd=ROOT,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=30,
        check=False,
    )
    if result.returncode != 0:
        raise LinuxPackageLifecycleEvidenceError(f"committed source absent: {relative}")
    return result.stdout


def source_paths(revision: str) -> tuple[str, ...]:
    output = require_command(["git", "ls-tree", "-r", "--name-only", revision], 30)
    paths = tuple(
        path
        for path in output.splitlines()
        if path in SOURCE_EXACT or path.startswith(SOURCE_PREFIXES)
    )
    if set(SOURCE_EXACT) - set(paths):
        raise LinuxPackageLifecycleEvidenceError("package source closure is incomplete")
    return paths


def source_identity(revision: str) -> dict[str, Any]:
    tree = require_command(["git", "rev-parse", f"{revision}^{{tree}}"], 30).strip()
    if OBJECT_ID.fullmatch(tree) is None:
        raise LinuxPackageLifecycleEvidenceError("source tree identity is unavailable")
    records = []
    for relative in source_paths(revision):
        committed = committed_file(revision, relative)
        current = ROOT / relative
        if not current.is_file() or current.read_bytes() != committed:
            raise LinuxPackageLifecycleEvidenceError(
                f"package source differs from revision: {relative}"
            )
        records.append(
            {
                "path": relative,
                "sha256": sha256_bytes(committed),
                "bytes": len(committed),
            }
        )
    return {"revision": revision, "tree": tree, "files": records}


def clean_build_record(revision: str) -> dict[str, Any]:
    try:
        report = json.loads(CLEAN_BUILD_REPORT.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise LinuxPackageLifecycleEvidenceError("clean build report is unavailable") from error
    failures = validate_clean_build_report(report, ROOT)
    if failures:
        raise LinuxPackageLifecycleEvidenceError("clean build report failed validation")
    if report.get("source", {}).get("revision") != revision:
        raise LinuxPackageLifecycleEvidenceError(
            "clean build report does not match the package source revision"
        )
    runs = report.get("platform_runs", {})
    if not isinstance(runs, dict):
        raise LinuxPackageLifecycleEvidenceError("clean build platform records are invalid")
    platforms = []
    for platform_id in EXPECTED_PLATFORMS:
        run = runs.get(platform_id)
        if not isinstance(run, dict) or run.get("status") != "pass":
            raise LinuxPackageLifecycleEvidenceError(
                f"clean build is incomplete for {platform_id}"
            )
        platforms.append(
            {
                "platform_id": platform_id,
                "status": "pass",
                "base_image": run.get("base_image"),
                "container_image_id": run.get("container_image_id"),
                "command_ids": [item.get("id") for item in run.get("commands", [])],
            }
        )
    return {
        "report_path": CLEAN_BUILD_REPORT.relative_to(ROOT).as_posix(),
        "report_sha256": sha256_file(CLEAN_BUILD_REPORT),
        "source_revision": revision,
        "linux_platforms": platforms,
        "network_boundary": report.get("container_controls", {}).get(
            "runtime_network"
        ),
        "macos_status": report.get("macos", {}).get("status"),
    }


def component_manifest(root: Path, version: str) -> dict[str, Any]:
    verify_payload(root, version=version)
    manifest_path = root / MANIFEST_PATH
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    return {
        "version": version,
        "sha256": sha256_file(manifest_path),
        "manifest": manifest,
    }


def package_records(
    artifacts: dict[str, dict[str, Path]], temporary: Path
) -> tuple[list[dict[str, Any]], list[dict[str, Any]], dict[str, str]]:
    records = []
    manifests = []
    lifecycle = verify_extracted_candidates(artifacts["0.0.0"], artifacts["0.0.1"])
    for version in ("0.0.0", "0.0.1"):
        for package_format in EXPECTED_FORMATS:
            artifact = artifacts[version][package_format]
            records.append(
                {
                    "version": version,
                    "format": package_format,
                    "filename": artifact.name,
                    "sha256": sha256_file(artifact),
                    "bytes": artifact.stat().st_size,
                }
            )
        rpm_root = temporary / f"rpm-{version}"
        deb_root = temporary / f"deb-{version}"
        rpm_root.mkdir()
        deb_root.mkdir()
        extract_rpm(artifacts[version]["rpm"], rpm_root)
        extract_deb(artifacts[version]["deb"], deb_root)
        rpm_manifest = component_manifest(rpm_root, version)
        deb_manifest = component_manifest(deb_root, version)
        if rpm_manifest != deb_manifest:
            raise LinuxPackageLifecycleEvidenceError(
                f"cross-format component manifest drifted for {version}"
            )
        manifests.append(rpm_manifest)
    return records, manifests, lifecycle


def build_report(revision: str) -> dict[str, Any]:
    source = source_identity(revision)
    clean_build = clean_build_record(revision)
    policy = json.loads(CLEAN_BUILD_POLICY.read_text(encoding="utf-8"))
    fedora_image = policy["linux_platforms"]["fedora-x86_64"]["base_image"]
    ubuntu_image = policy["linux_platforms"]["ubuntu-x86_64"]["base_image"]
    require_command(["npm", "run", "build", "--workspace", "@agentmage/vscode-shell"])
    require_command(["cargo", "build", "-p", "agentmage-host", "--release", "--locked"])
    require_command(
        [
            "cargo",
            "build",
            "-p",
            "agentmage-platform-linux-inference",
            "--bins",
            "--release",
            "--locked",
        ]
    )
    with tempfile.TemporaryDirectory(prefix="agentmage-package-evidence-") as directory:
        temporary = Path(directory)
        output = temporary / "packages"
        artifacts = {
            version: build_all(output, version) for version in ("0.0.0", "0.0.1")
        }
        repeated = build_all(temporary / "repeated", "0.0.0")
        if any(
            artifacts["0.0.0"][kind].read_bytes() != repeated[kind].read_bytes()
            for kind in EXPECTED_FORMATS
        ):
            raise LinuxPackageLifecycleEvidenceError("package rebuild drifted")
        packages, manifests, extracted = package_records(artifacts, temporary)
        container_results = verify_container_lifecycle(
            artifacts, fedora_image, ubuntu_image
        )
    lifecycle = container_results["container_lifecycle"]
    if lifecycle.get("status") != "pass":
        raise LinuxPackageLifecycleEvidenceError("clean package lifecycle did not pass")
    return {
        "schema_version": 1,
        "artifact_id": "linux-clean-package-lifecycle",
        "task_ids": ["9.1.1.7"],
        "status": "pass-linux-clean-package-lifecycle",
        "source": source,
        "clean_source_build": clean_build,
        "packages": packages,
        "component_manifests": manifests,
        "package_verification": extracted,
        "container_lifecycle": lifecycle,
        "package_build_network_used": False,
        "lifecycle_network_used": False,
        "private_values_present": False,
        "enabled_models": 0,
        "inference_available": False,
        "release_claim": "none",
        "limitations": LIMITATIONS,
    }


def validate_report(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["Linux package lifecycle report must be an object"]
    failures: list[str] = []
    source_value = value.get("source", {})
    source = source_value if isinstance(source_value, dict) else {}
    if (
        value.get("schema_version") != 1
        or value.get("artifact_id") != "linux-clean-package-lifecycle"
        or value.get("task_ids") != ["9.1.1.7"]
        or value.get("status") != "pass-linux-clean-package-lifecycle"
        or REVISION.fullmatch(str(source.get("revision"))) is None
        or OBJECT_ID.fullmatch(str(source.get("tree"))) is None
    ):
        failures.append("Linux package lifecycle report identity changed")
    source_files = source.get("files")
    if (
        not isinstance(source_files, list)
        or not source_files
        or any(not isinstance(item, dict) for item in source_files)
        or [item.get("path") for item in source_files]
        != sorted(item.get("path") for item in source_files)
        or any(
            SHA256.fullmatch(str(item.get("sha256"))) is None
            or not isinstance(item.get("bytes"), int)
            or item.get("bytes", -1) < 0
            for item in source_files
        )
    ):
        failures.append("package source closure is invalid")
    clean_build_value = value.get("clean_source_build", {})
    clean_build = clean_build_value if isinstance(clean_build_value, dict) else {}
    clean_platforms = clean_build.get("linux_platforms")
    if (
        clean_build.get("source_revision") != source.get("revision")
        or SHA256.fullmatch(str(clean_build.get("report_sha256"))) is None
        or clean_build.get("network_boundary") != "disabled-after-bootstrap"
        or not isinstance(clean_platforms, list)
        or any(not isinstance(item, dict) for item in clean_platforms)
        or [item.get("platform_id") for item in clean_platforms]
        != list(EXPECTED_PLATFORMS)
        or any(
            item.get("status") != "pass"
            or IMAGE_ID.fullmatch(str(item.get("container_image_id"))) is None
            for item in clean_platforms
        )
    ):
        failures.append("clean source build closure is invalid")
    packages = value.get("packages")
    expected_package_keys = [
        (version, package_format)
        for version in ("0.0.0", "0.0.1")
        for package_format in EXPECTED_FORMATS
    ]
    if (
        not isinstance(packages, list)
        or any(not isinstance(item, dict) for item in packages)
        or [(item.get("version"), item.get("format")) for item in packages]
        != expected_package_keys
        or any(
            SHA256.fullmatch(str(item.get("sha256"))) is None
            or not isinstance(item.get("bytes"), int)
            or item.get("bytes", 0) <= 0
            for item in packages
        )
    ):
        failures.append("package artifact closure is invalid")
    manifests = value.get("component_manifests")
    expected_paths = [path.as_posix() for path in sorted(PAYLOAD_FILES)]
    expected_modes = [0o755, 0o755, 0o755, 0o755, 0o755, 0o644, 0o644]
    if (
        not isinstance(manifests, list)
        or any(
            not isinstance(item, dict)
            or not isinstance(item.get("manifest"), dict)
            or not isinstance(item.get("manifest", {}).get("files"), list)
            or any(
                not isinstance(record, dict)
                for record in item.get("manifest", {}).get("files", [])
            )
            for item in manifests
        )
        or [item.get("version") for item in manifests] != ["0.0.0", "0.0.1"]
        or any(
            SHA256.fullmatch(str(item.get("sha256"))) is None
            or item.get("manifest", {}).get("schema_version") != 1
            or item.get("manifest", {}).get("record_type")
            != "agentmage-package-manifest"
            or item.get("manifest", {}).get("status") != "unsigned-candidate"
            or item.get("manifest", {}).get("package_id")
            != f"agentmage-linux-x86_64-{item.get('version')}-candidate"
            or [record.get("path") for record in item.get("manifest", {}).get("files", [])]
            != expected_paths
            or [record.get("mode") for record in item.get("manifest", {}).get("files", [])]
            != expected_modes
            or any(
                SHA256.fullmatch(str(record.get("sha256"))) is None
                or not isinstance(record.get("size"), int)
                or record.get("size", 0) <= 0
                for record in item.get("manifest", {}).get("files", [])
            )
            for item in manifests
        )
    ):
        failures.append("component manifest closure is invalid")
    lifecycle = value.get("container_lifecycle", {})
    if validate_container_lifecycle(lifecycle):
        failures.append("container lifecycle closure is invalid")
    if value.get("package_verification") != {
        "extracted_payloads": "pass",
        "mutation_refusal": "pass",
    }:
        failures.append("package verification closure changed")
    if (
        value.get("package_build_network_used") is not False
        or value.get("lifecycle_network_used") is not False
        or value.get("private_values_present") is not False
        or value.get("enabled_models") != 0
        or value.get("inference_available") is not False
        or value.get("release_claim") != "none"
    ):
        failures.append("network, private-value, model, or release state was overclaimed")
    if value.get("limitations") != LIMITATIONS:
        failures.append("Linux package lifecycle limitations changed")
    return failures


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(prefix=".agentmage-linux-package-", dir=path.parent)
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


def check_report() -> None:
    try:
        actual = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        revision = actual["source"]["revision"]
    except (OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise LinuxPackageLifecycleEvidenceError(
            "cannot read Linux package lifecycle report"
        ) from error
    failures = validate_report(actual)
    if not isinstance(revision, str) or actual != build_report(revision):
        failures.append("Linux package lifecycle report is stale or malformed")
    if failures:
        raise LinuxPackageLifecycleEvidenceError("; ".join(failures))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
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
        LinuxPackageLifecycleEvidenceError,
        subprocess.SubprocessError,
    ) as error:
        print(f"Linux package lifecycle evidence failed: {error}", file=sys.stderr)
        return 1
    print("Linux clean package lifecycle evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
