#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 26 evidence."""

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
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-26/local-evidence-report.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
SOURCE_PATHS: Final = (
    "capabilities/knowledge/Cargo.toml",
    "capabilities/knowledge/README.md",
    "capabilities/knowledge/src/authority.rs",
    "capabilities/knowledge/src/domain.rs",
    "capabilities/knowledge/src/index.rs",
    "capabilities/knowledge/src/lib.rs",
    "capabilities/knowledge/src/lifecycle.rs",
    "capabilities/knowledge/src/operations.rs",
    "capabilities/knowledge/src/plain_folder.rs",
    "capabilities/knowledge/src/schema.rs",
    "capabilities/knowledge/src/store.rs",
    "docs/architecture/knowledge-authority-boundary.md",
    "docs/verification/sprint-26-local-results.md",
    "scripts/sprint_26_evidence.py",
    "tests/test_sprint_26_evidence.py",
)
COMMANDS: Final = (
    ("knowledge-tests", ("cargo", "test", "-p", "agentmage-capability-knowledge", "--locked")),
    (
        "knowledge-clippy",
        (
            "cargo", "clippy", "-p", "agentmage-capability-knowledge", "--all-targets",
            "--locked", "--", "-D", "warnings",
        ),
    ),
    ("product-gate", ("npm", "run", "product:check")),
    ("documentation-gate", ("npm", "run", "docs:check")),
    ("module-inventory", ("python3", "scripts/module_inventory.py")),
    ("dependency-rules", ("python3", "scripts/dependency_rules.py")),
    ("build-contract", ("python3", "scripts/build_contract.py")),
    ("strict-local-source", ("python3", "scripts/strict_local_source_audit.py")),
    ("supply-chain", ("python3", "scripts/supply_chain.py")),
    ("evidence-tests", ("python3", "-m", "unittest", "tests.test_sprint_26_evidence")),
)
SECURITY_REQUIREMENTS: Final = [
    "SR-GOV-006", "SR-DAT-001", "SR-DAT-002", "SR-DAT-003", "SR-DAT-004",
    "SR-DAT-010", "SR-DAT-011", "SR-DAT-012", "SR-CIV-001", "SR-CIV-002",
    "SR-CIV-003", "SR-CIV-004", "SR-CIV-005",
]
BLOCKERS: Final = [
    {"code": "UPSTREAM-G-V0.1-BLOCKED", "owner": "25.1"},
    {"code": "PRIVACY-DECISION-PLACEHOLDER", "owner": "26.1.3.5"},
    {"code": "RECORDS-DECISION-PLACEHOLDER", "owner": "26.1.3.5"},
    {"code": "INDEPENDENT-SPRINT-26-REVIEW-ABSENT", "owner": "26.1.3.5"},
    {"code": "SUPPORTED-PACKAGE-INTEGRATION-ABSENT", "owner": "26.1.2.2"},
]
IMPLEMENTED: Final = {
    "closed_fourteen_record_schema_registry": True,
    "field_level_owner_and_lifecycle_dictionary": True,
    "stable_path_independent_identity": True,
    "plain_folder_snapshot_adapter": True,
    "configurable_layout_defaults": True,
    "preview_only_create_and_compare_swap_update": True,
    "relationship_duplicate_import_dashboard_and_export": True,
    "disposable_sqlite_index": True,
    "backup_restore_and_migration_preview": True,
    "secret_candidate_rejection": True,
    "canonical_markdown_write_authority": False,
    "operational_store_dependency": False,
    "obsidian_specific_behavior": False,
    "supported_package_integration": False,
}


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def git_file(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, check=False,
        capture_output=True, timeout=30,
    )
    if result.returncode != 0:
        raise ValueError(f"committed Sprint 26 source is absent: {path}")
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
        "record_type": "sprint_26_local_evidence",
        "source_revision": revision,
        "source_sha256": {path: digest(git_file(revision, path)) for path in SOURCE_PATHS},
        "environment": environment_manifest(),
        "commands": commands,
        "security_requirement_ids": SECURITY_REQUIREMENTS,
        "implemented_contracts": IMPLEMENTED,
        "verification_evidence": {
            "canonical_schema_and_data_dictionary": local_pass,
            "plain_folder_and_stable_identity": local_pass,
            "derived_index_export_and_dashboard": local_pass,
            "backup_restore_and_migration_dry_run": local_pass,
            "adversarial_secret_path_link_and_content_matrix": local_pass,
            "operational_knowledge_separation": local_pass,
            "complete_local_product_and_docs_gates": local_pass,
            "upstream_release_gate": False,
            "privacy_and_records_decisions": False,
            "independent_review": False,
            "supported_package_integration": False,
        },
        "blockers": BLOCKERS,
        "summary": {
            "local_contract_passed": local_pass,
            "sprint_status": "BLOCKED",
            "canonical_write_enabled": False,
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
        "local_contract_passed": True,
        "sprint_status": "BLOCKED",
        "canonical_write_enabled": False,
        "release_approval": False,
    }:
        failures.append("summary overclaim or local failure")
    verification = report.get("verification_evidence", {})
    for field in (
        "upstream_release_gate", "privacy_and_records_decisions", "independent_review",
        "supported_package_integration",
    ):
        if verification.get(field) is not False:
            failures.append(f"verification overclaim: {field}")
    for field in (
        "canonical_markdown_write_authority", "operational_store_dependency",
        "obsidian_specific_behavior", "supported_package_integration",
    ):
        if report.get("implemented_contracts", {}).get(field) is not False:
            failures.append(f"implementation overclaim: {field}")
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
