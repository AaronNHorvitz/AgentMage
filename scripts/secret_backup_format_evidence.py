#!/usr/bin/env python3
"""Build and validate Secret Service and encrypted-backup format evidence."""

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
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-11/story-11.1/secret-backup-format.json"
DOCUMENT_PATH: Final = ROOT / "docs/architecture/secret-store-encrypted-backup-format.md"
STORE_PATH: Final = ROOT / "kernel/engine/src/operational_store.rs"
SECRET_PATH: Final = ROOT / "platforms/linux/src/secret_service.rs"
LIFECYCLE_PATH: Final = ROOT / "platforms/linux/src/lifecycle.rs"
SOURCE_PATHS: Final = (
    "docs/architecture/secret-store-encrypted-backup-format.md",
    "kernel/engine/src/operational_store.rs",
    "platforms/linux/src/secret_service.rs",
    "platforms/linux/src/lifecycle.rs",
    "scripts/secret_backup_format_evidence.py",
    "tests/test_secret_backup_format_evidence.py",
)
DOCUMENT_FRAGMENTS: Final = (
    "**Backup format:** `agentmage-sqlcipher-backup-v1`",
    "makes no physical-overwrite claim. Interruption-safe key rotation is explicitly",
    "does not yet provision a dedicated backup-key identity",
    "backup-key lifecycle is therefore a required integration invariant, not a",
    "atomic publication or crash-complete cleanup of an in-progress backup;",
)
STORE_FRAGMENTS: Final = (
    "pub struct EncryptedBackupReceipt {",
    "/// Produces a separately keyed encrypted online backup and verifies its pages.",
    "let backup = rusqlite::backup::Backup::new(",
    "pub fn restore_to_fresh_candidate<",
    "open_current_keyed(backup, backup_key)",
    "encrypted_file_sha256: sha256_file(destination)?",
    "sqlite_artifact_paths(&backup)[1..]",
)
SECRET_FRAGMENTS: Final = (
    'const OPERATIONAL_STORE_KEY_PURPOSE: &str = "operational-store-key-v1";',
    ".env_clear()",
    ".stdin(if input_expected {",
    "pub struct LinuxOperationalStoreKeyProvider {",
    "with_decoded_operational_store_key(encoded, operation)",
    "fn destroy_key_and_verify_absent(&mut self)",
    "constant_time_equal(raw.as_slice(), decoded.as_slice())",
)
LIFECYCLE_FRAGMENTS: Final = (
    "pub fn provision_linux_operational_key(",
    '"linux.operational-key.state-without-key"',
    '"linux.operational-key.already-provisioned"',
    "pub fn rotate_linux_operational_key(",
    '"linux.operational-key.rotation-unavailable"',
)
COMMAND_SPECS: Final = (
    (("cargo", "test", "-p", "agentmage-kernel-engine", "operational_store", "--locked"), "23 passed; 0 failed"),
    (("cargo", "test", "-p", "agentmage-platform-linux", "secret_service", "--locked"), "7 passed; 0 failed; 3 ignored"),
    (("cargo", "test", "-p", "agentmage-platform-linux", "lifecycle", "--locked"), "2 passed; 0 failed"),
    (("python3", "-m", "unittest", "tests.test_secret_backup_format_evidence"), "Ran 4 tests"),
    (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
)
CLAIMS: Final = {
    "linux_secret_service_fixed_purpose": True,
    "secret_store_client_identity_reverified": True,
    "secret_store_value_uses_standard_input": True,
    "secret_value_in_arguments_or_environment": False,
    "operational_key_bytes": 32,
    "key_exposure_is_callback_bounded": True,
    "first_install_refuses_overwrite_and_state_without_key": True,
    "provisioned_key_retrieval_verified": True,
    "whole_store_key_deletion_verified_absent": True,
    "interruption_safe_key_rotation_available": False,
    "backup_format": "agentmage-sqlcipher-backup-v1",
    "backup_schema_version": 3,
    "backup_is_sqlcipher_database": True,
    "plaintext_outer_manifest_present": False,
    "successful_handoff_requires_sidecars": False,
    "receipt_hashes_closed_encrypted_main_file": True,
    "restore_writes_fresh_candidate_only": True,
    "backup_provider_supplied_separately": True,
    "distinct_backup_key_type_enforced": False,
    "production_backup_key_identity_provisioned": False,
    "crash_atomic_snapshot_publication": False,
    "live_secret_service_evidence_retained": False,
    "external_network_used": False,
    "release_support": False,
}
LIMITATIONS: Final = [
    "A distinct backup key is required by the integration contract but is not type-compared with the live key, and production Linux composition does not yet provision a dedicated backup-key identity.",
    "Interruption-safe key rotation is unavailable.",
    "The backup is not yet an atomically published snapshot family with an outer manifest, integrity tree, retention schedule, or crash-complete cleanup.",
    "Restore creates a verified fresh candidate but does not select or swap live authority, preserve a rollback point, or prove clean-device continuity.",
    "Three live Secret Service tests are environment-dependent and ignored by the standard gate.",
    "macOS Keychain, Windows credential storage, packaging, release acceptance, and later secret-canary and crash campaigns are not claimed.",
]
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class EvidenceError(ValueError):
    """Raised when secret/backup evidence is incomplete or overstated."""


def validate_sources(document: str, store: str, secret: str, lifecycle: str) -> list[str]:
    failures = []
    for label, value, fragments in (
        ("document", document, DOCUMENT_FRAGMENTS),
        ("store", store, STORE_FRAGMENTS),
        ("secret", secret, SECRET_FRAGMENTS),
        ("lifecycle", lifecycle, LIFECYCLE_FRAGMENTS),
    ):
        failures.extend(
            f"secret-backup {label} fragment changed: {index}"
            for index, fragment in enumerate(fragments, 1)
            if value.count(fragment) != 1
        )
    return failures


def git_revision(candidate: str) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"], cwd=ROOT,
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, timeout=30, check=False,
    )
    revision = result.stdout.strip()
    if result.returncode or REVISION.fullmatch(revision) is None:
        raise EvidenceError("source revision is unavailable")
    return revision


def git_bytes(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL, timeout=60, check=False,
    )
    if result.returncode or not result.stdout:
        raise EvidenceError("committed source is unavailable")
    return result.stdout


def command_record(arguments: tuple[str, ...], marker: str) -> dict[str, Any]:
    return {
        "command_id": hashlib.sha256("\0".join(arguments).encode()).hexdigest(),
        "expected_marker_sha256": hashlib.sha256(marker.encode()).hexdigest(),
        "exit_code": 0,
        "status": "pass",
    }


def expected_commands() -> list[dict[str, Any]]:
    return [command_record(arguments, marker) for arguments, marker in COMMAND_SPECS]


def run_checked(arguments: tuple[str, ...], marker: str) -> dict[str, Any]:
    result = subprocess.run(
        list(arguments), cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
        timeout=300, check=False, env={**os.environ, "LANG": "C", "LC_ALL": "C"},
    )
    if result.returncode or marker not in result.stdout + result.stderr:
        raise EvidenceError(f"verification command failed: {Path(arguments[0]).name}")
    return command_record(arguments, marker)


def source_records(revision: str) -> list[dict[str, Any]]:
    values: dict[str, str] = {}
    records = []
    for path in SOURCE_PATHS:
        data = git_bytes(revision, path)
        values[path] = data.decode()
        records.append({"bytes": len(data), "path": path, "sha256": hashlib.sha256(data).hexdigest()})
    failures = validate_sources(
        values[str(DOCUMENT_PATH.relative_to(ROOT))],
        values[str(STORE_PATH.relative_to(ROOT))],
        values[str(SECRET_PATH.relative_to(ROOT))],
        values[str(LIFECYCLE_PATH.relative_to(ROOT))],
    )
    if failures:
        raise EvidenceError("; ".join(failures))
    return records


def build_report(revision: str) -> dict[str, Any]:
    return {
        "artifact_id": "secret-store-encrypted-backup-format",
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "source_revision": revision,
        "sources": source_records(revision),
        "status": "pass-current-linux-secret-and-backup-format",
        "task_ids": ["11.1.2.3"],
        "verification_commands": [run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS],
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    failures = []
    exact = {
        "artifact_id": "secret-store-encrypted-backup-format",
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "status": "pass-current-linux-secret-and-backup-format",
        "task_ids": ["11.1.2.3"],
        "verification_commands": expected_commands(),
    }
    for key, value in exact.items():
        if report.get(key) != value:
            failures.append(f"secret-backup evidence {key} changed")
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        failures.append("secret-backup evidence revision is invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(SOURCE_PATHS):
        failures.append("secret-backup evidence sources changed")
    elif any(
        not isinstance(item.get("bytes"), int) or item["bytes"] <= 0
        or SHA256.fullmatch(str(item.get("sha256", ""))) is None for item in sources
    ):
        failures.append("secret-backup evidence source records are invalid")
    return failures


def validate_committed(report: dict[str, Any]) -> None:
    for item in report["sources"]:
        if hashlib.sha256(git_bytes(report["source_revision"], item["path"])).hexdigest() != item["sha256"]:
            raise EvidenceError("committed source binding changed")


def write_atomic(report: dict[str, Any]) -> None:
    REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(prefix=f".{REPORT_PATH.name}.", dir=REPORT_PATH.parent)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as stream:
            json.dump(report, stream, indent=2, sort_keys=True)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, REPORT_PATH)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    revision = git_revision(arguments.source_revision)
    if arguments.write:
        report = build_report(revision)
        write_atomic(report)
    else:
        report = json.loads(REPORT_PATH.read_text())
    failures = validate_report(report)
    if failures:
        raise EvidenceError("; ".join(failures))
    validate_committed(report)
    print("secret-backup format evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
