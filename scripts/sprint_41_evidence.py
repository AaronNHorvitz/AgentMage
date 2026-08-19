#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 41 evidence."""

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
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-41/local-evidence-report.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
IGNORED_TESTS: Final = re.compile(
    rb"test result: (?:ok|FAILED)\. \d+ passed; \d+ failed; (\d+) ignored;"
)
SOURCE_PATHS: Final = (
    "kernel/engine/src/command_runner.rs",
    "kernel/engine/src/validation_result.rs",
    "platforms/linux/src/command_runner.rs",
    "shells/host/src/linux_coding_runtime.rs",
    "kernel/engine/tests/command_injection_corpus.rs",
    "schemas/runtime/command-preview.schema.json",
    "schemas/runtime/command-receipt.schema.json",
    "schemas/runtime/examples/command-preview.valid.json",
    "schemas/runtime/examples/command-receipt.valid.json",
    "docs/architecture/bounded-command-runner.md",
    "docs/verification/sprint-41-command-injection-corpus.json",
    "docs/verification/sprint-41-local-results.md",
    "scripts/sprint_41_evidence.py",
    "tests/test_planning_schemas.mjs",
    "tests/test_sprint_41_evidence.py",
)
COMMANDS: Final = (
    (
        "kernel-command-contract",
        ("cargo", "test", "-p", "agentmage-kernel-engine", "command_runner", "--locked"),
    ),
    (
        "command-injection-corpus",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine", "--test",
            "command_injection_corpus", "--locked",
        ),
    ),
    (
        "linux-command-manifest",
        ("cargo", "test", "-p", "agentmage-platform-linux", "manifest_", "--locked"),
    ),
    (
        "linux-live-literal-command",
        (
            "cargo", "test", "-p", "agentmage-platform-linux",
            "live_exact_command_runs_offline_with_literal_output", "--locked", "--",
            "--ignored",
        ),
    ),
    (
        "linux-live-termination",
        (
            "cargo", "test", "-p", "agentmage-platform-linux",
            "live_timeout_and_cancellation_terminate_the_process_unit", "--locked", "--",
            "--ignored",
        ),
    ),
    (
        "linux-live-hostile-descendants",
        (
            "cargo", "test", "-p", "agentmage-platform-linux",
            "live_hostile_descendant_process_tree_is_bounded_and_removed", "--locked", "--",
            "--ignored",
        ),
    ),
    (
        "linux-live-inherited-descriptors",
        (
            "cargo", "test", "-p", "agentmage-platform-linux",
            "live_inherited_descriptors_do_not_reach_the_command_guest", "--locked", "--",
            "--ignored",
        ),
    ),
    (
        "linux-live-worktree-marker",
        (
            "cargo", "test", "-p", "agentmage-platform-linux",
            "live_owned_worktree_is_descriptor_bound_at_the_guest_working_directory",
            "--locked", "--", "--ignored",
        ),
    ),
    (
        "linux-live-worktree-write-denied",
        (
            "cargo", "test", "-p", "agentmage-platform-linux",
            "live_owned_worktree_command_cannot_mutate_the_held_directory", "--locked", "--",
            "--ignored",
        ),
    ),
    (
        "linux-live-worktree-descriptors",
        (
            "cargo", "test", "-p", "agentmage-platform-linux",
            "live_owned_worktree_guest_sees_no_inherited_descriptors", "--locked", "--",
            "--ignored",
        ),
    ),
    (
        "linux-live-worktree-identity",
        (
            "cargo", "test", "-p", "agentmage-platform-linux",
            "live_changed_held_worktree_identity_fails_before_launch", "--locked", "--",
            "--ignored",
        ),
    ),
    (
        "linux-live-guest-environment",
        (
            "cargo", "test", "-p", "agentmage-platform-linux",
            "live_guest_environment_holds_only_the_sealed_template_variables", "--locked", "--",
            "--ignored",
        ),
    ),
    (
        "linux-live-second-program-denied",
        (
            "cargo", "test", "-p", "agentmage-platform-linux",
            "live_guest_cannot_execute_any_second_program", "--locked", "--", "--ignored",
        ),
    ),
    (
        "linux-live-root-enumeration",
        (
            "cargo", "test", "-p", "agentmage-platform-linux",
            "live_guest_filesystem_root_exposes_only_declared_mounts", "--locked", "--",
            "--ignored",
        ),
    ),
    (
        "linux-live-output-ceiling",
        (
            "cargo", "test", "-p", "agentmage-platform-linux",
            "live_output_beyond_the_declared_ceiling_is_truncated_and_still_hashed",
            "--locked", "--", "--ignored",
        ),
    ),
    (
        "linux-live-maximum-limits",
        (
            "cargo", "test", "-p", "agentmage-platform-linux",
            "live_maximum_limit_boundary_runs_with_exact_identity", "--locked", "--", "--ignored",
        ),
    ),
    (
        "linux-live-minimum-timeout",
        (
            "cargo", "test", "-p", "agentmage-platform-linux",
            "live_minimum_timeout_boundary_terminates_the_unit", "--locked", "--", "--ignored",
        ),
    ),
    (
        "linux-live-parent-crash",
        (
            "cargo", "test", "-p", "agentmage-platform-linux",
            "live_parent_crash_leaves_no_owned_unit_descendant_or_scratch", "--locked", "--",
            "--ignored",
        ),
    ),
    (
        "linux-live-parent-crash-decoy",
        (
            "cargo", "test", "-p", "agentmage-platform-linux",
            "live_parent_crash_never_adopts_a_concurrent_unit", "--locked", "--", "--ignored",
        ),
    ),
    (
        "linux-task-minimum",
        (
            "cargo", "test", "-p", "agentmage-platform-linux",
            "task_ceilings_below_the_linux_minimum_fail_before_launch", "--locked",
        ),
    ),
    (
        "linux-live-scratch-residue",
        (
            "cargo", "test", "-p", "agentmage-platform-linux",
            "live_empty_scratch_retains_no_residue_between_attempts", "--locked", "--",
            "--ignored",
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
        ("python3", "-m", "unittest", "tests.test_sprint_41_evidence"),
    ),
)
FOCUSED_COMMANDS: Final = (
    "kernel-command-contract",
    "command-injection-corpus",
    "linux-command-manifest",
    "linux-live-literal-command",
    "linux-live-termination",
    "linux-live-hostile-descendants",
    "linux-live-inherited-descriptors",
    "linux-live-worktree-marker",
    "linux-live-worktree-write-denied",
    "linux-live-worktree-descriptors",
    "linux-live-worktree-identity",
    "linux-live-guest-environment",
    "linux-live-second-program-denied",
    "linux-live-root-enumeration",
    "linux-live-output-ceiling",
    "linux-live-maximum-limits",
    "linux-live-minimum-timeout",
    "linux-live-parent-crash",
    "linux-live-parent-crash-decoy",
    "linux-task-minimum",
    "linux-live-scratch-residue",
)
ARTIFACTS: Final = (
    ("systemd-run", Path("/usr/bin/systemd-run")),
    ("systemctl", Path("/usr/bin/systemctl")),
    ("bubblewrap", Path("/usr/bin/bwrap")),
    ("printf-fixture", Path("/usr/bin/printf")),
    ("sleep-fixture", Path("/usr/bin/sleep")),
    ("openssl-fixture", Path("/usr/bin/openssl")),
    ("cat-fixture", Path("/usr/bin/cat")),
    ("ls-fixture", Path("/usr/bin/ls")),
    ("touch-fixture", Path("/usr/bin/touch")),
    ("printenv-fixture", Path("/usr/bin/printenv")),
    ("timeout-fixture", Path("/usr/bin/timeout")),
)
SECURITY_REQUIREMENTS: Final = [
    "SR-ACC-001", "SR-ACC-002", "SR-ACC-003", "SR-ACC-004", "SR-ACC-005",
    "SR-ACC-006", "SR-ACC-007", "SR-PLT-003", "SR-AI-005", "SR-AI-009",
    "SR-TST-004", "SR-TST-006",
]
IMPLEMENTED: Final = {
    "exact_command_spec_and_registry": True,
    "direct_process_without_shell": True,
    "exact_executable_argument_working_directory_environment_validation": True,
    "cleared_environment_and_noninteractive_execution": True,
    "bounded_streams_timeout_cancellation_and_cleanup": True,
    "exact_previews_and_terminal_receipts": True,
    "trusted_deterministic_read_only_fixtures": True,
    "unrestricted_shell_and_hidden_expansion_absent": True,
    "fedora_live_literal_timeout_and_cancellation_exercised": True,
    "production_command_profile_registered": False,
    "peak_cpu_memory_and_task_accounting_retained": True,
    "fedora_hostile_multiprocess_timeout_exercised": True,
    "inherited_descriptor_confinement_exercised": True,
    "owned_worktree_descriptor_binding_exercised": True,
    "changed_worktree_identity_fails_before_launch": True,
    "guest_environment_containment_exercised": True,
    "empty_scratch_residue_absent": True,
    "second_program_execution_unreachable": True,
    "guest_root_exposes_only_declared_mounts": True,
    "bounded_output_ceiling_exercised": True,
    "minimum_deadline_boundary_exercised": True,
    "minimum_executable_task_ceiling_enforced": True,
    "maximum_limit_boundary_exercised": True,
    "parent_crash_recovery_campaign_complete": True,
    "parent_crash_unit_ownership_proven": True,
    "complete_limit_boundary_campaign": False,
    "planted_host_configuration_campaign_complete": True,
    "hostile_descendant_tree_campaign_complete": False,
    "native_cross_platform_command_acceptance": False,
    "independent_command_boundary_review": False,
    "manual_fuzzing_complete": False,
}
BLOCKERS: Final = [
    {"code": "UPSTREAM-G-V0.3-BLOCKED", "owner": "41.1"},
    {"code": "COMMAND-PROFILE-NOT-PRODUCT-REGISTERED", "owner": "41.1.1"},
    {"code": "HOSTILE-DESCENDANT-CRASH-CAMPAIGN-ABSENT", "owner": "41.1.3.3"},
    {"code": "MULTI-LEVEL-HELPER-BINARY-UNAVAILABLE", "owner": "41.1.3.3"},
    {"code": "COMPLETE-LIMIT-BOUNDARY-CAMPAIGN-ABSENT", "owner": "41.1.3.4"},
    {"code": "NATIVE-CROSS-PLATFORM-COMMAND-ACCEPTANCE-ABSENT", "owner": "41.1.3.4"},
    {"code": "TRUSTED-PACKAGE-LAUNCHER-ENVIRONMENT-ABSENT", "owner": "41.1.3.5"},
    {"code": "INDEPENDENT-COMMAND-BOUNDARY-REVIEW-ABSENT", "owner": "41.1.3.5"},
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
        raise ValueError(f"committed Sprint 41 source is absent: {path}")
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
        "systemd": version("systemd-run", "--version"),
        "bubblewrap": version("bwrap", "--version"),
    }


def native_artifacts() -> list[dict[str, Any]]:
    records = []
    for identifier, path in ARTIFACTS:
        if not path.is_file() or path.is_symlink() or path.stat().st_size <= 0:
            raise ValueError(f"native Sprint 41 artifact unavailable: {identifier}")
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
        "record_type": "sprint_41_local_evidence",
        "source_revision": revision,
        "source_sha256": {path: digest(git_file(revision, path)) for path in SOURCE_PATHS},
        "environment": environment_manifest(),
        "commands": commands,
        "native_fixture_artifacts": artifacts,
        "security_requirement_ids": SECURITY_REQUIREMENTS,
        "implemented_contracts": IMPLEMENTED,
        "verification_evidence": {
            "focused_local_contracts": local_pass,
            "focused_blocking_skip_count": 0 if local_pass else None,
            "fedora_live_literal_execution": local_pass,
            "fedora_live_timeout_and_cancellation_cleanup": local_pass,
            "production_command_profile_registration": False,
            "peak_resource_accounting": local_pass,
            "fedora_hostile_multiprocess_timeout": local_pass,
            "inherited_descriptor_confinement": local_pass,
            "owned_worktree_descriptor_binding": local_pass,
            "guest_environment_containment": local_pass,
            "empty_scratch_residue_absent": local_pass,
            "second_program_execution_unreachable": local_pass,
            "guest_root_exposes_only_declared_mounts": local_pass,
            "bounded_output_ceiling": local_pass,
            "minimum_deadline_boundary": local_pass,
            "minimum_executable_task_ceiling": local_pass,
            "maximum_limit_boundary": local_pass,
            "parent_crash_recovery_campaign": local_pass,
            "parent_crash_unit_ownership_proven": local_pass,
            "complete_limit_boundary_campaign": False,
            "planted_host_configuration_campaign": local_pass,
            "hostile_descendant_crash_campaign": False,
            "native_cross_platform_command_acceptance": False,
            "trusted_package_launcher_environment": False,
            "independent_command_boundary_review": False,
            "manual_fuzzing": False,
        },
        "blockers": BLOCKERS,
        "summary": {
            "local_sprint_41_contract_passed": local_pass,
            "sprint_status": "BLOCKED",
            "upstream_g_v0_3_closed": False,
            "command_profile_active": False,
            "generic_shell_present": False,
            "network_access_enabled": False,
            "cross_platform_acceptance_passed": False,
            "peak_resource_accounting_complete": local_pass,
            "hostile_descendant_campaign_complete": False,
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
        "fedora_live_literal_execution": True,
        "fedora_live_timeout_and_cancellation_cleanup": True,
        "production_command_profile_registration": False,
        "peak_resource_accounting": True,
        "fedora_hostile_multiprocess_timeout": True,
        "inherited_descriptor_confinement": True,
        "owned_worktree_descriptor_binding": True,
        "guest_environment_containment": True,
        "empty_scratch_residue_absent": True,
        "second_program_execution_unreachable": True,
        "guest_root_exposes_only_declared_mounts": True,
        "bounded_output_ceiling": True,
        "minimum_deadline_boundary": True,
        "minimum_executable_task_ceiling": True,
        "maximum_limit_boundary": True,
        "parent_crash_recovery_campaign": True,
        "parent_crash_unit_ownership_proven": True,
        "complete_limit_boundary_campaign": False,
        "planted_host_configuration_campaign": True,
        "hostile_descendant_crash_campaign": False,
        "native_cross_platform_command_acceptance": False,
        "trusted_package_launcher_environment": False,
        "independent_command_boundary_review": False,
        "manual_fuzzing": False,
    }
    if report.get("verification_evidence") != expected_verification:
        failures.append("verification claim drift")
    expected_summary = {
        "local_sprint_41_contract_passed": True,
        "sprint_status": "BLOCKED",
        "upstream_g_v0_3_closed": False,
        "command_profile_active": False,
        "generic_shell_present": False,
        "network_access_enabled": False,
        "cross_platform_acceptance_passed": False,
        "peak_resource_accounting_complete": True,
        "hostile_descendant_campaign_complete": False,
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
        print("Sprint 41 evidence validated")
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
