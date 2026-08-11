#!/usr/bin/env python3
"""Build and verify the immutable Story 4.1 kernel-contract source package."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import tarfile
import tempfile
import tomllib
from pathlib import Path, PurePosixPath
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
CRATE_NAME = "agentmage-kernel-contracts"
CRATE_VERSION = "0.0.0"
CONTRACT_SCHEMA_VERSION = 1
PREFIX = f"{CRATE_NAME}-{CRATE_VERSION}"
OUTPUT_DIR = ROOT / "artifacts/sprints/sprint-4/story-4.1"
ARCHIVE_PATH = OUTPUT_DIR / f"{PREFIX}.crate"
REPORT_PATH = OUTPUT_DIR / "kernel-contract-package-report.json"
MAX_ARCHIVE_BYTES = 2 * 1024 * 1024
MAX_MEMBER_BYTES = 1024 * 1024
MAX_TOTAL_MEMBER_BYTES = 4 * 1024 * 1024
SOURCE_MEMBERS = {
    "Cargo.toml.orig": "kernel/contracts/Cargo.toml",
    "LICENSE": "kernel/contracts/LICENSE",
    "README.md": "kernel/contracts/README.md",
    "rust-toolchain.toml": "kernel/contracts/rust-toolchain.toml",
    "src/approval.rs": "kernel/contracts/src/approval.rs",
    "src/boundary.rs": "kernel/contracts/src/boundary.rs",
    "src/common.rs": "kernel/contracts/src/common.rs",
    "src/display_link.rs": "kernel/contracts/src/display_link.rs",
    "src/evidence.rs": "kernel/contracts/src/evidence.rs",
    "src/grant.rs": "kernel/contracts/src/grant.rs",
    "src/ids.rs": "kernel/contracts/src/ids.rs",
    "src/lib.rs": "kernel/contracts/src/lib.rs",
    "src/network.rs": "kernel/contracts/src/network.rs",
    "src/path.rs": "kernel/contracts/src/path.rs",
    "src/platform.rs": "kernel/contracts/src/platform.rs",
    "src/platform_path.rs": "kernel/contracts/src/platform_path.rs",
    "src/prompt.rs": "kernel/contracts/src/prompt.rs",
    "src/serialization.rs": "kernel/contracts/src/serialization.rs",
    "src/task.rs": "kernel/contracts/src/task.rs",
    "src/tool.rs": "kernel/contracts/src/tool.rs",
    "tests/contract_family.rs": "kernel/contracts/tests/contract_family.rs",
}
EXCLUDED_CRATE_MEMBERS = (
    "kernel/contracts/tests/path_corpus.rs",
    "kernel/contracts/tests/platform_path_contract.rs",
)
GENERATED_MEMBERS = (".cargo_vcs_info.json", "Cargo.lock", "Cargo.toml")
EXPECTED_MEMBERS = tuple(sorted((*SOURCE_MEMBERS, *GENERATED_MEMBERS)))
DIRECT_DEPENDENCIES = {
    "serde": {"version": "=1.0.229", "features": ["derive"]},
    "serde_json": {"version": "=1.0.151"},
    "unicode-normalization": {"version": "=0.1.25"},
}
VERIFY_COMMANDS = (
    "cargo test --locked --offline --all-targets",
    "cargo clippy --locked --offline --all-targets -- -D warnings",
)


class PackageValidationError(ValueError):
    """Raised when the package cannot satisfy its closed review contract."""


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(prefix=".agentmage-contracts-", dir=path.parent)
    temporary = Path(temporary_name)
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


def safe_member_path(name: str) -> bool:
    path = PurePosixPath(name)
    return (
        not path.is_absolute()
        and ".." not in path.parts
        and len(path.parts) >= 2
        and path.parts[0] == PREFIX
        and str(path) == name
    )


def run(command: list[str], cwd: Path) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        command,
        cwd=cwd,
        check=False,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=120,
    )


def git_file(revision: str, path: str, root: Path = ROOT) -> bytes:
    completed = run(["git", "show", f"{revision}:{path}"], root)
    if completed.returncode != 0:
        raise PackageValidationError(f"package source revision does not contain {path}")
    return completed.stdout


def load_archive(path: Path) -> dict[str, tuple[bytes, int]]:
    try:
        archive_size = path.stat().st_size
    except OSError as error:
        raise PackageValidationError(f"cannot read package archive: {error}") from error
    if archive_size == 0 or archive_size > MAX_ARCHIVE_BYTES:
        raise PackageValidationError("package archive size is invalid")
    files: dict[str, tuple[bytes, int]] = {}
    total_member_bytes = 0
    try:
        with tarfile.open(path, mode="r:gz") as archive:
            for member in archive.getmembers():
                if not member.isfile() or not safe_member_path(member.name):
                    raise PackageValidationError("package contains a non-file or unsafe member")
                relative = PurePosixPath(member.name).relative_to(PREFIX).as_posix()
                if relative in files:
                    raise PackageValidationError("package contains a duplicate member")
                if member.size > MAX_MEMBER_BYTES or member.mode & 0o111:
                    raise PackageValidationError("package member is oversized or executable")
                total_member_bytes += member.size
                if total_member_bytes > MAX_TOTAL_MEMBER_BYTES:
                    raise PackageValidationError("package expands beyond its total size limit")
                handle = archive.extractfile(member)
                if handle is None:
                    raise PackageValidationError("package member cannot be read")
                content = handle.read(MAX_MEMBER_BYTES + 1)
                if len(content) != member.size:
                    raise PackageValidationError("package member size does not match its header")
                files[relative] = (content, member.mode)
    except (OSError, tarfile.TarError) as error:
        raise PackageValidationError(f"package archive is malformed: {error}") from error
    if tuple(sorted(files)) != EXPECTED_MEMBERS:
        raise PackageValidationError("package member closure is incomplete or broadened")
    return files


def validate_manifest(manifest: dict[str, Any]) -> None:
    package = manifest.get("package", {})
    expected_package = {
        "name": CRATE_NAME,
        "version": CRATE_VERSION,
        "edition": "2024",
        "rust-version": "1.95.0",
        "license": "Apache-2.0",
        "repository": "https://github.com/AaronNHorvitz/AgentMage",
        "publish": False,
        "exclude": ["tests/path_corpus.rs", "tests/platform_path_contract.rs"],
    }
    for field, expected in expected_package.items():
        if package.get(field) != expected:
            raise PackageValidationError(f"normalized package field is invalid: {field}")
    if package.get("build") is not False:
        raise PackageValidationError("package may not define a build script")
    if manifest.get("dependencies") != DIRECT_DEPENDENCIES:
        raise PackageValidationError("package dependencies are incomplete or broadened")
    if manifest.get("lints", {}).get("rust", {}).get("unsafe_code") != "forbid":
        raise PackageValidationError("package does not forbid unsafe Rust")


def package_revision(files: dict[str, tuple[bytes, int]]) -> str:
    try:
        metadata = json.loads(files[".cargo_vcs_info.json"][0])
        revision = metadata["git"]["sha1"]
        path_in_vcs = metadata["path_in_vcs"]
    except (KeyError, TypeError, json.JSONDecodeError) as error:
        raise PackageValidationError("package VCS metadata is malformed") from error
    if not isinstance(revision, str) or re.fullmatch(r"[0-9a-f]{40}", revision) is None:
        raise PackageValidationError("package source revision is invalid")
    if path_in_vcs != "kernel/contracts":
        raise PackageValidationError("package VCS source path is invalid")
    return revision


def validate_package(path: Path, root: Path = ROOT) -> tuple[dict[str, tuple[bytes, int]], str]:
    files = load_archive(path)
    revision = package_revision(files)
    ancestor = run(["git", "merge-base", "--is-ancestor", revision, "HEAD"], root)
    if ancestor.returncode != 0:
        raise PackageValidationError("package source revision is not an ancestor of HEAD")
    for member, repository_path in SOURCE_MEMBERS.items():
        if files[member][0] != git_file(revision, repository_path, root):
            raise PackageValidationError(f"packaged source differs from its revision: {member}")
    if git_file(revision, "LICENSE", root) != files["LICENSE"][0]:
        raise PackageValidationError("crate license differs from the repository license")
    if git_file(revision, "rust-toolchain.toml", root) != files["rust-toolchain.toml"][0]:
        raise PackageValidationError("crate toolchain differs from the repository toolchain")
    try:
        manifest = tomllib.loads(files["Cargo.toml"][0].decode("utf-8"))
        lock = tomllib.loads(files["Cargo.lock"][0].decode("utf-8"))
    except (UnicodeDecodeError, tomllib.TOMLDecodeError) as error:
        raise PackageValidationError("packaged Cargo metadata is malformed") from error
    validate_manifest(manifest)
    lock_names = {item.get("name") for item in lock.get("package", [])}
    if CRATE_NAME not in lock_names or not set(DIRECT_DEPENDENCIES).issubset(lock_names):
        raise PackageValidationError("packaged lockfile omits direct dependencies")
    common = files["src/common.rs"][0].decode("utf-8")
    version_match = re.search(r"CONTRACT_SCHEMA_VERSION: u16 = (\d+);", common)
    if version_match is None or int(version_match.group(1)) != CONTRACT_SCHEMA_VERSION:
        raise PackageValidationError("packaged contract schema version is invalid")
    return files, revision


def verify_unpacked(files: dict[str, tuple[bytes, int]]) -> None:
    with tempfile.TemporaryDirectory(prefix="agentmage-contract-package-") as temporary_name:
        package_root = Path(temporary_name) / PREFIX
        for relative, (content, mode) in files.items():
            destination = package_root / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(content)
            destination.chmod(mode & 0o666)
        commands = (
            ["cargo", "test", "--locked", "--offline", "--all-targets"],
            [
                "cargo",
                "clippy",
                "--locked",
                "--offline",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        )
        for command in commands:
            completed = run(command, package_root)
            if completed.returncode != 0:
                raise PackageValidationError(f"unpacked verification failed: {' '.join(command)}")


def build_report(path: Path, root: Path = ROOT) -> dict[str, Any]:
    files, revision = validate_package(path, root)
    verify_unpacked(files)
    return {
        "schema_version": 1,
        "task_id": "4.1.2.1",
        "artifact_id": "versioned-kernel-contract-package",
        "status": "pass-linux-source-package",
        "package": {
            "path": path.relative_to(root).as_posix(),
            "name": CRATE_NAME,
            "cargo_version": CRATE_VERSION,
            "contract_schema_version": CONTRACT_SCHEMA_VERSION,
            "format": "cargo-source-crate",
            "sha256": sha256_bytes(path.read_bytes()),
            "size_bytes": path.stat().st_size,
            "source_revision": revision,
            "source_path": "kernel/contracts",
            "license": "Apache-2.0",
            "publish": False,
        },
        "members": [
            {
                "path": relative,
                "sha256": sha256_bytes(content),
                "size_bytes": len(content),
                "executable": bool(mode & 0o111),
            }
            for relative, (content, mode) in sorted(files.items())
        ],
        "direct_dependencies": [
            {"name": name, **configuration}
            for name, configuration in sorted(DIRECT_DEPENDENCIES.items())
        ],
        "verification": {
            "commands": list(VERIFY_COMMANDS),
            "offline": True,
            "package_test": "pass",
            "package_clippy": "pass",
            "unsafe_rust": "forbidden",
            "network_authority": "none",
            "execution_authority": "none",
        },
        "platform_status": {
            "linux_source_package": "verified-local",
            "macos_source_package": "blocked-macos",
            "macos_implementation_claim": "none",
        },
        "limitations": [
            "This is an unpublished source-contract review package, not a product release.",
            "Cargo version 0.0.0 and wire contract schema version 1 are separate identities.",
            "No macOS build or execution evidence is claimed.",
            "The package contains no CapabilityGrant implementation or positive execution path.",
        ],
    }


def build_archive(root: Path = ROOT) -> None:
    with tempfile.TemporaryDirectory(prefix="agentmage-cargo-package-") as target_name:
        command = [
            "cargo",
            "package",
            "-p",
            CRATE_NAME,
            "--no-verify",
            "--locked",
            "--offline",
            "--target-dir",
            target_name,
        ]
        completed = run(command, root)
        if completed.returncode != 0:
            raise PackageValidationError("Cargo source packaging failed")
        generated = Path(target_name) / "package" / f"{PREFIX}.crate"
        if not generated.is_file():
            raise PackageValidationError("Cargo did not produce the expected source archive")
        OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
        write_atomic(ARCHIVE_PATH, generated.read_bytes())


def write_artifacts(root: Path = ROOT) -> None:
    build_archive(root)
    write_atomic(REPORT_PATH, canonical_json(build_report(ARCHIVE_PATH, root)))


def check_artifacts(root: Path = ROOT) -> None:
    try:
        actual = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise PackageValidationError(f"cannot read package report: {error}") from error
    expected = build_report(ARCHIVE_PATH, root)
    if actual != expected:
        raise PackageValidationError("kernel-contract package report is stale or malformed")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            write_artifacts()
        check_artifacts()
    except (OSError, PackageValidationError, subprocess.SubprocessError) as error:
        print(f"Kernel-contract package validation failed: {error}", file=sys.stderr)
        return 1
    print("Kernel-contract source package validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
