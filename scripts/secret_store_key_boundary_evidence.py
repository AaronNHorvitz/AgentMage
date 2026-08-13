#!/usr/bin/env python3
"""Build and validate Linux operational-store key-boundary evidence."""

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
REPORT_PATH: Final = (
    ROOT / "artifacts/sprints/sprint-11/story-11.1/secret-store-key-boundary.json"
)
SOURCE_PATHS: Final = (
    "docs/architecture/durable-authority-store.md",
    "docs/architecture/platform-adapter-contract.md",
    "kernel/engine/src/operational_store.rs",
    "platforms/linux/src/platform.rs",
    "platforms/linux/src/secret_service.rs",
    "scripts/secret_store_key_boundary_evidence.py",
    "tests/test_secret_store_key_boundary_evidence.py",
)
COMMAND_SPECS: Final = (
    (
        (
            "cargo",
            "test",
            "-p",
            "agentmage-kernel-engine",
            "operational_store::tests::encrypted_store_requires_key_and_hides_sqlite_header",
            "--locked",
        ),
        "1 passed; 0 failed",
    ),
    (
        ("cargo", "test", "-p", "agentmage-platform-linux", "--locked"),
        "68 passed; 0 failed; 21 ignored",
    ),
    (
        (
            "python3",
            "-m",
            "unittest",
            "tests.test_secret_store_key_boundary_evidence",
        ),
        "Ran 5 tests",
    ),
    (("npm", "run", "product:lint"), "Structural effect mediation boundary validated."),
)
BOUNDARY_PROFILE: Final = {
    "production_platform": "linux-fedora-ubuntu",
    "production_provider": "LinuxOperationalStoreKeyProvider",
    "secret_backend": "org.freedesktop.secrets",
    "secret_client": "secret-tool",
    "secret_identity": "operational-store-key-v1",
    "key_encoding": "64-byte-hex",
    "decoded_key_bytes": 32,
    "exposure": "closure-only-zeroizing-buffer",
    "primary_file_creation": "inside-successful-key-callback",
    "backup_file_creation": "inside-successful-key-callback",
}
CLAIMS: Final = {
    "production_linux_constructor_rejects_arbitrary_provider_types": True,
    "missing_primary_key_creates_no_database_file": True,
    "missing_backup_key_creates_no_backup_file": True,
    "malformed_key_never_invokes_database_callback": True,
    "key_is_not_returned_by_platform_provider_api": True,
    "test_only_generic_provider_is_excluded_from_normal_builds": True,
    "live_secret_service_round_trip_executed": False,
    "macos_or_windows_secret_store_implemented": False,
    "external_network_used": False,
    "release_support": False,
}
LIMITATIONS: Final = [
    "The deterministic gate validates the Linux production type boundary and unavailable or malformed key behavior without reading or writing a real user credential.",
    "Live Secret Service probe, round-trip, and operational-key provisioning tests remain explicitly ignored unless run in a suitable unlocked desktop session.",
    "The kernel retains a platform-neutral provider trait and the Linux test-support feature retains a synthetic provider constructor; neither is the normal Linux production entry point.",
    "Key rotation, deletion, uninstall orchestration, macOS Keychain, Windows Credential Manager, supported packaging, and release acceptance remain later work.",
]
REQUIRED_SOURCE_FRAGMENTS: Final = {
    "kernel/engine/src/operational_store.rs": (
        "provider\n            .with_key(|key| {\n                prepare_store_file(path)?;",
        "provider\n            .with_key(|key| {\n                prepare_new_store_file(destination)?;",
        "assert!(!backup.exists());",
    ),
    "platforms/linux/src/platform.rs": (
        "pub fn open_linux_authority(\n    verified: &VerifiedPlatformAdapter<LinuxPlatformAdapter>,\n    state_root: &Path,\n    provider: &mut LinuxOperationalStoreKeyProvider,",
        "#[cfg(feature = \"test-support\")]\npub fn open_test_linux_authority<P: OperationalStoreKeyProvider>",
    ),
    "platforms/linux/src/secret_service.rs": (
        "value.with_exposed(|encoded| with_decoded_operational_store_key(encoded, operation))",
        "let decoded = decode_operational_store_key(encoded)?;\n    Ok(operation(decoded.as_slice()))",
        "fn invalid_operational_store_key_never_invokes_database_callback()",
    ),
}
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class SecretStoreKeyEvidenceError(ValueError):
    """Raised when key-boundary evidence is incomplete or overstated."""


def validate_source(path: str, value: str) -> list[str]:
    fragments = REQUIRED_SOURCE_FRAGMENTS.get(path, ())
    return [
        f"secret-store key source fragment changed: {path}:{index}"
        for index, fragment in enumerate(fragments, start=1)
        if value.count(fragment) != 1
    ]


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
        raise SecretStoreKeyEvidenceError("source revision is unavailable")
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
        raise SecretStoreKeyEvidenceError("committed source is unavailable")
    return completed.stdout


def source_records(revision: str) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for path in SOURCE_PATHS:
        data = git_bytes(revision, path)
        try:
            value = data.decode("utf-8")
        except UnicodeError as error:
            raise SecretStoreKeyEvidenceError("committed source is invalid") from error
        if errors := validate_source(path, value):
            raise SecretStoreKeyEvidenceError("; ".join(errors))
        records.append(
            {"path": path, "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}
        )
    return records


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
        raise SecretStoreKeyEvidenceError(
            f"verification command failed: {Path(arguments[0]).name}"
        )
    return command_record(arguments, marker)


def build_report(revision: str) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "artifact_id": "linux-secret-store-key-boundary",
        "source_revision": revision,
        "task_ids": ["11.1.1.3"],
        "status": "pass-linux-production-key-boundary",
        "boundary_profile": BOUNDARY_PROFILE,
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
    failures: list[str] = []
    exact = {
        "schema_version": 1,
        "artifact_id": "linux-secret-store-key-boundary",
        "task_ids": ["11.1.1.3"],
        "status": "pass-linux-production-key-boundary",
        "boundary_profile": BOUNDARY_PROFILE,
        "verification_commands": expected_commands(),
        "claims": CLAIMS,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "external_network_used": False,
    }
    for key, value in exact.items():
        if report.get(key) != value:
            failures.append(f"secret-store key {key} changed")
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        failures.append("secret-store key source revision is invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(
        SOURCE_PATHS
    ):
        failures.append("secret-store key source evidence changed")
    elif any(
        not isinstance(item.get("bytes"), int)
        or item["bytes"] <= 0
        or SHA256.fullmatch(str(item.get("sha256", ""))) is None
        for item in sources
    ):
        failures.append("secret-store key source records are invalid")
    return failures


def validate_committed_sources(report: dict[str, Any]) -> None:
    revision = str(report["source_revision"])
    for item in report["sources"]:
        data = git_bytes(revision, item["path"])
        if hashlib.sha256(data).hexdigest() != item["sha256"]:
            raise SecretStoreKeyEvidenceError("committed source binding changed")
        if errors := validate_source(item["path"], data.decode("utf-8")):
            raise SecretStoreKeyEvidenceError("; ".join(errors))


def read_report(path: Path) -> dict[str, Any]:
    try:
        report = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise SecretStoreKeyEvidenceError("evidence report is unavailable") from error
    if not isinstance(report, dict):
        raise SecretStoreKeyEvidenceError("evidence report is invalid")
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
            raise SecretStoreKeyEvidenceError("; ".join(errors))
        write_atomic(REPORT_PATH, report)
    report = read_report(REPORT_PATH)
    if errors := validate_report(report):
        raise SecretStoreKeyEvidenceError("; ".join(errors))
    validate_committed_sources(report)
    print("Linux secret-store key boundary evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
