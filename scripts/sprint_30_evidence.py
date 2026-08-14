#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 30 evidence."""

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
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-30/local-evidence-report.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
SOURCE_PATHS: Final = (
    "capabilities/knowledge/src/lib.rs",
    "capabilities/knowledge/src/semantic.rs",
    "capabilities/knowledge/src/semantic_benchmark.rs",
    "docs/architecture/optional-local-semantic-retrieval.md",
    "docs/verification/sprint-30-local-results.md",
    "scripts/sprint_30_evidence.py",
    "tests/test_sprint_30_evidence.py",
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
    ("evidence-tests", ("python3", "-m", "unittest", "tests.test_sprint_30_evidence")),
)
SECURITY_REQUIREMENTS: Final = [
    "SR-DAT-001", "SR-DAT-002", "SR-DAT-003", "SR-DAT-004", "SR-SUP-007",
    "SR-SUP-008", "SR-AI-006", "SR-AI-007", "SR-AI-008", "SR-AI-009",
    "SR-AI-010", "SR-AI-011", "SR-AI-012", "SR-AI-013", "SR-TST-006",
]
BLOCKERS: Final = [
    {"code": "UPSTREAM-SPRINT-29-BLOCKED", "owner": "29.1"},
    {"code": "REAL-APPROVED-SEMANTIC-PROFILES-ABSENT", "owner": "30.1.1.1"},
    {"code": "REAL-HARDWARE-COMPARATIVE-BENCHMARK-ABSENT", "owner": "30.1.1.8"},
    {"code": "APPLICATION-SEMANTIC-INTEGRATION-ABSENT", "owner": "30.1.3.4"},
    {"code": "INDEPENDENT-SPRINT-30-REVIEW-ABSENT", "owner": "30.1.3.5"},
]
IMPLEMENTED: Final = {
    "exact_embedding_and_optional_reranker_admission_contract": True,
    "per_workspace_file_and_field_opt_in": True,
    "local_only_storage_and_lifecycle_disclosure": True,
    "content_addressed_source_ranged_fixed_point_index": True,
    "atomic_rebuild_stale_omission_and_configuration_invalidation": True,
    "inspect_delete_clear_and_complete_rebuild": True,
    "deterministic_lexical_fallback": True,
    "four_mode_integer_comparison_gate": True,
    "synthetic_contract_fixture_only": True,
    "real_approved_embedding_profile": False,
    "real_approved_reranker_profile": False,
    "real_runtime_generated_vectors": False,
    "real_hardware_benefit_benchmark": False,
    "remote_semantic_capability": False,
    "semantic_release_enabled": False,
    "source_write_authority": False,
}


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def git_file(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, check=False,
        capture_output=True, timeout=30,
    )
    if result.returncode != 0:
        raise ValueError(f"committed Sprint 30 source is absent: {path}")
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
        "record_type": "sprint_30_local_evidence",
        "source_revision": revision,
        "source_sha256": {path: digest(git_file(revision, path)) for path in SOURCE_PATHS},
        "environment": environment_manifest(),
        "commands": commands,
        "security_requirement_ids": SECURITY_REQUIREMENTS,
        "implemented_contracts": IMPLEMENTED,
        "verification_evidence": {
            "synthetic_manifest_scope_lifecycle_and_adversarial_suite": local_pass,
            "synthetic_four_mode_comparison_gate": local_pass,
            "complete_local_product_and_docs_gates": local_pass,
            "upstream_sprint_29_gate": False,
            "real_approved_semantic_profiles": False,
            "real_hardware_comparative_benchmark": False,
            "application_semantic_integration": False,
            "independent_review": False,
        },
        "blockers": BLOCKERS,
        "summary": {
            "local_semantic_contract_passed": local_pass,
            "sprint_status": "BLOCKED",
            "semantic_release_enabled": False,
            "remote_semantic_capability": False,
            "source_write_authority": False,
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
        "local_semantic_contract_passed": True,
        "sprint_status": "BLOCKED",
        "semantic_release_enabled": False,
        "remote_semantic_capability": False,
        "source_write_authority": False,
        "release_approval": False,
    }:
        failures.append("summary overclaim or local failure")
    verification = report.get("verification_evidence", {})
    for field in (
        "upstream_sprint_29_gate", "real_approved_semantic_profiles",
        "real_hardware_comparative_benchmark", "application_semantic_integration",
        "independent_review",
    ):
        if verification.get(field) is not False:
            failures.append(f"verification overclaim: {field}")
    for field in (
        "real_approved_embedding_profile", "real_approved_reranker_profile",
        "real_runtime_generated_vectors", "real_hardware_benefit_benchmark",
        "remote_semantic_capability", "semantic_release_enabled", "source_write_authority",
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
