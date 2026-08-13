#!/usr/bin/env python3
"""Build and validate strict-local storage-detection fixture evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import tempfile
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
FIXTURE_PATH: Final = "fixtures/strict-local-storage/v1/detection-fixtures.json"
REPORT_PATH: Final = (
    ROOT / "artifacts/sprints/sprint-10/story-10.1/storage-detection-fixtures.json"
)
SOURCE_PATHS: Final = (
    "Cargo.lock",
    "docs/architecture/strict-local-boundary.md",
    FIXTURE_PATH,
    "kernel/engine/src/strict_local.rs",
    "platforms/linux/Cargo.toml",
    "platforms/linux/src/strict_local.rs",
    "scripts/storage_detection_fixture_evidence.py",
    "security/strict-local-source-policy.json",
    "tests/test_storage_detection_fixture_evidence.py",
)
COMMAND_SPECS: Final = (
    (
        (
            "cargo",
            "test",
            "-p",
            "agentmage-platform-linux",
            "strict_local_storage_fixture",
            "--locked",
        ),
        "2 passed; 0 failed",
    ),
    (
        (
            "cargo",
            "test",
            "-p",
            "agentmage-kernel-engine",
            "storage_policy_rejects_every_risky_or_unknown_observation",
            "--locked",
        ),
        "1 passed; 0 failed",
    ),
    (
        (
            "python3",
            "-m",
            "unittest",
            "tests.test_storage_detection_fixture_evidence",
        ),
        "Ran 5 tests",
    ),
    (
        ("npm", "run", "product:lint"),
        "Structural effect mediation boundary validated.",
    ),
)
FILESYSTEM_FIXTURES: Final = [
    ("ext-family", "0x0000ef53", "local"),
    ("xfs", "0x58465342", "local"),
    ("btrfs", "0x9123683e", "local"),
    ("tmpfs", "0x01021994", "local"),
    ("ramfs", "0x858458f6", "local"),
    ("f2fs", "0xf2f52010", "local"),
    ("erofs", "0xe0f5e1e2", "local"),
    ("zfs", "0x2fc12fc1", "local"),
    ("msdos", "0x00004d44", "local"),
    ("exfat", "0x2011bab0", "local"),
    ("ntfs3", "0x7366746e", "local"),
    ("nfs", "0x00006969", "remote"),
    ("cifs-smb", "0xff534d42", "remote"),
    ("nine-p", "0x01021997", "remote"),
    ("afs", "0x5346414f", "remote"),
    ("ceph", "0x00c36400", "remote"),
    ("ncp", "0x0000564c", "remote"),
    ("coda", "0x73757245", "remote"),
    ("fuse", "0x65735546", "fuse"),
    ("zero", "0x00000000", "unknown"),
    ("unrecognized", "0xdeadbeef", "unknown"),
    ("maximum", "0xffffffffffffffff", "unknown"),
]
PROVIDER_FIXTURES: Final = [
    ("Dropbox", "dropbox"),
    ("Dropbox (Personal)", "dropbox"),
    ("OneDrive", "onedrive"),
    ("OneDrive - Example Organization", "onedrive"),
    ("Google Drive", "google-drive"),
    ("Nextcloud", "nextcloud"),
    ("ownCloud", "owncloud"),
    ("iCloud Drive", "icloud-drive"),
    ("Mobile Documents", "icloud-drive"),
    ("Syncthing", "syncthing"),
    ("Box", "other-known"),
    ("Box Drive", "other-known"),
    ("Box Sync", "other-known"),
    ("MEGA", "other-known"),
    ("MEGAsync", "other-known"),
    ("pCloud Drive", "other-known"),
    ("Proton Drive", "other-known"),
    ("Tresorit", "other-known"),
]
SENTINEL_FIXTURES: Final = [
    (".stfolder", "syncthing"),
    (".dropbox.cache", "dropbox"),
    (".agentmage-cloud-synchronized", "other-known"),
]
ORDINARY_COMPONENTS: Final = [
    "ordinary-project",
    "drive",
    "proton",
    "megabyte",
    "cloud-analysis",
    "local-notes",
]
FIXTURE_PROFILE: Final = {
    "schema_version": 1,
    "filesystem_magic_count": 22,
    "known_local_count": 11,
    "known_remote_count": 7,
    "fuse_count": 1,
    "unknown_boundary_count": 3,
    "provider_component_count": 18,
    "root_sentinel_count": 3,
    "ordinary_component_count": 6,
    "provider_and_sentinel_directories_created": True,
}
CLAIMS: Final = {
    "versioned_fixture_corpus_implemented": True,
    "rust_executes_checked_fixture_corpus": True,
    "all_known_filesystem_magic_values_covered": True,
    "provider_and_sentinel_directories_tested": True,
    "live_remote_mount_tested": False,
    "all_sync_clients_detected": False,
    "external_network_used": False,
    "release_support": False,
}
LIMITATIONS: Final = [
    "Filesystem magic cases are deterministic classifier fixtures; no live NFS, CIFS, 9P, AFS, Ceph, NCP, Coda, or FUSE mount is claimed.",
    "Provider and sentinel cases use disposable owner-only local directories and do not claim detection of every synchronization client or a deliberately disguised synchronizer.",
    "Live remote-mount classification remains part of Sub-task 10.1.3.1, and no macOS, Windows, cross-platform, or release acceptance is claimed.",
]
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class StorageFixtureEvidenceError(ValueError):
    """Raised when storage fixture evidence is incomplete or overstated."""


def expected_fixture() -> dict[str, Any]:
    return {
        "filesystem_magic": [
            {"id": identifier, "magic": magic, "expected": expected}
            for identifier, magic, expected in FILESYSTEM_FIXTURES
        ],
        "ordinary_components": ORDINARY_COMPONENTS,
        "provider_components": [
            {"component": component, "expected": expected}
            for component, expected in PROVIDER_FIXTURES
        ],
        "root_sentinels": [
            {"name": name, "expected": expected}
            for name, expected in SENTINEL_FIXTURES
        ],
        "schema_version": 1,
    }


def validate_fixture(value: Any) -> list[str]:
    return [] if value == expected_fixture() else ["storage detection fixture changed"]


def git_revision(candidate: str) -> str:
    completed = subprocess.run(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True,
        timeout=30,
        check=False,
    )
    revision = completed.stdout.strip()
    if completed.returncode != 0 or REVISION.fullmatch(revision) is None:
        raise StorageFixtureEvidenceError("source revision is unavailable")
    return revision


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
        raise StorageFixtureEvidenceError("committed source is unavailable")
    return completed.stdout


def validate_committed_fixture(revision: str) -> str:
    data = git_bytes(revision, FIXTURE_PATH)
    try:
        value = json.loads(data)
    except (UnicodeError, json.JSONDecodeError) as error:
        raise StorageFixtureEvidenceError("committed fixture is invalid") from error
    if errors := validate_fixture(value):
        raise StorageFixtureEvidenceError("; ".join(errors))
    return hashlib.sha256(data).hexdigest()


def source_records(revision: str) -> list[dict[str, Any]]:
    return [
        {
            "path": path,
            "bytes": len(data := git_bytes(revision, path)),
            "sha256": hashlib.sha256(data).hexdigest(),
        }
        for path in SOURCE_PATHS
    ]


def command_record(arguments: tuple[str, ...], marker: str) -> dict[str, Any]:
    return {
        "command_id": hashlib.sha256("\0".join(arguments).encode()).hexdigest(),
        "exit_code": 0,
        "expected_marker_sha256": hashlib.sha256(marker.encode()).hexdigest(),
        "status": "pass",
    }


def expected_commands() -> list[dict[str, Any]]:
    return [command_record(arguments, marker) for arguments, marker in COMMAND_SPECS]


def run_checked(arguments: tuple[str, ...], marker: str) -> dict[str, Any]:
    completed = subprocess.run(
        list(arguments),
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        timeout=300,
        check=False,
        env={**os.environ, "LANG": "C", "LC_ALL": "C"},
    )
    if completed.returncode != 0 or marker not in completed.stdout + completed.stderr:
        raise StorageFixtureEvidenceError(
            f"verification command failed: {Path(arguments[0]).name}"
        )
    return command_record(arguments, marker)


def build_report(revision: str) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "artifact_id": "strict-local-storage-detection-fixtures",
        "source_revision": revision,
        "task_ids": ["10.1.2.4"],
        "status": "pass-versioned-storage-detection-fixture-corpus",
        "fixture_sha256": validate_committed_fixture(revision),
        "fixture_profile": FIXTURE_PROFILE,
        "verification_commands": [
            run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS
        ],
        "claims": CLAIMS,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "external_network_used": False,
        "sources": source_records(revision),
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    exact = {
        "schema_version": 1,
        "artifact_id": "strict-local-storage-detection-fixtures",
        "task_ids": ["10.1.2.4"],
        "status": "pass-versioned-storage-detection-fixture-corpus",
        "fixture_profile": FIXTURE_PROFILE,
        "verification_commands": expected_commands(),
        "claims": CLAIMS,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "external_network_used": False,
    }
    for key, value in exact.items():
        if report.get(key) != value:
            errors.append(f"storage fixture {key} changed")
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        errors.append("storage fixture source revision is invalid")
    if SHA256.fullmatch(str(report.get("fixture_sha256", ""))) is None:
        errors.append("storage fixture identity is invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(
        SOURCE_PATHS
    ):
        errors.append("storage fixture source evidence changed")
    elif any(
        not isinstance(item.get("bytes"), int)
        or item["bytes"] <= 0
        or SHA256.fullmatch(str(item.get("sha256", ""))) is None
        for item in sources
    ):
        errors.append("storage fixture source records are invalid")
    else:
        fixture_record = sources[SOURCE_PATHS.index(FIXTURE_PATH)]
        if fixture_record["sha256"] != report.get("fixture_sha256"):
            errors.append("storage fixture identity changed")
    return errors


def validate_committed_sources(report: dict[str, Any]) -> None:
    revision = str(report["source_revision"])
    for item in report["sources"]:
        if hashlib.sha256(git_bytes(revision, item["path"])).hexdigest() != item["sha256"]:
            raise StorageFixtureEvidenceError("committed source binding changed")
    if validate_committed_fixture(revision) != report["fixture_sha256"]:
        raise StorageFixtureEvidenceError("committed fixture binding changed")


def read_report(path: Path) -> dict[str, Any]:
    try:
        report = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise StorageFixtureEvidenceError("evidence report is unavailable") from error
    if not isinstance(report, dict):
        raise StorageFixtureEvidenceError("evidence report is invalid")
    return report


def write_atomic(path: Path, report: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as stream:
            json.dump(report, stream, indent=2, sort_keys=True)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    if arguments.write:
        report = build_report(git_revision(arguments.source_revision))
        if errors := validate_report(report):
            raise StorageFixtureEvidenceError("; ".join(errors))
        write_atomic(REPORT_PATH, report)
    report = read_report(REPORT_PATH)
    if errors := validate_report(report):
        raise StorageFixtureEvidenceError("; ".join(errors))
    validate_committed_sources(report)
    print("Strict-local storage-detection fixture evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
