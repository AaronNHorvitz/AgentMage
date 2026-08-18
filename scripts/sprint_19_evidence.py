#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 19 evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import subprocess
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-19/local-evidence-report.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
SOURCE_PATHS: Final = (
    "TASKS.md",
    "capabilities/repository-map/src/cache.rs",
    "capabilities/repository-map/src/invariance_tests.rs",
    "capabilities/repository-map/src/inventory.rs",
    "capabilities/repository-map/src/lib.rs",
    "capabilities/repository-map/src/parser.rs",
    "capabilities/repository-map/src/renderer.rs",
    "capabilities/repository-map/src/resolution.rs",
    "docs/architecture/pinned-repository-map.md",
    "docs/verification/task-19-1-3-5-product-security-evidence.md",
    "docs/verification/sprint-19-local-results.md",
    "scripts/sprint_19_evidence.py",
    "tests/test_sprint_19_evidence.py",
)
COMMANDS: Final = (
    (
        "repository-map-tests",
        (
            "cargo",
            "test",
            "-p",
            "agentmage-capability-repository-map",
            "--all-targets",
            "--locked",
        ),
    ),
    (
        "repository-map-clippy",
        (
            "cargo",
            "clippy",
            "-p",
            "agentmage-capability-repository-map",
            "--all-targets",
            "--locked",
            "--",
            "-D",
            "warnings",
        ),
    ),
    (
        "linux-repository-host-tests",
        (
            "cargo",
            "test",
            "-p",
            "agentmage-host",
            "linux_repository_map",
            "--locked",
        ),
    ),
    (
        "repository-cache-tests",
        (
            "cargo",
            "test",
            "-p",
            "agentmage-kernel-engine",
            "repository_cache",
            "--locked",
        ),
    ),
    ("sprint-18-evidence", ("python3", "scripts/sprint_18_evidence.py")),
    ("effect-boundary", ("python3", "scripts/effect_boundary.py")),
    ("strict-local-source", ("python3", "scripts/strict_local_source_audit.py")),
    ("supply-chain", ("python3", "scripts/supply_chain.py")),
)
GRAMMAR_COMMAND: Final = (
    "cargo",
    "run",
    "-q",
    "-p",
    "agentmage-capability-repository-map",
    "--example",
    "grammar_bom",
    "--locked",
)
SECURITY_REQUIREMENTS: Final = [
    "SR-ACC-008",
    "SR-AI-003",
    "SR-AI-007",
    "SR-AI-010",
    "SR-OPS-001",
    "SR-TST-002",
    "SR-TST-004",
]
BLOCKERS: Final = [
    {"code": "NATIVE-UBUNTU-REPOSITORY-MAP-EVIDENCE-INCOMPLETE", "owner": "19.1.3.4"},
    {"code": "NATIVE-MACOS-REPOSITORY-MAP-EVIDENCE-INCOMPLETE", "owner": "19.1.3.4"},
    {"code": "NATIVE-WINDOWS-REPOSITORY-MAP-EVIDENCE-INCOMPLETE", "owner": "19.1.3.4"},
    {"code": "MANUAL-REPOSITORY-PARSER-FUZZING-DEFERRED", "owner": "19.1.3.5"},
    {"code": "INDEPENDENT-SPRINT-19-REVIEW-NOT-RETAINED", "owner": "19.1.3.5"},
]
IMPLEMENTED_CONTRACTS: Final = {
    "coverage_file_states": 10,
    "coverage_fixed_budgets": 5,
    "render_priority_tiers": 6,
    "cache_key_dimensions": 7,
    "maximum_lexical_matches_per_file": 32,
    "minimum_context_tokens": 512,
    "maximum_context_tokens": 262_144,
    "unknown_blocked_fallback": True,
    "cooperative_parser_cancellation": True,
    "rust_panic_containment": True,
    "partial_parser_results_emitted": False,
    "filesystem_authority": False,
    "process_authority": False,
    "network_authority": False,
    "golden_map_sha256": "296fd3fb3768778f17af5bd6d00879cd922c5c74cbc9a467cc0c290db7bc8106",
    "golden_context_sha256": "e96786f4873c335ae350a98fa6b5d31aaf39d4e4015ed5b5a0f41adbe9cdb0ef",
}


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def git_file(revision: str, relative: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{relative}"],
        cwd=ROOT,
        check=False,
        capture_output=True,
        timeout=30,
    )
    if result.returncode != 0:
        raise ValueError(f"committed Sprint 19 source is absent: {relative}")
    return result.stdout


def run_command(argv: tuple[str, ...], timeout: int = 900) -> tuple[int, bytes]:
    executable = shutil.which(argv[0])
    if executable is None:
        return 127, b"executable-unavailable"
    result = subprocess.run(
        (executable, *argv[1:]),
        cwd=ROOT,
        check=False,
        capture_output=True,
        timeout=timeout,
    )
    return result.returncode, result.stdout + result.stderr


def run_commands() -> list[dict[str, Any]]:
    records = []
    for identifier, argv in COMMANDS:
        exit_code, output = run_command(argv)
        records.append(
            {
                "id": identifier,
                "argv": list(argv),
                "exit_code": exit_code,
                "output_sha256": sha256_bytes(output),
            }
        )
    return records


def run_grammar_bom() -> dict[str, Any]:
    exit_code, output = run_command(GRAMMAR_COMMAND)
    if exit_code != 0:
        raise ValueError("compiled grammar BOM command failed")
    try:
        return json.loads(output)
    except (UnicodeError, json.JSONDecodeError) as failure:
        raise ValueError("compiled grammar BOM is malformed") from failure


def build_report(
    source_revision: str,
    commands: list[dict[str, Any]],
    grammar_bom: dict[str, Any],
) -> dict[str, Any]:
    local_pass = all(command["exit_code"] == 0 for command in commands)
    return {
        "schema_version": 1,
        "record_type": "sprint_19_local_evidence",
        "source_revision": source_revision,
        "source_sha256": {
            path: sha256_bytes(git_file(source_revision, path)) for path in SOURCE_PATHS
        },
        "commands": commands,
        "grammar_bom": grammar_bom,
        "security_requirement_ids": SECURITY_REQUIREMENTS,
        "implemented_contracts": IMPLEMENTED_CONTRACTS,
        "verification_evidence": {
            "coverage_matrix": local_pass,
            "selective_invalidation": local_pass,
            "source_resolution": local_pass,
            "lexical_fallback_unknown_blocked": local_pass,
            "golden_hashes": local_pass,
            "disposable_git_invariance": local_pass,
            "parser_cancellation": local_pass,
            "rust_parser_panic_containment": local_pass,
            "pre_citation_cache_reconciliation": local_pass,
            "one_use_freshness_permit": local_pass,
            "manual_parser_fuzzing": False,
            "independent_review": False,
        },
        "platform_evidence": {
            "local_pure_core": local_pass,
            "fedora_held_repository_projection": local_pass,
            "fedora_encrypted_derivative_cache": local_pass,
            "ubuntu_native_map": False,
            "macos_native_map": False,
            "windows_native_map": False,
        },
        "blockers": BLOCKERS,
        "summary": {
            "local_contract_passed": local_pass,
            "sprint_status": "BLOCKED",
            "release_approval": False,
        },
    }


def validate_grammar_bom(value: Any) -> list[str]:
    if not isinstance(value, dict) or set(value) != {
        "grammar_set_sha256",
        "grammars",
        "schema_version",
    }:
        return ["grammar BOM fields invalid"]
    grammars = value.get("grammars")
    if (
        value.get("schema_version") != 1
        or not SHA256.fullmatch(str(value.get("grammar_set_sha256", "")))
        or not isinstance(grammars, list)
        or len(grammars) != 6
    ):
        return ["grammar BOM identity invalid"]
    return []


def validate_report(report: dict[str, Any], verify_current: bool = True) -> list[str]:
    failures: list[str] = []
    revision = str(report.get("source_revision", ""))
    if not REVISION.fullmatch(revision):
        failures.append("source revision invalid")
    if report.get("security_requirement_ids") != SECURITY_REQUIREMENTS:
        failures.append("security mapping drift")
    if report.get("blockers") != BLOCKERS:
        failures.append("blocker drift")
    if report.get("implemented_contracts") != IMPLEMENTED_CONTRACTS:
        failures.append("implemented-contract inventory drift")
    commands = report.get("commands", [])
    if [item.get("id") for item in commands] != [item[0] for item in COMMANDS]:
        failures.append("command inventory drift")
    if any(
        item.get("exit_code") != 0
        or not SHA256.fullmatch(str(item.get("output_sha256", "")))
        for item in commands
    ):
        failures.append("command result invalid")
    failures.extend(validate_grammar_bom(report.get("grammar_bom")))
    if report.get("summary") != {
        "local_contract_passed": True,
        "sprint_status": "BLOCKED",
        "release_approval": False,
    }:
        failures.append("summary overclaim or local failure")
    for field in ("ubuntu_native_map", "macos_native_map", "windows_native_map"):
        if report.get("platform_evidence", {}).get(field) is not False:
            failures.append(f"platform overclaim: {field}")
    for field in (
        "local_pure_core",
        "fedora_held_repository_projection",
        "fedora_encrypted_derivative_cache",
    ):
        if report.get("platform_evidence", {}).get(field) is not True:
            failures.append(f"missing local platform evidence: {field}")
    for field in (
        "coverage_matrix",
        "selective_invalidation",
        "source_resolution",
        "lexical_fallback_unknown_blocked",
        "golden_hashes",
        "disposable_git_invariance",
        "parser_cancellation",
        "rust_parser_panic_containment",
        "pre_citation_cache_reconciliation",
        "one_use_freshness_permit",
    ):
        if report.get("verification_evidence", {}).get(field) is not True:
            failures.append(f"missing local verification evidence: {field}")
    for field in ("manual_parser_fuzzing", "independent_review"):
        if report.get("verification_evidence", {}).get(field) is not False:
            failures.append(f"verification overclaim: {field}")
    if verify_current and REVISION.fullmatch(revision):
        for path in SOURCE_PATHS:
            digest = str(report.get("source_sha256", {}).get(path, ""))
            if not SHA256.fullmatch(digest) or digest != sha256_bytes(git_file(revision, path)):
                failures.append(f"source digest drift: {path}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    args = parser.parse_args()
    if args.write:
        revision = subprocess.run(
            ["git", "rev-parse", args.source_revision],
            cwd=ROOT,
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
        report = build_report(revision, run_commands(), run_grammar_bom())
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    else:
        report = json.loads(OUTPUT.read_text(encoding="utf-8"))
    failures = validate_report(report)
    if failures:
        for failure in failures:
            print(failure)
        return 1
    print(json.dumps(report["summary"], sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
