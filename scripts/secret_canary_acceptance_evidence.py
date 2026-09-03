#!/usr/bin/env python3
"""Build and validate S-011-ST01 secret-canary acceptance evidence."""

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
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-11/story-11.1/s-011-st01.json"
SOURCE_PATHS: Final = (
    "docs/verification/s-011-st01-secret-canary-results.md",
    "kernel/engine/src/persistence.rs",
    "kernel/engine/src/operational_store.rs",
    "security/strict-local-source-policy.json",
    "scripts/secret_canary_acceptance_evidence.py",
    "tests/test_secret_canary_acceptance_evidence.py",
)
SURFACE_IDS: Final = tuple(f"SC-{index:02}" for index in range(1, 13))
DOCUMENT_FRAGMENTS: Final = (
    "Pass for every currently representable persistence input and reachable surface",
    "Synthetic record families exercised: **11 of 11**.",
    "Reachable output surfaces closed: **12 of 12**.",
    "Logs and model context are closed by structural absence",
    "Typed production writers for all 11 normalized tables do not yet exist",
    "manually deferred fuzzing remain separate gates.",
)
PERSISTENCE_FRAGMENTS: Final = (
    "pub enum PersistenceRecordFamily {",
    "pub enum PersistenceFieldHandling {",
    "pub enum EphemeralContentClass {",
    "pub enum SecretFindingClass {",
    "fn synthetic_canary_campaign_covers_every_family_and_field_boundary()",
    "assert_eq!(record.matches(canary).count(), 2);",
    "assert_eq!(denied.receipt().outcome(), PersistenceOutcome::DeniedSecret);",
)
STORE_FRAGMENTS: Final = (
    "fn synthetic_canary_is_absent_from_encrypted_and_derived_artifacts()",
    "PRAGMA cipher_log_level = NONE;",
    "PRAGMA temp_store = MEMORY;",
    "assert_artifacts_exclude_canary(&path, &backup, &export, canary);\n        drop(store);",
)
AUDIT_FRAGMENTS: Final = (
    '"kernel/engine/src",',
    '"platforms/linux-inference/src",',
    '"shells/vscode/src"',
    '"@sentry/node",',
    '"posthog-node",',
)
COMMAND_SPECS: Final = (
    (("cargo", "test", "-p", "agentmage-kernel-engine", "persistence", "--locked"), "20 passed; 0 failed"),
    (("cargo", "test", "-p", "agentmage-kernel-engine", "synthetic_canary_is_absent_from_encrypted_and_derived_artifacts", "--locked"), "1 passed; 0 failed"),
    (("python3", "scripts/strict_local_source_audit.py"), "zero undeclared network paths"),
    (("python3", "-m", "unittest", "tests.test_secret_canary_acceptance_evidence"), "Ran 4 tests"),
    (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
)
CLAIMS: Final = {
    "record_families_exercised": 11,
    "field_handling_modes_exercised": 3,
    "ephemeral_content_classes_exercised": 5,
    "secret_finding_classes_exercised": 6,
    "reachable_surfaces_closed": 12,
    "raw_canary_values_retained_in_evidence": 0,
    "plaintext_fallback_paths_observed": 0,
    "unauthorized_model_context_paths_observed": 0,
    "logs_closed_by_structural_absence": True,
    "model_context_closed_by_structural_absence": True,
    "typed_domain_writers_complete": False,
    "active_model_workflow_tested": False,
    "physical_remanence_claim": False,
    "manual_fuzzing_executed": False,
    "external_network_used": False,
    "release_support": False,
}
LIMITATIONS: Final = [
    "The public pre-persistence contract is fully matrix-tested, but typed production writers for all 11 normalized tables do not yet exist; encrypted artifact scanning directly seeds the current sessions table.",
    "Logs and model context are closed by structural absence because no persistence logger or active model bridge exists; future activation invalidates this result until the matrix is extended.",
    "Byte scanning is not forensic proof of SSD, filesystem snapshot, swap, or external host instrumentation remanence.",
    "The at-least-100-seed crash campaign, live Secret Service, cross-platform, packaging, release acceptance, and manually deferred fuzzing remain separate gates.",
]
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class EvidenceError(ValueError):
    pass


def validate_sources(document: str, persistence: str, store: str, audit: str) -> list[str]:
    failures = []
    if tuple(re.findall(r"`(SC-\d{2})`", document)) != SURFACE_IDS:
        failures.append("S-011-ST01 surface closure changed")
    for label, value, fragments in (
        ("document", document, DOCUMENT_FRAGMENTS),
        ("persistence", persistence, PERSISTENCE_FRAGMENTS),
        ("store", store, STORE_FRAGMENTS),
        ("audit", audit, AUDIT_FRAGMENTS),
    ):
        failures.extend(
            f"S-011-ST01 {label} fragment changed: {index}"
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
    failures = validate_sources(values[SOURCE_PATHS[0]], values[SOURCE_PATHS[1]], values[SOURCE_PATHS[2]], values[SOURCE_PATHS[3]])
    if failures:
        raise EvidenceError("; ".join(failures))
    return records


def build_report(revision: str) -> dict[str, Any]:
    return {
        "artifact_id": "s-011-st01-secret-canary-results",
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "source_revision": revision,
        "sources": source_records(revision),
        "status": "pass-current-representable-input-and-surface-closure",
        "surface_ids": list(SURFACE_IDS),
        "task_ids": ["11.1.3.2", "S-011-ST01"],
        "verification_commands": [run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS],
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    exact = {
        "artifact_id": "s-011-st01-secret-canary-results",
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "status": "pass-current-representable-input-and-surface-closure",
        "surface_ids": list(SURFACE_IDS),
        "task_ids": ["11.1.3.2", "S-011-ST01"],
        "verification_commands": expected_commands(),
    }
    failures = [f"S-011-ST01 evidence {key} changed" for key, value in exact.items() if report.get(key) != value]
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        failures.append("S-011-ST01 evidence revision invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(SOURCE_PATHS):
        failures.append("S-011-ST01 evidence sources changed")
    elif any(not isinstance(item.get("bytes"), int) or item["bytes"] <= 0 or SHA256.fullmatch(str(item.get("sha256", ""))) is None for item in sources):
        failures.append("S-011-ST01 evidence source records invalid")
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
    print("S-011-ST01 evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
