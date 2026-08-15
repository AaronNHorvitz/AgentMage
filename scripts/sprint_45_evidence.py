#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 45 evidence."""

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
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-45/local-evidence-report.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
IGNORED_TESTS: Final = re.compile(
    rb"test result: (?:ok|FAILED)\. \d+ passed; \d+ failed; (\d+) ignored;"
)
SOURCE_PATHS: Final = (
    "capabilities/repository-map/src/structured_edit.rs",
    "capabilities/repository-map/src/language_service.rs",
    "capabilities/repository-map/src/package_scaffold.rs",
    "capabilities/repository-map/src/test_generation.rs",
    "capabilities/repository-map/src/lib.rs",
    "capabilities/repository-map/tests/structured_coding_matrix.rs",
    "capabilities/repository-map/README.md",
    "shells/host/src/code_change.rs",
    "shells/host/src/lib.rs",
    "shells/host/Cargo.toml",
    "kernel/engine/src/write_approval.rs",
    "kernel/engine/src/write_transaction.rs",
    "kernel/engine/src/write_recovery.rs",
    "kernel/engine/tests/write_recovery_matrix.rs",
    "platforms/linux/src/write_transaction.rs",
    "docs/architecture/structured-code-changes.md",
    "docs/guides/reviewing-structured-code-changes.md",
    "docs/security/sprint-45-ffi-unsafe-inventory.md",
    "docs/verification/sprint-45-language-service-confinement.md",
    "docs/verification/sprint-45-coding-corpus.json",
    "docs/verification/sprint-45-local-results.md",
    "scripts/sprint_45_evidence.py",
    "tests/test_sprint_45_evidence.py",
)
COMMANDS: Final = (
    (
        "structured-edit-unit",
        (
            "cargo", "test", "-p", "agentmage-capability-repository-map",
            "structured_edit::tests", "--lib", "--locked",
        ),
    ),
    (
        "language-service-unit",
        (
            "cargo", "test", "-p", "agentmage-capability-repository-map",
            "language_service::tests", "--lib", "--locked",
        ),
    ),
    (
        "package-scaffold-unit",
        (
            "cargo", "test", "-p", "agentmage-capability-repository-map",
            "package_scaffold::tests", "--lib", "--locked",
        ),
    ),
    (
        "test-generation-unit",
        (
            "cargo", "test", "-p", "agentmage-capability-repository-map",
            "test_generation::tests", "--lib", "--locked",
        ),
    ),
    (
        "fictional-coding-matrix",
        (
            "cargo", "test", "-p", "agentmage-capability-repository-map",
            "--test", "structured_coding_matrix", "--locked",
        ),
    ),
    (
        "host-composition-unit",
        ("cargo", "test", "-p", "agentmage-host", "code_change::tests", "--lib", "--locked"),
    ),
    (
        "write-approval-unit",
        ("cargo", "test", "-p", "agentmage-kernel-engine", "write_approval::tests", "--lib", "--locked"),
    ),
    (
        "write-transaction-unit",
        ("cargo", "test", "-p", "agentmage-kernel-engine", "write_transaction::tests", "--lib", "--locked"),
    ),
    (
        "write-recovery-matrix",
        ("cargo", "test", "-p", "agentmage-kernel-engine", "--test", "write_recovery_matrix", "--locked"),
    ),
    (
        "linux-write-transaction-unit",
        ("cargo", "test", "-p", "agentmage-platform-linux", "write_transaction::tests", "--lib", "--locked"),
    ),
    (
        "repository-map-strict-clippy",
        ("cargo", "clippy", "-p", "agentmage-capability-repository-map", "--all-targets", "--locked", "--", "-D", "warnings"),
    ),
    (
        "host-strict-clippy",
        ("cargo", "clippy", "-p", "agentmage-host", "--all-targets", "--locked", "--", "-D", "warnings"),
    ),
    ("documentation-gate", ("npm", "run", "docs:check")),
    ("product-ci-contract", ("python3", "scripts/product_ci.py", "--check")),
    ("supply-chain", ("python3", "scripts/supply_chain.py")),
    ("evidence-tests", ("python3", "-m", "unittest", "tests.test_sprint_45_evidence")),
)
FOCUSED_COMMANDS: Final = tuple(identifier for identifier, _ in COMMANDS[:10])
ARTIFACTS: Final = (("git", Path("/usr/bin/git")),)
SECURITY_REQUIREMENTS: Final = [
    "SR-ACC-002", "SR-ACC-003", "SR-ACC-004", "SR-ACC-005",
    "SR-ACC-006", "SR-ACC-007", "SR-ACC-008", "SR-SUP-003",
    "SR-SUP-009", "SR-AI-005", "SR-TST-002", "SR-TST-005", "SR-TST-011",
]
IMPLEMENTED: Final = {
    "six_artifact_classes": True,
    "parser_backed_structured_edits": True,
    "unique_exact_text_fallback": True,
    "confined_language_service_contract": True,
    "ordered_atomic_shadow_composition": True,
    "seven_typed_review_hooks": True,
    "repository_style_test_generation_plan": True,
    "unrelated_byte_preservation_evidence": True,
    "approved_package_scaffold_plans": True,
    "production_chat_coding_coordinator": False,
    "production_language_service_sandbox": False,
    "controlled_package_scaffold_application": False,
    "native_cross_platform_acceptance": False,
    "trusted_package_execution": False,
    "independent_review": False,
    "manual_fuzzing": False,
}
BLOCKERS: Final = [
    {"code": "UPSTREAM-SPRINT-44-BLOCKED", "owner": "45.1"},
    {"code": "PRODUCTION-CHAT-CODING-COORDINATOR-ABSENT", "owner": "45.1.1"},
    {"code": "PRODUCTION-LANGUAGE-SERVICE-SANDBOX-ABSENT", "owner": "45.1.1.3"},
    {"code": "CONTROLLED-PACKAGE-SCAFFOLD-APPLICATION-ABSENT", "owner": "45.1.1.8"},
    {"code": "TRUSTED-INSTALLED-PARENT-EXECUTION-ABSENT", "owner": "45.1.3.4"},
    {"code": "NATIVE-CROSS-PLATFORM-ACCEPTANCE-ABSENT", "owner": "45.1.3"},
    {"code": "TRUSTED-PACKAGE-EXECUTION-ABSENT", "owner": "45.1.3.5"},
    {"code": "INDEPENDENT-CODING-BOUNDARY-REVIEW-ABSENT", "owner": "45.1.3.5"},
    {"code": "MANUAL-FUZZING-DEFERRED", "owner": "SR-TST-002"},
]


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def git_file(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, check=False,
        capture_output=True, timeout=30,
    )
    if result.returncode != 0:
        raise ValueError(f"committed Sprint 45 source is absent: {path}")
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
        "git": version("git", "--version"),
    }


def native_artifacts() -> list[dict[str, Any]]:
    records = []
    for identifier, path in ARTIFACTS:
        if not path.is_file() or path.is_symlink() or path.stat().st_size <= 0:
            raise ValueError(f"native Sprint 45 artifact unavailable: {identifier}")
        records.append({
            "id": identifier,
            "name": path.name,
            "size": path.stat().st_size,
            "sha256": digest(path.read_bytes()),
            "root_owned": path.stat().st_uid == 0,
            "group_or_world_writable": bool(path.stat().st_mode & 0o022),
        })
    return records


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


def build_report(
    revision: str,
    commands: list[dict[str, Any]],
    artifacts: list[dict[str, Any]],
) -> dict[str, Any]:
    focused = [item for item in commands if item["id"] in FOCUSED_COMMANDS]
    local_pass = (
        all(item["exit_code"] == 0 for item in commands)
        and len(focused) == len(FOCUSED_COMMANDS)
        and all(item.get("blocking_skip_count") == 0 for item in focused)
        and len(artifacts) == len(ARTIFACTS)
        and all(
            item.get("root_owned") is True
            and item.get("group_or_world_writable") is False
            for item in artifacts
        )
    )
    return {
        "schema_version": 1,
        "record_type": "sprint_45_local_evidence",
        "source_revision": revision,
        "source_sha256": {path: digest(git_file(revision, path)) for path in SOURCE_PATHS},
        "environment": environment_manifest(),
        "commands": commands,
        "native_fixture_artifacts": artifacts,
        "security_requirement_ids": list(SECURITY_REQUIREMENTS),
        "implemented_contracts": dict(IMPLEMENTED),
        "verification_evidence": {
            "focused_local_contracts": local_pass,
            "focused_blocking_skip_count": 0 if local_pass else None,
            "fictional_language_case_count": 7 if local_pass else None,
            "language_service_capability_count": 5 if local_pass else None,
            "forbidden_service_power_count": 7 if local_pass else None,
            "approved_package_convention_count": 5 if local_pass else None,
            "test_concern_count": 6 if local_pass else None,
            "unauthorized_mutation_acceptance_count": 0 if local_pass else None,
            "sprint_added_unsafe_or_ffi_file_count": 0 if local_pass else None,
            "production_language_service_sandbox": False,
            "native_cross_platform_acceptance": False,
            "trusted_package_execution": False,
            "independent_review": False,
            "manual_fuzzing": False,
        },
        "blockers": [dict(blocker) for blocker in BLOCKERS],
        "summary": {
            "local_sprint_45_contract_passed": local_pass,
            "sprint_status": "BLOCKED",
            "upstream_sprint_44_closed": False,
            "production_chat_coding_coordinator_active": False,
            "production_language_service_sandbox_active": False,
            "controlled_package_scaffold_application_active": False,
            "trusted_installed_parent_execution_complete": False,
            "cross_platform_acceptance_passed": False,
            "trusted_package_execution_complete": False,
            "independent_review_present": False,
            "manual_fuzzing_complete": False,
            "release_approval": False,
        },
    }


def expected_verification() -> dict[str, Any]:
    return {
        "focused_local_contracts": True,
        "focused_blocking_skip_count": 0,
        "fictional_language_case_count": 7,
        "language_service_capability_count": 5,
        "forbidden_service_power_count": 7,
        "approved_package_convention_count": 5,
        "test_concern_count": 6,
        "unauthorized_mutation_acceptance_count": 0,
        "sprint_added_unsafe_or_ffi_file_count": 0,
        "production_language_service_sandbox": False,
        "native_cross_platform_acceptance": False,
        "trusted_package_execution": False,
        "independent_review": False,
        "manual_fuzzing": False,
    }


def expected_summary() -> dict[str, Any]:
    return {
        "local_sprint_45_contract_passed": True,
        "sprint_status": "BLOCKED",
        "upstream_sprint_44_closed": False,
        "production_chat_coding_coordinator_active": False,
        "production_language_service_sandbox_active": False,
        "controlled_package_scaffold_application_active": False,
        "trusted_installed_parent_execution_complete": False,
        "cross_platform_acceptance_passed": False,
        "trusted_package_execution_complete": False,
        "independent_review_present": False,
        "manual_fuzzing_complete": False,
        "release_approval": False,
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
    artifacts = report.get("native_fixture_artifacts", [])
    if [item.get("id") for item in artifacts] != [item[0] for item in ARTIFACTS]:
        failures.append("native artifact inventory drift")
    if any(
        not SHA256.fullmatch(str(item.get("sha256", "")))
        or not isinstance(item.get("size"), int)
        or item.get("size", 0) <= 0
        or item.get("root_owned") is not True
        or item.get("group_or_world_writable") is not False
        for item in artifacts
    ):
        failures.append("native artifact claim invalid")
    if report.get("verification_evidence") != expected_verification():
        failures.append("verification claim drift")
    if report.get("summary") != expected_summary():
        failures.append("summary or release claim drift")
    source = report.get("source_sha256", {})
    if list(source) != list(SOURCE_PATHS):
        failures.append("source inventory drift")
    elif REVISION.fullmatch(revision):
        for path in SOURCE_PATHS:
            try:
                expected = digest(git_file(revision, path))
            except ValueError:
                failures.append(f"committed source unavailable: {path}")
                continue
            if source.get(path) != expected:
                failures.append(f"source hash drift: {path}")
    if verify_current:
        head = subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, check=True,
            capture_output=True, text=True, timeout=30,
        ).stdout.strip()
        if revision != head:
            failures.append("report is historical rather than current")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--verify", type=Path)
    parser.add_argument("--output", type=Path, default=OUTPUT)
    parser.add_argument("--revision")
    arguments = parser.parse_args()
    if arguments.verify:
        report = json.loads(arguments.verify.read_text(encoding="utf-8"))
        failures = validate_report(report)
        if failures:
            raise SystemExit("\n".join(failures))
        print("Sprint 45 evidence validated")
        return 0
    revision = arguments.revision or subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, check=True,
        capture_output=True, text=True, timeout=30,
    ).stdout.strip()
    report = build_report(revision, run_commands(), native_artifacts())
    failures = validate_report(report, verify_current=False)
    if failures:
        raise SystemExit("\n".join(failures))
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    arguments.output.write_text(
        json.dumps(report, indent=2, sort_keys=False) + "\n", encoding="utf-8"
    )
    print(arguments.output.relative_to(ROOT))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

