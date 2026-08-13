#!/usr/bin/env python3
"""Build and validate S-011-IT01 provider-substitution evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
from pathlib import Path
from typing import Any, Final

try:
    from scripts.evidence_core import atomic_write, canonical_json_bytes
except ModuleNotFoundError:
    from evidence_core import atomic_write, canonical_json_bytes

ROOT: Final = Path(__file__).resolve().parents[1]
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-11/story-11.1/s-011-it01.json"
SOURCE_PATHS: Final = (
    "Cargo.toml",
    "docs/verification/s-011-it01-provider-substitution-results.md",
    "kernel/engine/src/operational_store.rs",
    "kernel/engine/src/persistence.rs",
    "platforms/linux/src/platform.rs",
    "platforms/linux/src/secret_service.rs",
    "scripts/provider_substitution_evidence.py",
    "tests/test_provider_substitution_evidence.py",
)
CASE_IDS: Final = tuple(f"IT-{index:02}" for index in range(1, 9))
DOCUMENT_FRAGMENTS: Final = (
    "Pass for the current Fedora and Ubuntu implementation boundary",
    "Integration cases closed: **8 of 8**.",
    "Substituted client process launches: **0**.",
    "Ephemeral decisions selecting storage: **0**.",
    "No real user credential is read, written, listed, changed, or removed.",
    "manually deferred fuzzing remain separate gates.",
)
SOURCE_FRAGMENTS: Final = {
    "Cargo.toml": (
        'rusqlite = { version = "=0.40.2", default-features = false, features = ["backup", "bundled-sqlcipher"] }',
    ),
    "kernel/engine/src/operational_store.rs": (
        "fn plaintext_sqlite_substitution_never_becomes_operational_authority()",
        'query_row("PRAGMA cipher_version", [], |row| row.get(0))',
        "return Err(OperationalStoreError::CipherUnavailable);",
        'OperationalStore::open(&path, &observation(), &mut MissingKey)\n                .expect_err("missing key must fail")',
    ),
    "kernel/engine/src/persistence.rs": (
        "fn restricted_and_ephemeral_candidates_select_no_storage()",
        "fn every_raw_content_class_is_ephemeral_without_value_or_digest_receipt()",
        "assert_eq!(decision.receipt().encryption(), PersistenceEncryption::None);",
    ),
    "platforms/linux/src/platform.rs": (
        "provider: &mut LinuxOperationalStoreKeyProvider,",
        "fn every_linux_control_disablement_maps_to_fail_closed_startup_capabilities()",
    ),
    "platforms/linux/src/secret_service.rs": (
        "fn secret_bytes_never_enter_process_arguments_or_environment()",
        "fn substituted_secret_client_digest_fails_before_service_contact()",
        ".env_clear()",
        "let write_result = child.stdin.take().map(|mut stdin| stdin.write_all(secret));",
        "if verify_client(&self.manifest.client_path)? != self.manifest.client_sha256",
        "fn invalid_operational_store_key_never_invokes_database_callback()",
    ),
}
COMMAND_SPECS: Final = (
    (("cargo", "test", "-p", "agentmage-platform-linux", "--lib", "--locked"), "70 passed; 0 failed; 21 ignored"),
    (("cargo", "test", "-p", "agentmage-kernel-engine", "--lib", "--locked"), "136 passed; 0 failed; 1 ignored"),
    (("cargo", "clippy", "-p", "agentmage-platform-linux", "-p", "agentmage-kernel-engine", "--all-targets", "--locked", "--", "-D", "warnings"), "Finished `dev` profile"),
    (("python3", "-m", "unittest", "tests.test_provider_substitution_evidence"), "Ran 4 tests"),
    (("npm", "run", "strict-local-source:check"), "zero undeclared network paths"),
    (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
)
CLAIMS: Final = {
    "integration_case_count": 8,
    "missing_or_malformed_key_callback_count": 0,
    "substituted_secret_client_launch_count": 0,
    "plaintext_fallback_accepted_count": 0,
    "secret_argument_or_environment_occurrences": 0,
    "ephemeral_storage_selection_count": 0,
    "production_linux_provider_is_concrete": True,
    "bundled_sqlcipher_is_pinned": True,
    "cipher_version_is_mandatory": True,
    "live_secret_service_operations_executed": False,
    "malicious_toolchain_substitution_tested": False,
    "manual_fuzzing_executed": False,
    "external_network_used": False,
    "release_support": False,
}
LIMITATIONS: Final = [
    "No real user credential is read, written, listed, changed, or removed; live Secret Service operations remain environment-dependent ignored tests.",
    "The client-digest substitution test uses the installed root-owned /usr/bin/secret-tool identity and refuses before D-Bus contact.",
    "Pinned bundled SQLCipher, mandatory runtime cipher presence, and plaintext-file refusal do not prove resistance to a malicious compiler, linker, operating system, or hardware.",
    "macOS Keychain, Windows Credential Manager, packaging, release acceptance, support, and manually deferred fuzzing remain separate gates.",
]
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class EvidenceError(ValueError):
    pass


def validate_sources(values: dict[str, str]) -> list[str]:
    failures = []
    document = values[SOURCE_PATHS[1]]
    if tuple(re.findall(r"`(IT-\d{2})`", document)) != CASE_IDS:
        failures.append("S-011-IT01 case closure changed")
    failures.extend(
        f"S-011-IT01 document fragment changed: {index}"
        for index, fragment in enumerate(DOCUMENT_FRAGMENTS, 1)
        if document.count(fragment) != 1
    )
    for path, fragments in SOURCE_FRAGMENTS.items():
        failures.extend(
            f"S-011-IT01 source fragment changed: {path}:{index}"
            for index, fragment in enumerate(fragments, 1)
            if values[path].count(fragment) != 1
        )
    return failures


def git_revision(candidate: str) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"], cwd=ROOT,
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, timeout=30, check=False,
    )
    revision = result.stdout.strip()
    if result.returncode or REVISION.fullmatch(revision) is None:
        raise EvidenceError("source revision unavailable")
    return revision


def git_bytes(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL, timeout=60, check=False,
    )
    if result.returncode or not result.stdout:
        raise EvidenceError("committed source unavailable")
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
        raise EvidenceError(f"verification failed: {arguments[0]}")
    return command_record(arguments, marker)


def source_records(revision: str) -> list[dict[str, Any]]:
    values = {}
    records = []
    for path in SOURCE_PATHS:
        data = git_bytes(revision, path)
        values[path] = data.decode()
        records.append({"bytes": len(data), "path": path, "sha256": hashlib.sha256(data).hexdigest()})
    if failures := validate_sources(values):
        raise EvidenceError("; ".join(failures))
    return records


def build_report(revision: str) -> dict[str, Any]:
    return {
        "artifact_id": "s-011-it01-provider-substitution-results",
        "case_ids": list(CASE_IDS),
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "source_revision": revision,
        "sources": source_records(revision),
        "status": "pass-current-linux-provider-boundary",
        "task_ids": ["11.1.3.4", "S-011-IT01"],
        "verification_commands": [run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS],
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    exact = {
        "artifact_id": "s-011-it01-provider-substitution-results",
        "case_ids": list(CASE_IDS),
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "status": "pass-current-linux-provider-boundary",
        "task_ids": ["11.1.3.4", "S-011-IT01"],
        "verification_commands": expected_commands(),
    }
    failures = [f"S-011-IT01 evidence {key} changed" for key, value in exact.items() if report.get(key) != value]
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        failures.append("S-011-IT01 evidence revision invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(SOURCE_PATHS):
        failures.append("S-011-IT01 evidence sources changed")
    elif any(not isinstance(item.get("bytes"), int) or item["bytes"] <= 0 or SHA256.fullmatch(str(item.get("sha256", ""))) is None for item in sources):
        failures.append("S-011-IT01 evidence source records invalid")
    return failures


def validate_committed(report: dict[str, Any]) -> None:
    for item in report["sources"]:
        if hashlib.sha256(git_bytes(report["source_revision"], item["path"])).hexdigest() != item["sha256"]:
            raise EvidenceError("committed source binding changed")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    revision = git_revision(arguments.source_revision)
    report = build_report(revision) if arguments.write else json.loads(REPORT_PATH.read_text())
    if arguments.write:
        atomic_write(REPORT_PATH, canonical_json_bytes(report))
    failures = validate_report(report)
    if failures:
        raise EvidenceError("; ".join(failures))
    validate_committed(report)
    print("S-011-IT01 evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
