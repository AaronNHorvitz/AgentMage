#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 35 evidence."""

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
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-35/local-evidence-report.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
SOURCE_PATHS: Final = (
    "kernel/engine/src/write_approval.rs",
    "kernel/engine/src/grants.rs",
    "kernel/engine/src/approval.rs",
    "kernel/engine/src/lib.rs",
    "docs/architecture/exact-preimage-write-approval.md",
    "docs/verification/sprint-35-local-results.md",
    "scripts/sprint_35_evidence.py",
    "tests/test_sprint_35_evidence.py",
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
    ("effect-boundary", ("python3", "scripts/effect_boundary.py")),
    ("build-contract", ("python3", "scripts/build_contract.py")),
    ("strict-local-source", ("python3", "scripts/strict_local_source_audit.py")),
    ("supply-chain", ("python3", "scripts/supply_chain.py")),
    ("evidence-tests", ("python3", "-m", "unittest", "tests.test_sprint_35_evidence")),
)
SECURITY_REQUIREMENTS: Final = [
    "SR-ACC-001", "SR-ACC-002", "SR-ACC-003", "SR-ACC-004", "SR-ACC-005",
    "SR-ACC-006", "SR-ACC-007", "SR-DAT-002", "SR-OPS-001", "SR-OPS-002",
    "SR-TST-005", "SR-TST-011", "SR-TST-012",
]
BLOCKERS: Final = [
    {"code": "UPSTREAM-SPRINT-34-BLOCKED", "owner": "34.1"},
    {"code": "INDEPENDENT-SPRINT-35-TRANSACTION-REVIEW-ABSENT", "owner": "35.1.3.4"},
]
IMPLEMENTED: Final = {
    "current_parent_read_grant_observation": True,
    "exact_held_target_preimage_binding": True,
    "authority_free_in_memory_shadow_change_set": True,
    "deterministic_shadow_validation": True,
    "complete_exact_review_preview": True,
    "short_lived_single_use_workspace_write_grant": True,
    "unchanged_parent_revision_requirement": True,
    "fresh_preapply_revalidation_and_stale_invalidation": True,
    "target_file_mutation": False,
    "atomic_application": False,
    "rollback": False,
    "post_write_command_execution": False,
    "generic_shell": False,
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
        raise ValueError(f"committed Sprint 35 source is absent: {path}")
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
        "record_type": "sprint_35_local_evidence",
        "source_revision": revision,
        "source_sha256": {path: digest(git_file(revision, path)) for path in SOURCE_PATHS},
        "environment": environment_manifest(),
        "commands": commands,
        "security_requirement_ids": SECURITY_REQUIREMENTS,
        "implemented_contracts": IMPLEMENTED,
        "verification_evidence": {
            "exact_preimage_shadow_and_preview_suite": local_pass,
            "grant_and_stale_invalidation_suite": local_pass,
            "complete_local_product_and_docs_gates": local_pass,
            "upstream_sprint_34_gate": False,
            "independent_transaction_review": False,
        },
        "blockers": BLOCKERS,
        "summary": {
            "local_exact_preimage_approval_contract_passed": local_pass,
            "sprint_status": "BLOCKED",
            "upstream_dependency_passed": False,
            "independent_review_passed": False,
            "target_write_enabled": False,
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
    expected_summary = {
        "local_exact_preimage_approval_contract_passed": True,
        "sprint_status": "BLOCKED",
        "upstream_dependency_passed": False,
        "independent_review_passed": False,
        "target_write_enabled": False,
        "network_access_enabled": False,
        "release_approval": False,
    }
    if report.get("summary") != expected_summary:
        failures.append("summary overclaim or local failure")
    verification = report.get("verification_evidence", {})
    for field in ("upstream_sprint_34_gate", "independent_transaction_review"):
        if verification.get(field) is not False:
            failures.append(f"verification overclaim: {field}")
    for field in (
        "target_file_mutation", "atomic_application", "rollback",
        "post_write_command_execution", "generic_shell", "network_access", "external_delivery",
    ):
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
