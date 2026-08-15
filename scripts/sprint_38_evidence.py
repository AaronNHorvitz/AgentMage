#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 38 evidence."""

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
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-38/local-evidence-report.json"
CORPUS_PATH: Final = "docs/verification/sprint-38-markdown-knowledge-corpus.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
IGNORED_TESTS: Final = re.compile(
    rb"test result: (?:ok|FAILED)\. \d+ passed; \d+ failed; (\d+) ignored;"
)
SOURCE_PATHS: Final = (
    "capabilities/knowledge/src/markdown_write.rs",
    "capabilities/knowledge/src/knowledge_write.rs",
    "capabilities/knowledge/src/obsidian_index.rs",
    "capabilities/knowledge/src/plain_folder.rs",
    "capabilities/knowledge/src/obsidian.rs",
    "shells/host/src/knowledge_write.rs",
    "architecture/dependency-rules.json",
    "security/strict-local-source-policy.json",
    "docs/architecture/controlled-markdown-knowledge-writes.md",
    "docs/verification/sprint-38-local-results.md",
    CORPUS_PATH,
    "scripts/sprint_38_evidence.py",
    "tests/test_sprint_38_evidence.py",
)
COMMANDS: Final = (
    (
        "knowledge-tests",
        ("cargo", "test", "-p", "agentmage-capability-knowledge", "--locked"),
    ),
    (
        "host-knowledge-tests",
        ("cargo", "test", "-p", "agentmage-host", "knowledge_write", "--locked"),
    ),
    (
        "host-library-tests",
        ("cargo", "test", "-p", "agentmage-host", "--lib", "--locked"),
    ),
    (
        "knowledge-host-clippy",
        (
            "cargo", "clippy", "-p", "agentmage-capability-knowledge", "-p",
            "agentmage-host", "--all-targets", "--locked", "--", "-D", "warnings",
        ),
    ),
    ("documentation-gate", ("npm", "run", "docs:check")),
    ("dependency-rules", ("python3", "scripts/dependency_rules.py")),
    ("effect-boundary", ("python3", "scripts/effect_boundary.py")),
    ("build-contract", ("python3", "scripts/build_contract.py")),
    ("dependency-classes", ("python3", "scripts/dependency_classes.py")),
    ("strict-local-source", ("python3", "scripts/strict_local_source_audit.py")),
    ("supply-chain", ("python3", "scripts/supply_chain.py")),
    ("evidence-tests", ("python3", "-m", "unittest", "tests.test_sprint_38_evidence")),
)
FOCUSED_COMMANDS: Final = ("knowledge-tests", "host-knowledge-tests")
SECURITY_REQUIREMENTS: Final = [
    "SR-ACC-004", "SR-ACC-005", "SR-ACC-006", "SR-ACC-007", "SR-ACC-008",
    "SR-DAT-001", "SR-DAT-002", "SR-DAT-003", "SR-CIV-003", "SR-CIV-004",
    "SR-TST-004", "SR-TST-005",
]
CORPUS_CLASSES: Final = {
    "adapter-parity": 1,
    "fidelity-warning": 4,
    "host-composition-denial": 2,
    "hostile-denial": 5,
    "index-recovery": 3,
    "namespace-denial": 3,
    "parser-valid": 4,
    "safe-refusal": 6,
    "scoped-update": 8,
}
BLOCKERS: Final = [
    {"code": "UPSTREAM-SPRINT-37-BLOCKED", "owner": "38.1"},
    {"code": "SPRINT-38-NATIVE-KNOWLEDGE-TRANSACTION-ABSENT", "owner": "38.1.3.4"},
    {"code": "SPRINT-38-CRASH-RACE-MATRIX-INCOMPLETE", "owner": "38.1.3.4"},
    {"code": "TRUSTED-PACKAGE-LAUNCHER-ENVIRONMENT-ABSENT", "owner": "38.1.3.5"},
    {"code": "SPRINT-38-NON-FEDORA-EVIDENCE-ABSENT", "owner": "38.1.3.5"},
    {"code": "INDEPENDENT-SPRINT-38-REVIEW-ABSENT", "owner": "38.1.3.5"},
    {"code": "MANUAL-FUZZING-DEFERRED", "owner": "SR-TST-004"},
]
IMPLEMENTED: Final = {
    "byte_preserving_markdown_parser": True,
    "closed_structure_preserving_edits": True,
    "raw_notes_protected": True,
    "exact_create_and_update_previews": True,
    "stable_identity_and_source_hash_binding": True,
    "case_and_unicode_namespace_collision_denial": True,
    "wiki_link_target_validation": True,
    "seven_closed_creation_workflows": True,
    "single_file_high_risk_structural_previews": True,
    "bulk_reorganization_unrepresentable": True,
    "host_to_kernel_draft_composition": True,
    "namespace_compare_and_swap": True,
    "canonical_first_index_publication": True,
    "plain_folder_obsidian_domain_parity": True,
    "native_end_to_end_knowledge_transaction": False,
    "complete_native_crash_and_race_matrix": False,
    "trusted_package_launcher_test_environment": False,
    "non_fedora_native_evidence": False,
    "independent_review": False,
    "manual_fuzzing_complete": False,
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
        raise ValueError(f"committed Sprint 38 source is absent: {path}")
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
        "system": platform.system(),
        "release": platform.release(),
        "machine": platform.machine(),
        "python": platform.python_version(),
        "rustc": version("rustc", "--version"),
        "cargo": version("cargo", "--version"),
        "node": version("node", "--version"),
        "npm": version("npm", "--version"),
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
        blocking_skip_count = None
        if identifier in FOCUSED_COMMANDS:
            matches = IGNORED_TESTS.findall(output)
            blocking_skip_count = sum(int(value) for value in matches) if matches else -1
        records.append({
            "id": identifier,
            "argv": list(argv),
            "exit_code": code,
            "output_sha256": digest(output),
            "blocking_skip_count": blocking_skip_count,
        })
    return records


def build_report(revision: str, commands: list[dict[str, Any]]) -> dict[str, Any]:
    focused = [item for item in commands if item["id"] in FOCUSED_COMMANDS]
    local_pass = (
        all(item["exit_code"] == 0 for item in commands)
        and len(focused) == len(FOCUSED_COMMANDS)
        and all(item.get("blocking_skip_count") == 0 for item in focused)
    )
    return {
        "schema_version": 1,
        "record_type": "sprint_38_local_evidence",
        "source_revision": revision,
        "source_sha256": {path: digest(git_file(revision, path)) for path in SOURCE_PATHS},
        "environment": environment_manifest(),
        "commands": commands,
        "security_requirement_ids": SECURITY_REQUIREMENTS,
        "implemented_contracts": IMPLEMENTED,
        "verification_evidence": {
            "focused_contracts": local_pass,
            "focused_blocking_skip_count": 0 if local_pass else None,
            "upstream_sprint_37_gate": False,
            "native_end_to_end_knowledge_transaction": False,
            "complete_native_crash_and_race_matrix": False,
            "trusted_package_launcher_test_environment": False,
            "non_fedora_native_evidence": False,
            "independent_review": False,
            "manual_fuzzing": False,
        },
        "blockers": BLOCKERS,
        "summary": {
            "local_markdown_knowledge_contract_passed": local_pass,
            "sprint_status": "BLOCKED",
            "upstream_dependency_passed": False,
            "native_end_to_end_passed": False,
            "crash_and_race_matrix_passed": False,
            "trusted_launcher_environment_passed": False,
            "cross_platform_evidence_passed": False,
            "independent_review_passed": False,
            "manual_fuzzing_complete": False,
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
    for identifier in FOCUSED_COMMANDS:
        focused = next((item for item in commands if item.get("id") == identifier), None)
        if focused is None or focused.get("blocking_skip_count") != 0:
            failures.append(f"focused skipped, suppressed, or unavailable check: {identifier}")
    if any(
        item.get("blocking_skip_count") is not None
        for item in commands
        if item.get("id") not in FOCUSED_COMMANDS
    ):
        failures.append("supporting command skip count must remain not-applicable")
    expected_summary = {
        "local_markdown_knowledge_contract_passed": True,
        "sprint_status": "BLOCKED",
        "upstream_dependency_passed": False,
        "native_end_to_end_passed": False,
        "crash_and_race_matrix_passed": False,
        "trusted_launcher_environment_passed": False,
        "cross_platform_evidence_passed": False,
        "independent_review_passed": False,
        "manual_fuzzing_complete": False,
        "network_access_enabled": False,
        "release_approval": False,
    }
    if report.get("summary") != expected_summary:
        failures.append("summary overclaim or local failure")
    verification = report.get("verification_evidence", {})
    if verification.get("focused_contracts") is not True:
        failures.append("focused contract failure")
    if verification.get("focused_blocking_skip_count") != 0:
        failures.append("focused blocking skip summary invalid")
    for field in (
        "upstream_sprint_37_gate",
        "native_end_to_end_knowledge_transaction",
        "complete_native_crash_and_race_matrix",
        "trusted_package_launcher_test_environment",
        "non_fedora_native_evidence",
        "independent_review",
        "manual_fuzzing",
    ):
        if verification.get(field) is not False:
            failures.append(f"verification overclaim: {field}")
    for field in (
        "native_end_to_end_knowledge_transaction",
        "complete_native_crash_and_race_matrix",
        "trusted_package_launcher_test_environment",
        "non_fedora_native_evidence",
        "independent_review",
        "manual_fuzzing_complete",
        "generic_shell",
        "network_access",
        "external_delivery",
    ):
        if report.get("implemented_contracts", {}).get(field) is not False:
            failures.append(f"capability overclaim: {field}")
    environment = report.get("environment", {})
    if set(environment) != {
        "system", "release", "machine", "python", "rustc", "cargo", "node", "npm",
    }:
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
    corpus = json.loads(git_file(revision, CORPUS_PATH)) if REVISION.fullmatch(revision) else {}
    cases = corpus.get("cases", []) if isinstance(corpus, dict) else []
    class_counts = {
        category: sum(item.get("class") == category for item in cases)
        for category in CORPUS_CLASSES
    }
    if (
        corpus.get("privacy") != "public-synthetic-only"
        or len(cases) != sum(CORPUS_CLASSES.values())
        or len({item.get("id") for item in cases}) != len(cases)
        or class_counts != CORPUS_CLASSES
        or any(not item.get("test_reference") for item in cases)
    ):
        failures.append("synthetic corpus drift")
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
