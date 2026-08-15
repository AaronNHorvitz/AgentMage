#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 46 evidence."""

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
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-46/local-evidence-report.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
IGNORED_TESTS: Final = re.compile(
    rb"test result: (?:ok|FAILED)\. \d+ passed; \d+ failed; (\d+) ignored;"
)
SOURCE_PATHS: Final = (
    "kernel/engine/src/command_runner.rs",
    "kernel/engine/src/validation_template.rs",
    "kernel/engine/src/validation_result.rs",
    "kernel/engine/src/lib.rs",
    "schemas/runtime/validation-receipt.schema.json",
    "schemas/runtime/examples/validation-receipt.valid.json",
    "scripts/validate_planning_schemas.mjs",
    "tests/test_planning_schemas.mjs",
    "docs/verification/sprint-46-failure-corpus.json",
    "tests/test_sprint_46_validation_corpus.py",
    "docs/architecture/trusted-validation-runner.md",
    "docs/guides/reviewing-validation-results.md",
    "docs/verification/sprint-46-local-results.md",
    "README.md",
    "scripts/sprint_46_evidence.py",
    "tests/test_sprint_46_evidence.py",
)
COMMANDS: Final = (
    (
        "validation-template-unit",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine",
            "validation_template::tests", "--lib", "--locked",
        ),
    ),
    (
        "command-runner-unit",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine",
            "command_runner::tests", "--lib", "--locked",
        ),
    ),
    (
        "validation-result-unit",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine",
            "validation_result::tests", "--lib", "--locked",
        ),
    ),
    (
        "kernel-library",
        ("cargo", "test", "-p", "agentmage-kernel-engine", "--lib", "--locked"),
    ),
    (
        "validation-corpus",
        ("python3", "-m", "unittest", "tests.test_sprint_46_validation_corpus"),
    ),
    (
        "runtime-schema-contract",
        ("node", "--test", "tests/test_planning_schemas.mjs"),
    ),
    (
        "kernel-strict-clippy",
        (
            "cargo", "clippy", "-p", "agentmage-kernel-engine",
            "--all-targets", "--locked", "--", "-D", "warnings",
        ),
    ),
    ("documentation-gate", ("npm", "run", "docs:check")),
    ("product-ci-contract", ("python3", "scripts/product_ci.py", "--check")),
    ("supply-chain", ("python3", "scripts/supply_chain.py")),
    ("evidence-tests", ("python3", "-m", "unittest", "tests.test_sprint_46_evidence")),
)
FOCUSED_COMMANDS: Final = tuple(identifier for identifier, _ in COMMANDS[:3])
ARTIFACTS: Final = ((
    "cargo-toolchain-dispatcher",
    Path(shutil.which("cargo") or "/unavailable/cargo").resolve(),
),)
SECURITY_REQUIREMENTS: Final = [
    "SR-OPS-003", "SR-SUP-003", "SR-TST-001", "SR-TST-002", "SR-TST-003",
    "SR-TST-004", "SR-TST-005", "SR-TST-006", "SR-TST-010",
]
IMPLEMENTED: Final = {
    "trusted_command_template_registry": True,
    "separate_template_and_scope_approval_contract": True,
    "focused_selection_without_argument_rewrite": True,
    "exact_terminal_command_receipt_verification": True,
    "strict_non_conflated_result_parser": True,
    "independent_artifact_and_affected_file_observation": True,
    "partial_and_unrun_validation_evidence": True,
    "deterministic_failure_classification": True,
    "idempotent_separately_approved_rerun_plan": True,
    "secret_safe_detailed_validation_receipt": True,
    "production_chat_validation_coordinator": False,
    "native_validation_worker_campaign": False,
    "protected_raw_log_integration": False,
    "native_cross_platform_acceptance": False,
    "trusted_package_execution": False,
    "independent_review": False,
    "manual_fuzzing": False,
}
BLOCKERS: Final = [
    {"code": "UPSTREAM-SPRINT-45-BLOCKED", "owner": "46.1"},
    {"code": "PRODUCTION-CHAT-VALIDATION-COORDINATOR-ABSENT", "owner": "46.1.1"},
    {"code": "TRUSTED-INSTALLED-PARENT-EXECUTION-ABSENT", "owner": "46.1.3.4"},
    {"code": "NATIVE-VALIDATION-WORKER-CAMPAIGN-ABSENT", "owner": "46.1.3.3"},
    {"code": "PROTECTED-RAW-LOG-INTEGRATION-ABSENT", "owner": "46.1.3.5"},
    {"code": "NATIVE-CROSS-PLATFORM-ACCEPTANCE-ABSENT", "owner": "46.1.3"},
    {"code": "TRUSTED-PACKAGE-EXECUTION-ABSENT", "owner": "46.1.3.5"},
    {"code": "INDEPENDENT-VALIDATION-BOUNDARY-REVIEW-ABSENT", "owner": "46.1.3.5"},
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
        raise ValueError(f"committed Sprint 46 source is absent: {path}")
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
            raise ValueError(f"native Sprint 46 artifact unavailable: {identifier}")
        records.append({
            "id": identifier,
            "name": path.name,
            "size": path.stat().st_size,
            "sha256": digest(path.read_bytes()),
            "owner_is_current_user": path.stat().st_uid == Path.home().stat().st_uid,
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


def expected_verification(local_pass: bool = True) -> dict[str, Any]:
    return {
        "focused_local_contracts": local_pass,
        "focused_blocking_skip_count": 0 if local_pass else None,
        "validation_kind_count": 9 if local_pass else None,
        "normalized_result_state_count": 14 if local_pass else None,
        "failure_classification_count": 8 if local_pass else None,
        "unauthorized_command_acceptance_count": 0 if local_pass else None,
        "process_stream_value_field_count": 0 if local_pass else None,
        "sprint_added_unsafe_or_ffi_file_count": 0 if local_pass else None,
        "production_validation_coordinator": False,
        "native_worker_campaign": False,
        "protected_raw_log_integration": False,
        "native_cross_platform_acceptance": False,
        "trusted_package_execution": False,
        "independent_review": False,
        "manual_fuzzing": False,
    }


def expected_summary(local_pass: bool = True) -> dict[str, Any]:
    return {
        "local_sprint_46_contract_passed": local_pass,
        "sprint_status": "BLOCKED",
        "upstream_sprint_45_closed": False,
        "production_chat_validation_coordinator_active": False,
        "trusted_installed_parent_execution_complete": False,
        "native_validation_worker_campaign_complete": False,
        "protected_raw_log_integration_complete": False,
        "cross_platform_acceptance_passed": False,
        "trusted_package_execution_complete": False,
        "independent_review_present": False,
        "manual_fuzzing_complete": False,
        "release_approval": False,
    }


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
            item.get("owner_is_current_user") is True
            and item.get("group_or_world_writable") is False
            for item in artifacts
        )
    )
    return {
        "schema_version": 1,
        "record_type": "sprint_46_local_evidence",
        "source_revision": revision,
        "source_sha256": {path: digest(git_file(revision, path)) for path in SOURCE_PATHS},
        "environment": environment_manifest(),
        "commands": commands,
        "native_fixture_artifacts": artifacts,
        "security_requirement_ids": list(SECURITY_REQUIREMENTS),
        "implemented_contracts": dict(IMPLEMENTED),
        "verification_evidence": expected_verification(local_pass),
        "blockers": [dict(blocker) for blocker in BLOCKERS],
        "summary": expected_summary(local_pass),
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
    if len(commands) != len(COMMANDS) or any(
        item.get("argv") != list(expected[1])
        or item.get("exit_code") != 0
        or not SHA256.fullmatch(str(item.get("output_sha256", "")))
        for item, expected in zip(commands, COMMANDS, strict=False)
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
        or item.get("owner_is_current_user") is not True
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
            except ValueError as error:
                failures.append(str(error))
                continue
            if source.get(path) != expected:
                failures.append(f"source digest drift: {path}")
    if verify_current and REVISION.fullmatch(revision):
        head = subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, check=False,
            capture_output=True, text=True, timeout=30,
        ).stdout.strip()
        if head != revision:
            failures.append("report source revision is not current HEAD")
    encoded = json.dumps(report, sort_keys=True).lower()
    for prohibited in (
        "credential_value", "secret_value", "raw_output", "stdout_content",
        "stderr_content", "remote_url", "repository_path",
    ):
        if prohibited in encoded:
            failures.append(f"prohibited evidence field: {prohibited}")
    return failures


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.write:
        revision = args.source_revision or subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, check=True,
            capture_output=True, text=True, timeout=30,
        ).stdout.strip()
        report = build_report(revision, run_commands(), native_artifacts())
        failures = validate_report(report, verify_current=False)
        if failures:
            for failure in failures:
                print(f"error: {failure}")
            return 1
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        print(f"wrote {OUTPUT.relative_to(ROOT)}")
        return 0
    if not OUTPUT.is_file():
        print(f"error: missing {OUTPUT.relative_to(ROOT)}")
        return 1
    failures = validate_report(json.loads(OUTPUT.read_text(encoding="utf-8")))
    if failures:
        for failure in failures:
            print(f"error: {failure}")
        return 1
    print("Sprint 46 local evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
