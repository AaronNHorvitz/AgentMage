#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 50 evidence."""

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
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-50/local-evidence-report.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
IGNORED_TESTS: Final = re.compile(
    rb"test result: (?:ok|FAILED)\. \d+ passed; \d+ failed; (\d+) ignored;"
)
SOURCE_PATHS: Final = (
    "capabilities/knowledge/src/coding_skills.rs",
    "capabilities/knowledge/src/lib.rs",
    "capabilities/knowledge/examples/coding_skill_pack.rs",
    "capabilities/knowledge/tests/sprint_50_coding_corpus.rs",
    "artifacts/sprints/sprint-50/coding-skill-pack.json",
    "docs/verification/sprint-50-coding-corpus.json",
    "scripts/coding_skill_contract.py",
    "tests/test_coding_skill_contract.py",
    "docs/architecture/coding-skills-and-release-boundary.md",
    "docs/guides/bounded-coding-workflows.md",
    "docs/guides/skills.md",
    "release/v0.4-coding-pack-manifest.json",
    "scripts/v0_4_coding_release_gate.py",
    "tests/test_v0_4_coding_release_gate.py",
    "artifacts/sprints/sprint-50/v0.4-release-readiness.json",
    "docs/release/v0.4-coding-acceptance-and-recovery-bundle.md",
    "docs/release/v0.4-capability-matrix.md",
    "docs/release/release-notes-v0.4.0-draft.md",
    "docs/verification/sprint-50-local-results.md",
    "shells/host/src/coding_client.rs",
    "shells/host/src/coding_harness.rs",
    "shells/host/src/runtime_parity_tests.rs",
    "scripts/sprint_50_evidence.py",
    "tests/test_sprint_50_evidence.py",
)
COMMANDS: Final = (
    (
        "coding-skill-unit",
        (
            "cargo", "test", "-p", "agentmage-capability-knowledge",
            "coding_skills::tests", "--lib", "--locked",
        ),
    ),
    (
        "coding-skill-corpus",
        (
            "cargo", "test", "-p", "agentmage-capability-knowledge",
            "--test", "sprint_50_coding_corpus", "--locked",
        ),
    ),
    (
        "coding-skill-artifact",
        ("python3", "-m", "unittest", "tests.test_coding_skill_contract"),
    ),
    (
        "repository-safety",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine",
            "repository_safety::tests", "--lib", "--locked",
        ),
    ),
    (
        "instruction-provenance",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine",
            "instruction_provenance::tests", "--lib", "--locked",
        ),
    ),
    (
        "write-recovery",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine",
            "write_transaction::tests", "--lib", "--locked",
        ),
    ),
    (
        "trusted-validation",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine",
            "validation_result::tests", "--lib", "--locked",
        ),
    ),
    (
        "review-accounting",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine",
            "review_packet::tests", "--lib", "--locked",
        ),
    ),
    (
        "structured-edits",
        (
            "cargo", "test", "-p", "agentmage-capability-repository-map",
            "structured_edit::tests", "--lib", "--locked",
        ),
    ),
    (
        "thin-cli",
        (
            "cargo", "test", "-p", "agentmage-host", "cli::tests", "--lib",
            "--locked",
        ),
    ),
    (
        "shared-runtime-parity",
        (
            "cargo", "test", "-p", "agentmage-host",
            "story_50_2_read_only_and_coding_packets_are_equal_across_all_three_callers",
            "--lib", "--locked",
        ),
    ),
    (
        "v0.4-release-gate",
        ("python3", "-m", "unittest", "tests.test_v0_4_coding_release_gate"),
    ),
    (
        "coding-strict-clippy",
        (
            "cargo", "clippy", "-p", "agentmage-capability-knowledge", "-p",
            "agentmage-capability-repository-map", "-p", "agentmage-kernel-engine",
            "-p", "agentmage-host", "--all-targets", "--locked", "--", "-D",
            "warnings",
        ),
    ),
    ("documentation-gate", ("npm", "run", "docs:check")),
    ("product-ci-contract", ("python3", "scripts/product_ci.py", "--check")),
    ("supply-chain", ("python3", "scripts/supply_chain.py")),
    (
        "evidence-tests",
        ("python3", "-m", "unittest", "tests.test_sprint_50_evidence"),
    ),
)
FOCUSED_COMMANDS: Final = tuple(item[0] for item in COMMANDS[:12])
RUST_FOCUSED_COMMANDS: Final = {
    "coding-skill-unit",
    "coding-skill-corpus",
    "repository-safety",
    "instruction-provenance",
    "write-recovery",
    "trusted-validation",
    "review-accounting",
    "structured-edits",
    "thin-cli",
    "shared-runtime-parity",
}
SECURITY_REQUIREMENTS: Final = [
    "SR-ACC-002", "SR-ACC-003", "SR-ACC-004", "SR-ACC-005",
    "SR-ACC-006", "SR-ACC-007", "SR-ACC-008", "SR-SUP-003",
    "SR-SUP-009", "SR-AI-005", "SR-OPS-001", "SR-OPS-002",
    "SR-OPS-003", "SR-OPS-004", "SR-OPS-005", "SR-OPS-006",
    "SR-OPS-007", "SR-OPS-008", "SR-OPS-009", "SR-OPS-010",
    "SR-TST-002", "SR-TST-005", "SR-TST-006", "SR-TST-011",
    "SR-CIV-006", "SR-CIV-007", "SR-CIV-008", "SR-CIV-009",
]
IMPLEMENTED: Final = {
    "declarative_coding_skill_count": 9,
    "definition_attack_case_count": 54,
    "prohibited_operation_attempt_count": 117,
    "fictional_workflow_combination_count": 56,
    "repository_failure_scenario_count": 6,
    "data_only_skill_authority": False,
    "coding_product_coordinator_implemented": True,
    "authenticated_chat_cli_workflow": False,
    "source_level_cross_interface_parity": True,
    "automatic_publication": False,
    "admitted_live_model": False,
    "native_cross_platform_acceptance": False,
    "trusted_package_execution": False,
    "independent_review": False,
    "manual_fuzzing": False,
}
BLOCKERS: Final = [
    {"code": "UPSTREAM-SPRINTS-41-THROUGH-49-BLOCKED", "owner": "50.1"},
    {"code": "AUTHENTICATED-CHAT-CLI-WORKFLOW-ABSENT", "owner": "50.1.1.5"},
    {"code": "WRITE-COMMAND-COMMIT-PROFILES-UNREGISTERED", "owner": "50.1.3.2"},
    {"code": "ADMITTED-LIVE-MODEL-PROFILE-ABSENT", "owner": "50.1.3.2"},
    {"code": "NATIVE-CROSS-PLATFORM-ACCEPTANCE-ABSENT", "owner": "50.1.3.4"},
    {"code": "LIFECYCLE-ACCESSIBILITY-RECOVERY-CAMPAIGN-ABSENT", "owner": "50.1.3.4"},
    {"code": "TRUSTED-INSTALLED-PACKAGE-EXECUTION-ABSENT", "owner": "50.1.3.5"},
    {"code": "INDEPENDENT-V0.4-REVIEW-ABSENT", "owner": "50.1.3.5"},
    {"code": "MANUAL-FUZZING-DEFERRED", "owner": "SR-TST-006"},
]


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def git_file(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, check=False,
        capture_output=True, timeout=30,
    )
    if result.returncode != 0:
        raise ValueError(f"committed Sprint 50 source is absent: {path}")
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
        if identifier in RUST_FOCUSED_COMMANDS:
            matches = IGNORED_TESTS.findall(output)
            blocking_skip_count = sum(int(value) for value in matches) if matches else -1
        elif identifier in FOCUSED_COMMANDS:
            blocking_skip_count = 0 if code == 0 else None
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
        "declarative_coding_skill_count": 9 if local_pass else None,
        "expanded_corpus_case_count": 233 if local_pass else None,
        "accepted_autonomous_operation_count": 0 if local_pass else None,
        "integrated_fictional_workflow_campaign": False,
        "shared_coding_coordinator_contract": local_pass,
        "source_level_three_client_parity": local_pass,
        "native_chat_cli_parity": False,
        "admitted_live_model": False,
        "native_cross_platform_acceptance": False,
        "trusted_package_execution": False,
        "independent_review": False,
        "manual_fuzzing": False,
        "gate_v0_4_closed": False,
    }


def expected_summary(local_pass: bool = True) -> dict[str, Any]:
    return {
        "local_sprint_50_contract_passed": local_pass,
        "sprint_status": "BLOCKED",
        "upstream_sprints_41_through_49_closed": False,
        "coding_product_coordinator_integrated": False,
        "chat_cli_parity_complete": False,
        "automatic_publication_enabled": False,
        "cross_platform_acceptance_passed": False,
        "trusted_package_execution_complete": False,
        "independent_review_present": False,
        "manual_fuzzing_complete": False,
        "release_approval": False,
    }


def build_report(revision: str, commands: list[dict[str, Any]]) -> dict[str, Any]:
    focused = [item for item in commands if item["id"] in FOCUSED_COMMANDS]
    local_pass = (
        all(item["exit_code"] == 0 for item in commands)
        and len(focused) == len(FOCUSED_COMMANDS)
        and all(item.get("blocking_skip_count") == 0 for item in focused)
    )
    return {
        "schema_version": 1,
        "record_type": "sprint_50_local_evidence",
        "source_revision": revision,
        "source_sha256": {
            path: digest(git_file(revision, path)) for path in SOURCE_PATHS
        },
        "environment": environment_manifest(),
        "commands": commands,
        "security_requirement_ids": list(SECURITY_REQUIREMENTS),
        "implemented_contracts": dict(IMPLEMENTED),
        "verification_evidence": expected_verification(local_pass),
        "blockers": [dict(blocker) for blocker in BLOCKERS],
        "summary": expected_summary(local_pass),
    }


def revision_is_ancestor(revision: str) -> bool:
    result = subprocess.run(
        ["git", "merge-base", "--is-ancestor", revision, "HEAD"],
        cwd=ROOT, check=False, capture_output=True, timeout=30,
    )
    return result.returncode == 0


def validate_report(report: dict[str, Any], verify_ancestry: bool = True) -> list[str]:
    failures: list[str] = []
    revision = str(report.get("source_revision", ""))
    if not REVISION.fullmatch(revision):
        failures.append("source revision invalid")
    if report.get("schema_version") != 1 or report.get("record_type") != "sprint_50_local_evidence":
        failures.append("report identity invalid")
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
    if verify_ancestry and REVISION.fullmatch(revision) and not revision_is_ancestor(revision):
        failures.append("report source revision is not an ancestor of current HEAD")
    encoded = json.dumps(report, sort_keys=True).lower()
    for prohibited in (
        "credential_value", "secret_value", "private_key", "raw_output",
        "raw_result", "prompt_text", "remote_url", "repository_path", "access_token",
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
        report = build_report(revision, run_commands())
        failures = validate_report(report, verify_ancestry=False)
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
    print("Sprint 50 local evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
