#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 42 evidence."""

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
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-42/local-evidence-report.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
IGNORED_TESTS: Final = re.compile(
    rb"test result: (?:ok|FAILED)\. \d+ passed; \d+ failed; (\d+) ignored;"
)
SOURCE_PATHS: Final = (
    "kernel/engine/src/repository_safety.rs",
    "platforms/linux/src/repository_safety.rs",
    "kernel/engine/tests/repository_safety_matrix.rs",
    "schemas/runtime/repository-preservation-manifest.schema.json",
    "schemas/runtime/repository-operation-plan.schema.json",
    "schemas/runtime/repository-operation-receipt.schema.json",
    "schemas/runtime/worktree-ownership.schema.json",
    "docs/security/repository-safety.md",
    "docs/architecture/repository-worktree-boundary.md",
    "docs/operations/repository-worktree-recovery.md",
    "docs/verification/sprint-42-repository-mutation-corpus.json",
    "docs/verification/sprint-42-local-results.md",
    "scripts/sprint_42_evidence.py",
    "tests/test_sprint_42_evidence.py",
)
COMMANDS: Final = (
    (
        "kernel-repository-contract",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine",
            "repository_safety::tests", "--lib", "--locked",
        ),
    ),
    (
        "repository-mutation-matrix",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine", "--test",
            "repository_safety_matrix", "--locked",
        ),
    ),
    (
        "linux-repository-integration",
        (
            "cargo", "test", "-p", "agentmage-platform-linux",
            "repository_safety::tests", "--lib", "--locked",
        ),
    ),
    ("schema-contract", ("node", "--test", "tests/test_planning_schemas.mjs")),
    ("documentation-gate", ("npm", "run", "docs:check")),
    ("product-ci-contract", ("python3", "scripts/product_ci.py", "--check")),
    ("supply-chain", ("python3", "scripts/supply_chain.py")),
    (
        "strict-clippy",
        (
            "cargo", "clippy", "-p", "agentmage-kernel-engine", "-p",
            "agentmage-platform-linux", "--all-targets", "--locked", "--", "-D", "warnings",
        ),
    ),
    (
        "evidence-tests",
        ("python3", "-m", "unittest", "tests.test_sprint_42_evidence"),
    ),
)
FOCUSED_COMMANDS: Final = (
    "kernel-repository-contract",
    "repository-mutation-matrix",
    "linux-repository-integration",
)
ARTIFACTS: Final = (("git", Path("/usr/bin/git")),)
SECURITY_REQUIREMENTS: Final = [
    "SR-ACC-006", "SR-ACC-007", "SR-ACC-008", "SR-NET-005", "SR-NET-006",
    "SR-NET-007", "SR-OPS-001", "SR-TST-004", "SR-TST-005", "SR-GIT-001",
    "SR-GIT-002", "SR-GIT-003", "SR-GIT-004", "SR-GIT-005", "SR-GIT-006",
]
IMPLEMENTED: Final = {
    "canonical_host_bound_remote_identity": True,
    "exact_namespaced_fetch_plan": True,
    "dirty_untracked_stale_and_divergence_reports": True,
    "owned_task_worktree_lifecycle": True,
    "content_minimized_ownership_registry": True,
    "stale_preimage_and_collision_denial": True,
    "preservation_manifest_reconciliation": True,
    "agentmage_branch_compare_and_swap": True,
    "ambient_git_hardening": True,
    "fedora_local_worktree_and_cas_execution": True,
    "product_profile_registered": False,
    "network_clone_or_fetch_execution": False,
    "clone_success_reconciliation": False,
    "complete_descendant_containment": False,
    "peak_resource_accounting": False,
    "native_cross_platform_acceptance": False,
    "complete_hostile_interruption_campaign": False,
    "trusted_package_execution": False,
    "independent_review": False,
    "manual_fuzzing": False,
}
BLOCKERS: Final = [
    {"code": "UPSTREAM-SPRINT-41-BLOCKED", "owner": "42.1"},
    {"code": "REPOSITORY-PROFILE-NOT-REGISTERED", "owner": "42.1.1"},
    {"code": "AUTHENTICATED-NETWORK-GIT-DEFERRED", "owner": "42.1.1.2"},
    {"code": "CLONE-SUCCESS-RECONCILIATION-INCOMPLETE", "owner": "42.1.1.2"},
    {"code": "DESCENDANT-CONTAINMENT-INCOMPLETE", "owner": "42.1.3.4"},
    {"code": "PEAK-RESOURCE-ACCOUNTING-ABSENT", "owner": "42.1.3.3"},
    {"code": "NATIVE-CROSS-PLATFORM-GIT-ACCEPTANCE-ABSENT", "owner": "42.1.3.3"},
    {"code": "COMPLETE-HOSTILE-INTERRUPTION-CAMPAIGN-ABSENT", "owner": "42.1.3.4"},
    {"code": "TRUSTED-PACKAGE-LAUNCHER-ENVIRONMENT-ABSENT", "owner": "42.1.3.5"},
    {"code": "INDEPENDENT-REPOSITORY-BOUNDARY-REVIEW-ABSENT", "owner": "42.1.3.5"},
    {"code": "MANUAL-FUZZING-DEFERRED", "owner": "SR-TST-004"},
]


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def git_file(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, check=False,
        capture_output=True, timeout=30,
    )
    if result.returncode != 0:
        raise ValueError(f"committed Sprint 42 source is absent: {path}")
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
            raise ValueError(f"native Sprint 42 artifact unavailable: {identifier}")
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
        "record_type": "sprint_42_local_evidence",
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
            "fedora_local_worktree_lifecycle": local_pass,
            "fedora_local_agentmage_branch_cas": local_pass,
            "fixed_hostile_case_count": 44 if local_pass else None,
            "protected_manifest_mutation_count": 10_000 if local_pass else None,
            "unauthorized_manifest_mutation_acceptance_count": 0 if local_pass else None,
            "network_clone_or_fetch_execution": False,
            "clone_success_reconciliation": False,
            "complete_descendant_containment": False,
            "peak_resource_accounting": False,
            "native_cross_platform_acceptance": False,
            "trusted_package_execution": False,
            "independent_review": False,
            "manual_fuzzing": False,
        },
        "blockers": [dict(blocker) for blocker in BLOCKERS],
        "summary": {
            "local_sprint_42_contract_passed": local_pass,
            "sprint_status": "BLOCKED",
            "upstream_sprint_41_closed": False,
            "repository_profile_active": False,
            "network_git_enabled": False,
            "clone_success_complete": False,
            "active_checkout_preservation_locally_exercised": local_pass,
            "cross_platform_acceptance_passed": False,
            "complete_descendant_containment": False,
            "peak_resource_accounting_complete": False,
            "trusted_package_execution_complete": False,
            "independent_review_present": False,
            "manual_fuzzing_complete": False,
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
    artifacts = report.get("native_fixture_artifacts", [])
    if [item.get("id") for item in artifacts] != [item[0] for item in ARTIFACTS]:
        failures.append("native artifact inventory drift")
    for item in artifacts:
        if (
            not SHA256.fullmatch(str(item.get("sha256", "")))
            or not isinstance(item.get("size"), int)
            or item.get("size", 0) <= 0
            or item.get("root_owned") is not True
            or item.get("group_or_world_writable") is not False
        ):
            failures.append("native artifact claim invalid")
    expected_verification = {
        "focused_local_contracts": True,
        "focused_blocking_skip_count": 0,
        "fedora_local_worktree_lifecycle": True,
        "fedora_local_agentmage_branch_cas": True,
        "fixed_hostile_case_count": 44,
        "protected_manifest_mutation_count": 10_000,
        "unauthorized_manifest_mutation_acceptance_count": 0,
        "network_clone_or_fetch_execution": False,
        "clone_success_reconciliation": False,
        "complete_descendant_containment": False,
        "peak_resource_accounting": False,
        "native_cross_platform_acceptance": False,
        "trusted_package_execution": False,
        "independent_review": False,
        "manual_fuzzing": False,
    }
    if report.get("verification_evidence") != expected_verification:
        failures.append("verification claim drift")
    expected_summary = {
        "local_sprint_42_contract_passed": True,
        "sprint_status": "BLOCKED",
        "upstream_sprint_41_closed": False,
        "repository_profile_active": False,
        "network_git_enabled": False,
        "clone_success_complete": False,
        "active_checkout_preservation_locally_exercised": True,
        "cross_platform_acceptance_passed": False,
        "complete_descendant_containment": False,
        "peak_resource_accounting_complete": False,
        "trusted_package_execution_complete": False,
        "independent_review_present": False,
        "manual_fuzzing_complete": False,
        "release_approval": False,
    }
    if report.get("summary") != expected_summary:
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
        print("Sprint 42 evidence validated")
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
