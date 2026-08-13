#!/usr/bin/env python3
"""Build and validate pre-persistence policy evidence."""

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
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-11/story-11.1/persistence-policy.json"
SOURCE_PATHS: Final = (
    "docs/architecture/durable-authority-store.md",
    "kernel/engine/src/lib.rs",
    "kernel/engine/src/persistence.rs",
    "scripts/persistence_policy_evidence.py",
    "tests/test_persistence_policy_evidence.py",
)
COMMAND_SPECS: Final = (
    (("cargo", "test", "-p", "agentmage-kernel-engine", "persistence::tests", "--locked"), "9 passed; 0 failed"),
    (("cargo", "test", "-p", "agentmage-kernel-engine", "--locked"), "120 passed; 0 failed"),
    (("python3", "-m", "unittest", "tests.test_persistence_policy_evidence"), "Ran 4 tests"),
    (("npm", "run", "product:lint"), "Structural effect mediation boundary validated."),
)
PROFILE: Final = {
    "field_handling": ["persist", "digest_only", "ephemeral"],
    "secret_classes": ["credential_field", "private_key", "bearer_credential", "provider_token", "cloud_access_key", "embedded_uri_credential"],
    "admitted_encryption": "sql_cipher_operational_store",
    "ephemeral_encryption": "none",
    "restricted_persistence": "denied-until-policy",
    "maximum_fields": 64,
    "maximum_field_bytes": 262144,
    "maximum_total_bytes": 1048576,
}
CLAIMS: Final = {
    "classification_precedes_prepared_record": True,
    "persisted_secret_denied": True,
    "omitted_secret_value_and_digest_absent": True,
    "retention_policy_bound": True,
    "content_free_receipt_generated": True,
    "typed_domain_store_writes_implemented": False,
    "complete_secret_discovery_claimed": False,
    "restricted_persistence_implemented": False,
    "external_network_used": False,
    "release_support": False,
}
LIMITATIONS: Final = [
    "The deterministic scanner covers the six declared signature classes and does not claim exhaustive secret discovery.",
    "PreparedPersistence is a sealed policy output, but typed domain-table write APIs and their storage integration remain later work.",
    "Raw attachment, tool-output, environment, prompt, and model-response lifecycle coverage continues in Sub-task 11.1.1.5.",
    "Expiration execution, holds, erasure, export, restore, corruption recovery, cross-platform evidence, and release acceptance remain later work.",
]
FRAGMENTS: Final = (
    "pub struct PreparedPersistence {",
    "Some(PersistenceOutcome::DeniedRestricted)",
    "Some(PersistenceOutcome::DeniedSecret)",
    "let encryption = if prepared.is_some() {\n            PersistenceEncryption::SqlCipherOperationalStore",
    "fn persisted_secret_denies_without_value_or_value_digest_in_receipt()",
)
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class EvidenceError(ValueError):
    """Raised when persistence evidence is incomplete or overstated."""


def validate_source(value: str) -> list[str]:
    return [f"persistence source fragment changed: {index}" for index, fragment in enumerate(FRAGMENTS, 1) if value.count(fragment) != 1]


def git_revision(candidate: str) -> str:
    result = subprocess.run(["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"], cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, timeout=30, check=False)
    revision = result.stdout.strip()
    if result.returncode or REVISION.fullmatch(revision) is None:
        raise EvidenceError("source revision is unavailable")
    return revision


def git_bytes(revision: str, path: str) -> bytes:
    result = subprocess.run(["git", "show", f"{revision}:{path}"], cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=60, check=False)
    if result.returncode or not result.stdout:
        raise EvidenceError("committed source is unavailable")
    return result.stdout


def command_record(arguments: tuple[str, ...], marker: str) -> dict[str, Any]:
    return {"command_id": hashlib.sha256("\0".join(arguments).encode()).hexdigest(), "exit_code": 0, "expected_marker_sha256": hashlib.sha256(marker.encode()).hexdigest(), "status": "pass"}


def expected_commands() -> list[dict[str, Any]]:
    return [command_record(arguments, marker) for arguments, marker in COMMAND_SPECS]


def run_checked(arguments: tuple[str, ...], marker: str) -> dict[str, Any]:
    result = subprocess.run(list(arguments), cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, timeout=300, check=False, env={**os.environ, "LANG": "C", "LC_ALL": "C"})
    if result.returncode or marker not in result.stdout + result.stderr:
        raise EvidenceError(f"verification command failed: {Path(arguments[0]).name}")
    return command_record(arguments, marker)


def source_records(revision: str) -> list[dict[str, Any]]:
    records = []
    for path in SOURCE_PATHS:
        data = git_bytes(revision, path)
        if path.endswith("persistence.rs") and (errors := validate_source(data.decode())):
            raise EvidenceError("; ".join(errors))
        records.append({"path": path, "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()})
    return records


def build_report(revision: str) -> dict[str, Any]:
    return {"schema_version": 1, "artifact_id": "pre-persistence-policy", "source_revision": revision, "task_ids": ["11.1.1.4"], "status": "pass-classification-minimization-encryption-retention-receipt", "profile": PROFILE, "verification_commands": [run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS], "claims": CLAIMS, "limitations": LIMITATIONS, "private_user_data_used": False, "external_network_used": False, "sources": source_records(revision)}


def validate_report(report: dict[str, Any]) -> list[str]:
    failures = []
    exact = {"schema_version": 1, "artifact_id": "pre-persistence-policy", "task_ids": ["11.1.1.4"], "status": "pass-classification-minimization-encryption-retention-receipt", "profile": PROFILE, "verification_commands": expected_commands(), "claims": CLAIMS, "limitations": LIMITATIONS, "private_user_data_used": False, "external_network_used": False}
    for key, value in exact.items():
        if report.get(key) != value:
            failures.append(f"persistence evidence {key} changed")
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        failures.append("persistence evidence revision is invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(SOURCE_PATHS):
        failures.append("persistence evidence sources changed")
    elif any(not isinstance(item.get("bytes"), int) or item["bytes"] <= 0 or SHA256.fullmatch(str(item.get("sha256", ""))) is None for item in sources):
        failures.append("persistence evidence source records are invalid")
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
    if arguments.write:
        report = build_report(git_revision(arguments.source_revision))
        if errors := validate_report(report):
            raise EvidenceError("; ".join(errors))
        write_atomic(report)
    report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    if errors := validate_report(report):
        raise EvidenceError("; ".join(errors))
    validate_committed(report)
    print("Pre-persistence policy evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
