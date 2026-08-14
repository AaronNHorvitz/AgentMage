#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 28 evidence."""

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
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-28/local-evidence-report.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
SOURCE_PATHS: Final = (
    "capabilities/knowledge/Cargo.toml",
    "capabilities/knowledge/README.md",
    "capabilities/knowledge/src/domain.rs",
    "capabilities/knowledge/src/lib.rs",
    "capabilities/knowledge/src/obsidian.rs",
    "capabilities/knowledge/src/obsidian_index.rs",
    "docs/architecture/obsidian-index-boundary.md",
    "docs/verification/sprint-28-local-results.md",
    "scripts/sprint_28_evidence.py",
    "tests/test_sprint_28_evidence.py",
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
    ("evidence-tests", ("python3", "-m", "unittest", "tests.test_sprint_28_evidence")),
)
SECURITY_REQUIREMENTS: Final = [
    "SR-ACC-004", "SR-ACC-005", "SR-ACC-006", "SR-ACC-007", "SR-ACC-008",
    "SR-AI-005", "SR-DAT-002", "SR-TST-002", "SR-TST-004",
]
BLOCKERS: Final = [
    {"code": "UPSTREAM-SPRINT-27-BLOCKED", "owner": "27.1"},
    {"code": "OPERATING-SYSTEM-WATCHER-ADAPTER-ABSENT", "owner": "28.1.1.4"},
    {"code": "INDEPENDENT-SPRINT-28-REVIEW-ABSENT", "owner": "28.1.3.5"},
]
IMPLEMENTED: Final = {
    "source_ranged_obsidian_parser_generation_two": True,
    "verified_attachment_embed_and_coverage_model": True,
    "disposable_seven_table_sqlite_projection": True,
    "atomic_rebuild_and_transaction_rollback": True,
    "current_historical_query_and_cycle_safe_traversal": True,
    "stale_detection_and_visible_conflicts": True,
    "raw_section_preserving_change_preview": True,
    "exact_watcher_event_reconciliation": True,
    "content_free_access_receipts": True,
    "plain_folder_and_obsidian_store_conformance": True,
    "secret_candidates_omitted_from_searchable_projection": True,
    "operating_system_filesystem_watcher": False,
    "obsidian_process_or_network_access": False,
    "vault_write_authority": False,
}


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def git_file(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, check=False,
        capture_output=True, timeout=30,
    )
    if result.returncode != 0:
        raise ValueError(f"committed Sprint 28 source is absent: {path}")
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
        "record_type": "sprint_28_local_evidence",
        "source_revision": revision,
        "source_sha256": {path: digest(git_file(revision, path)) for path in SOURCE_PATHS},
        "environment": environment_manifest(),
        "commands": commands,
        "security_requirement_ids": SECURITY_REQUIREMENTS,
        "implemented_contracts": IMPLEMENTED,
        "verification_evidence": {
            "synthetic_parser_and_link_corpus": local_pass,
            "index_transaction_and_interruption_traces": local_pass,
            "stale_update_and_source_invariance": local_pass,
            "injection_secret_uri_and_no_obsidian_proof": local_pass,
            "adapter_contract_conformance": local_pass,
            "complete_local_product_and_docs_gates": local_pass,
            "upstream_sprint_27_gate": False,
            "operating_system_watcher_adapter": False,
            "independent_review": False,
        },
        "blockers": BLOCKERS,
        "summary": {
            "local_index_contract_passed": local_pass,
            "sprint_status": "BLOCKED",
            "vault_write_enabled": False,
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
        "local_index_contract_passed": True,
        "sprint_status": "BLOCKED",
        "vault_write_enabled": False,
        "release_approval": False,
    }:
        failures.append("summary overclaim or local failure")
    verification = report.get("verification_evidence", {})
    for field in (
        "upstream_sprint_27_gate", "operating_system_watcher_adapter", "independent_review",
    ):
        if verification.get(field) is not False:
            failures.append(f"verification overclaim: {field}")
    for field in (
        "operating_system_filesystem_watcher", "obsidian_process_or_network_access",
        "vault_write_authority",
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
