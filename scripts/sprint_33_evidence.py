#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 33 evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import platform
import re
import shutil
import subprocess
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-33/local-evidence-report.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
SOURCE_PATHS: Final = (
    "kernel/engine/src/conversation_archive.rs",
    "kernel/engine/src/conversation_library.rs",
    "kernel/engine/src/evidence_bundle.rs",
    "kernel/engine/src/operational_store.rs",
    "kernel/engine/src/persistence.rs",
    "docs/architecture/private-archives-evidence-bundles.md",
    "docs/verification/sprint-33-local-results.md",
    "scripts/sprint_33_evidence.py",
    "tests/test_sprint_33_evidence.py",
)
COMMANDS: Final = (
    ("kernel-tests", ("cargo", "test", "-p", "agentmage-kernel-engine", "--locked")),
    (
        "kernel-clippy",
        (
            "cargo", "clippy", "-p", "agentmage-kernel-engine", "--all-targets",
            "--locked", "--", "-D", "warnings",
        ),
    ),
    ("contract-tests", ("cargo", "test", "-p", "agentmage-kernel-contracts", "--locked")),
    ("product-gate", ("npm", "run", "product:check")),
    ("documentation-gate", ("npm", "run", "docs:check")),
    ("module-inventory", ("python3", "scripts/module_inventory.py")),
    ("dependency-rules", ("python3", "scripts/dependency_rules.py")),
    ("build-contract", ("python3", "scripts/build_contract.py")),
    ("strict-local-source", ("python3", "scripts/strict_local_source_audit.py")),
    ("supply-chain", ("python3", "scripts/supply_chain.py")),
    ("evidence-tests", ("python3", "-m", "unittest", "tests.test_sprint_33_evidence")),
)
SECURITY_REQUIREMENTS: Final = [
    "SR-DAT-001", "SR-DAT-002", "SR-DAT-003", "SR-DAT-004", "SR-DAT-007",
    "SR-DAT-010", "SR-DAT-011", "SR-DAT-012", "SR-CIV-003", "SR-CIV-004",
    "SR-CIV-005", "SR-OPS-003",
]
BLOCKERS: Final = [
    {"code": "UPSTREAM-SPRINT-32-BLOCKED", "owner": "32.1"},
    {"code": "INDEPENDENT-SPRINT-33-REVIEW-ABSENT", "owner": "33.1.3.5"},
    {"code": "SHELL-CONVERSATION-INTEGRATION-ABSENT", "owner": "33.AC5"},
]
IMPLEMENTED: Final = {
    "separately_keyed_sqlcipher_archive": True,
    "exact_reviewed_archive_inventory": True,
    "archive_inspection_and_fresh_restore": True,
    "archive_retention_revision_and_deletion": True,
    "redacted_portable_evidence_bundle": True,
    "exact_byte_disclosure_preview": True,
    "derived_citation_receipt_and_source_sets": True,
    "secret_hidden_unrelated_and_unapproved_exclusion": True,
    "private_atomic_local_publication": True,
    "bounded_evidence_state_and_ancestry_search": True,
    "every_turn_role_branch_corpus": True,
    "disclosure_canary_corpus": True,
    "process_stop_and_concurrent_access_matrix": True,
    "bundle_import_authority": False,
    "network_access": False,
    "external_delivery": False,
}


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def git_file(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, check=False,
        capture_output=True, timeout=30,
    )
    if result.returncode != 0:
        raise ValueError(f"committed Sprint 33 source is absent: {path}")
    return result.stdout


def version(executable: str, *arguments: str) -> str:
    resolved = shutil.which(executable)
    if resolved is None:
        return "unavailable"
    result = subprocess.run(
        (resolved, *arguments), cwd=ROOT, check=False, capture_output=True,
        text=True, timeout=30,
    )
    output = (result.stdout + result.stderr).strip().splitlines()
    return output[0][:256] if result.returncode == 0 and output else "unavailable"


def environment_manifest() -> dict[str, str]:
    return {
        "system": platform.system(), "release": platform.release(),
        "machine": platform.machine(), "python": platform.python_version(),
        "rustc": version("rustc", "--version"), "cargo": version("cargo", "--version"),
        "node": version("node", "--version"), "npm": version("npm", "--version"),
    }


def run_commands() -> list[dict[str, Any]]:
    records = []
    for identifier, argv in COMMANDS:
        executable = shutil.which(argv[0])
        if executable is None:
            code, output = 127, b"executable-unavailable"
        else:
            result = subprocess.run(
                (executable, *argv[1:]), cwd=ROOT, check=False,
                capture_output=True, timeout=1800,
            )
            code, output = result.returncode, result.stdout + result.stderr
        records.append({
            "id": identifier, "argv": list(argv), "exit_code": code,
            "output_sha256": digest(output),
        })
    return records


def build_report(revision: str, commands: list[dict[str, Any]]) -> dict[str, Any]:
    local_pass = all(item["exit_code"] == 0 for item in commands)
    return {
        "schema_version": 1,
        "record_type": "sprint_33_local_evidence",
        "source_revision": revision,
        "source_sha256": {path: digest(git_file(revision, path)) for path in SOURCE_PATHS},
        "environment": environment_manifest(),
        "commands": commands,
        "security_requirement_ids": SECURITY_REQUIREMENTS,
        "implemented_contracts": IMPLEMENTED,
        "verification_evidence": {
            "archive_lifecycle_suite": local_pass,
            "evidence_disclosure_and_redaction_suite": local_pass,
            "complete_local_product_and_docs_gates": local_pass,
            "complete_ut01_search_matrix": local_pass,
            "complete_ut02_every_turn_role_branch_matrix": local_pass,
            "complete_st01_disclosure_canary_matrix": local_pass,
            "complete_rt01_crash_and_simultaneous_access_matrix": local_pass,
            "upstream_sprint_32_gate": False,
            "independent_review": False,
            "shell_integration": False,
        },
        "blockers": BLOCKERS,
        "summary": {
            "local_archive_and_bundle_contract_passed": local_pass,
            "sprint_status": "BLOCKED",
            "rt01_matrix_complete": local_pass,
            "upstream_dependency_passed": False,
            "independent_review_passed": False,
            "network_access_enabled": False,
            "release_approval": False,
        },
    }


def validate_report(report: dict[str, Any], verify_current: bool = True) -> list[str]:
    failures: list[str] = []
    revision = str(report.get("source_revision", ""))
    if not REVISION.fullmatch(revision):
        failures.append("source revision invalid")
    if report.get("security_requirement_ids") != SECURITY_REQUIREMENTS:
        failures.append("security mapping drift")
    if report.get("implemented_contracts") != IMPLEMENTED:
        failures.append("implemented contract drift")
    if report.get("blockers") != BLOCKERS:
        failures.append("blocker drift")
    commands = report.get("commands", [])
    if [item.get("id") for item in commands] != [item[0] for item in COMMANDS]:
        failures.append("command inventory drift")
    if any(
        item.get("exit_code") != 0
        or not SHA256.fullmatch(str(item.get("output_sha256", "")))
        for item in commands
    ):
        failures.append("command result invalid")
    if report.get("summary") != {
        "local_archive_and_bundle_contract_passed": True,
        "sprint_status": "BLOCKED",
        "rt01_matrix_complete": True,
        "upstream_dependency_passed": False,
        "independent_review_passed": False,
        "network_access_enabled": False,
        "release_approval": False,
    }:
        failures.append("summary overclaim or local failure")
    verification = report.get("verification_evidence", {})
    for field in ("upstream_sprint_32_gate", "independent_review", "shell_integration"):
        if verification.get(field) is not False:
            failures.append(f"verification overclaim: {field}")
    for field in (
        "archive_lifecycle_suite",
        "evidence_disclosure_and_redaction_suite",
        "complete_local_product_and_docs_gates",
        "complete_ut01_search_matrix",
        "complete_ut02_every_turn_role_branch_matrix",
        "complete_st01_disclosure_canary_matrix",
        "complete_rt01_crash_and_simultaneous_access_matrix",
    ):
        if verification.get(field) is not True:
            failures.append(f"verification missing: {field}")
    for field in ("bundle_import_authority", "network_access", "external_delivery"):
        if report.get("implemented_contracts", {}).get(field) is not False:
            failures.append(f"capability overclaim: {field}")
    environment = report.get("environment", {})
    expected_environment = {
        "system", "release", "machine", "python", "rustc", "cargo", "node", "npm",
    }
    if set(environment) != expected_environment:
        failures.append("environment manifest drift")
    elif any(
        not isinstance(value, str) or not value or len(value) > 256
        for value in environment.values()
    ):
        failures.append("environment manifest invalid")
    sources = report.get("source_sha256", {})
    if set(sources) != set(SOURCE_PATHS):
        failures.append("source inventory drift")
    elif verify_current and REVISION.fullmatch(revision):
        for path in SOURCE_PATHS:
            if sources[path] != digest(git_file(revision, path)):
                failures.append(f"source digest drift: {path}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    args = parser.parse_args()
    if args.write:
        revision = subprocess.run(
            ["git", "rev-parse", args.source_revision], cwd=ROOT, check=True,
            capture_output=True, text=True,
        ).stdout.strip()
        report = build_report(revision, run_commands())
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8",
        )
    else:
        report = json.loads(OUTPUT.read_text(encoding="utf-8"))
    failures = validate_report(report)
    if failures:
        print("\n".join(failures))
        return 1
    print(json.dumps(report["summary"], sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
