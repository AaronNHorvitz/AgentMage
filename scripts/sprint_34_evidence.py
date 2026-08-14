#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 34 evidence."""

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
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-34/local-evidence-report.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
SOURCE_PATHS: Final = (
    "capabilities/knowledge/src/tasks.rs",
    "capabilities/knowledge/src/skills.rs",
    "capabilities/knowledge/src/workflows.rs",
    "capabilities/knowledge/src/schema.rs",
    "capabilities/knowledge/src/lifecycle.rs",
    "capabilities/knowledge/README.md",
    "docs/architecture/knowledge-tasks-and-declarative-skills.md",
    "docs/guides/knowledge.md",
    "docs/guides/vault.md",
    "docs/guides/memory.md",
    "docs/guides/conversations.md",
    "docs/guides/retrieval.md",
    "docs/guides/privacy.md",
    "docs/guides/tasks.md",
    "docs/guides/skills.md",
    "docs/release/v0.2-capability-matrix.md",
    "docs/release/v0.2-acceptance-and-migration-bundle.md",
    "docs/release/release-notes-v0.2.0-draft.md",
    "docs/verification/sprint-34-local-results.md",
    "scripts/sprint_34_evidence.py",
    "tests/test_sprint_34_evidence.py",
)
COMMANDS: Final = (
    (
        "knowledge-tests",
        ("cargo", "test", "-p", "agentmage-capability-knowledge", "--locked"),
    ),
    (
        "knowledge-clippy",
        (
            "cargo", "clippy", "-p", "agentmage-capability-knowledge", "--all-targets",
            "--locked", "--", "-D", "warnings",
        ),
    ),
    ("kernel-tests", ("cargo", "test", "-p", "agentmage-kernel-engine", "--locked")),
    ("contract-tests", ("cargo", "test", "-p", "agentmage-kernel-contracts", "--locked")),
    ("product-gate", ("npm", "run", "product:check")),
    ("documentation-gate", ("npm", "run", "docs:check")),
    ("module-inventory", ("python3", "scripts/module_inventory.py")),
    ("dependency-rules", ("python3", "scripts/dependency_rules.py")),
    ("build-contract", ("python3", "scripts/build_contract.py")),
    ("strict-local-source", ("python3", "scripts/strict_local_source_audit.py")),
    ("supply-chain", ("python3", "scripts/supply_chain.py")),
    ("evidence-tests", ("python3", "-m", "unittest", "tests.test_sprint_34_evidence")),
)
SECURITY_REQUIREMENTS: Final = [
    "SR-ACC-001", "SR-ACC-003", "SR-ACC-005", "SR-DAT-001", "SR-DAT-003",
    "SR-DAT-007", "SR-DAT-010", "SR-AI-001", "SR-AI-004", "SR-AI-007",
    "SR-AI-010", "SR-OPS-003", "SR-OPS-006", "SR-TST-001", "SR-TST-005",
    "SR-TST-009", "SR-CIV-003", "SR-CIV-004", "SR-CIV-005",
]
BLOCKERS: Final = [
    {"code": "UPSTREAM-SPRINT-33-BLOCKED", "owner": "33.1"},
    {"code": "SPRINT-34-NATIVE-CHAT-CLI-INTEGRATION-ABSENT", "owner": "34.1.3.3"},
    {"code": "SPRINT-34-PLATFORM-MIGRATION-ROLLBACK-ABSENT", "owner": "34.1.3.4"},
    {"code": "INDEPENDENT-SIGNED-V0.2-REVIEW-ABSENT", "owner": "34.1.3.5"},
]
IMPLEMENTED: Final = {
    "canonical_task_projection_and_views": True,
    "evidence_required_unapplied_transition_preview": True,
    "hash_bound_declarative_skill_registry": True,
    "data_only_skill_assets": True,
    "authority_free_skill_contract": True,
    "visible_precedence_conflicts_and_influence_receipts": True,
    "eight_builtin_read_only_workflows": True,
    "lexical_and_approved_semantic_evidence_parity": True,
    "plain_and_obsidian_canonical_parity": True,
    "v0_2_guides_and_draft_release_bundle": True,
    "task_write_apply": False,
    "native_chat_product_integration": False,
    "cli_product_integration": False,
    "supported_platform_migration_and_rollback": False,
    "signed_v0_2_release": False,
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
        raise ValueError(f"committed Sprint 34 source is absent: {path}")
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
        "record_type": "sprint_34_local_evidence",
        "source_revision": revision,
        "source_sha256": {path: digest(git_file(revision, path)) for path in SOURCE_PATHS},
        "environment": environment_manifest(),
        "commands": commands,
        "security_requirement_ids": SECURITY_REQUIREMENTS,
        "implemented_contracts": IMPLEMENTED,
        "verification_evidence": {
            "task_and_declarative_skill_suite": local_pass,
            "read_only_workflow_parity_suite": local_pass,
            "complete_local_product_and_docs_gates": local_pass,
            "native_chat_and_cli_end_to_end": False,
            "supported_platform_migration_and_rollback": False,
            "upstream_sprint_33_gate": False,
            "independent_signed_release_review": False,
        },
        "blockers": BLOCKERS,
        "summary": {
            "local_knowledge_task_skill_contract_passed": local_pass,
            "sprint_status": "BLOCKED",
            "native_interface_evidence_passed": False,
            "platform_migration_evidence_passed": False,
            "upstream_dependency_passed": False,
            "independent_signed_review_passed": False,
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
        "local_knowledge_task_skill_contract_passed": True,
        "sprint_status": "BLOCKED",
        "native_interface_evidence_passed": False,
        "platform_migration_evidence_passed": False,
        "upstream_dependency_passed": False,
        "independent_signed_review_passed": False,
        "network_access_enabled": False,
        "release_approval": False,
    }
    if report.get("summary") != expected_summary:
        failures.append("summary overclaim or local failure")
    verification = report.get("verification_evidence", {})
    for field in (
        "native_chat_and_cli_end_to_end", "supported_platform_migration_and_rollback",
        "upstream_sprint_33_gate", "independent_signed_release_review",
    ):
        if verification.get(field) is not False:
            failures.append(f"verification overclaim: {field}")
    for field in (
        "task_write_apply", "native_chat_product_integration", "cli_product_integration",
        "supported_platform_migration_and_rollback", "signed_v0_2_release",
        "network_access", "external_delivery",
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
