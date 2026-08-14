#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 18 evidence."""

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
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-18/local-evidence-report.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
SOURCE_PATHS: Final = (
    "capabilities/repository-map/Cargo.toml",
    "capabilities/repository-map/examples/grammar_bom.rs",
    "capabilities/repository-map/src/cache.rs",
    "capabilities/repository-map/src/grammar.rs",
    "capabilities/repository-map/src/inventory.rs",
    "capabilities/repository-map/src/lib.rs",
    "capabilities/repository-map/src/parser.rs",
    "docs/architecture/pinned-repository-map.md",
    "docs/verification/sprint-18-local-results.md",
    "scripts/sprint_18_evidence.py",
    "tests/test_sprint_18_evidence.py",
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
    ("build-contract", ("python3", "scripts/build_contract.py")),
    ("dependency-rules", ("python3", "scripts/dependency_rules.py")),
    ("module-inventory", ("python3", "scripts/module_inventory.py")),
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
    {"code": "PACKAGED-REPOSITORY-MAP-WORKER-NOT-INTEGRATED", "owner": "18.1.1.2"},
    {"code": "LIVE-GITIGNORE-POLICY-PROJECTION-NOT-INTEGRATED", "owner": "18.1.1.2"},
    {"code": "ENCRYPTED-PERSISTENT-MAP-CACHE-NOT-INTEGRATED", "owner": "18.1.1.4"},
    {"code": "HOST-CANCELLATION-AND-FAILURE-MATRIX-INCOMPLETE", "owner": "18.1.3.1"},
    {"code": "NATIVE-PLATFORM-MAP-EVIDENCE-INCOMPLETE", "owner": "18.1.3.2"},
    {"code": "MANUAL-PARSER-FUZZING-DEFERRED", "owner": "18.1.3.3"},
    {"code": "INDEPENDENT-SPRINT-18-REVIEW-NOT-RETAINED", "owner": "18.1.3.3"},
]


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
        raise ValueError(f"committed Sprint 18 source is absent: {relative}")
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
    records: list[dict[str, Any]] = []
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
        value = json.loads(output)
    except (UnicodeError, json.JSONDecodeError) as failure:
        raise ValueError("compiled grammar BOM is malformed") from failure
    return value


def build_report(
    source_revision: str,
    commands: list[dict[str, Any]],
    grammar_bom: dict[str, Any],
) -> dict[str, Any]:
    local_pass = all(command["exit_code"] == 0 for command in commands)
    return {
        "schema_version": 1,
        "record_type": "sprint_18_local_evidence",
        "source_revision": source_revision,
        "source_sha256": {
            path: sha256_bytes(git_file(source_revision, path)) for path in SOURCE_PATHS
        },
        "commands": commands,
        "grammar_bom": grammar_bom,
        "security_requirement_ids": SECURITY_REQUIREMENTS,
        "implemented_contracts": {
            "supported_language_dialects": 6,
            "parser_runtime_version": "0.26.12",
            "maximum_source_bytes": 4 * 1024 * 1024,
            "maximum_structural_items": 10_000,
            "reliable_relationship_kinds": 1,
            "cache_key_dimensions": 6,
            "filesystem_authority": False,
            "process_authority": False,
            "network_authority": False,
            "excluded_content_admitted": False,
        },
        "platform_evidence": {
            "local_pure_core": local_pass,
            "linux_packaged_worker": False,
            "linux_encrypted_persistent_cache": False,
            "macos_native_map": False,
            "windows_native_map": False,
        },
        "verification_evidence": {
            "grammar_identity": local_pass,
            "deterministic_inventory": local_pass,
            "parser_range_and_hash_integrity": local_pass,
            "exact_cache_invalidation": local_pass,
            "manual_parser_fuzzing": False,
            "independent_review": False,
        },
        "blockers": BLOCKERS,
        "summary": {
            "local_contract_passed": local_pass,
            "sprint_status": "BLOCKED",
            "release_approval": False,
        },
    }


def validate_grammar_bom(value: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict) or set(value) != {
        "grammar_set_sha256",
        "grammars",
        "schema_version",
    }:
        return ["grammar BOM fields invalid"]
    grammars = value.get("grammars")
    if value.get("schema_version") != 1 or not SHA256.fullmatch(
        str(value.get("grammar_set_sha256", ""))
    ):
        failures.append("grammar BOM identity invalid")
    if not isinstance(grammars, list) or len(grammars) != 6:
        failures.append("grammar BOM count invalid")
        return failures
    expected = ["rust", "python", "type_script", "tsx", "java_script", "swift"]
    if [item.get("language") for item in grammars if isinstance(item, dict)] != expected:
        failures.append("grammar BOM language order drift")
    for item in grammars:
        if not isinstance(item, dict) or set(item) != {
            "abi_version",
            "crate_name",
            "crate_version",
            "descriptor_sha256",
            "language",
            "node_types_sha256",
            "parser_version",
            "repository",
        }:
            failures.append("grammar descriptor fields invalid")
            continue
        if item.get("parser_version") != "0.26.12" or not SHA256.fullmatch(
            str(item.get("descriptor_sha256", ""))
        ) or not SHA256.fullmatch(str(item.get("node_types_sha256", ""))):
            failures.append("grammar descriptor identity invalid")
    return failures


def validate_report(report: dict[str, Any], verify_current: bool = True) -> list[str]:
    failures: list[str] = []
    revision = str(report.get("source_revision", ""))
    if not REVISION.fullmatch(revision):
        failures.append("source revision invalid")
    if report.get("security_requirement_ids") != SECURITY_REQUIREMENTS:
        failures.append("security mapping drift")
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
    failures.extend(validate_grammar_bom(report.get("grammar_bom")))
    expected_contracts = {
        "supported_language_dialects": 6,
        "parser_runtime_version": "0.26.12",
        "maximum_source_bytes": 4 * 1024 * 1024,
        "maximum_structural_items": 10_000,
        "reliable_relationship_kinds": 1,
        "cache_key_dimensions": 6,
        "filesystem_authority": False,
        "process_authority": False,
        "network_authority": False,
        "excluded_content_admitted": False,
    }
    if report.get("implemented_contracts") != expected_contracts:
        failures.append("implemented-contract inventory drift")
    if report.get("summary") != {
        "local_contract_passed": True,
        "sprint_status": "BLOCKED",
        "release_approval": False,
    }:
        failures.append("summary overclaim or local failure")
    platform = report.get("platform_evidence", {})
    verification = report.get("verification_evidence", {})
    for field in (
        "linux_packaged_worker",
        "linux_encrypted_persistent_cache",
        "macos_native_map",
        "windows_native_map",
    ):
        if platform.get(field) is not False:
            failures.append(f"platform overclaim: {field}")
    for field in ("manual_parser_fuzzing", "independent_review"):
        if verification.get(field) is not False:
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
