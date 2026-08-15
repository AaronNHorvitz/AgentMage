#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 47 evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-47/local-evidence-report.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
IGNORED_TESTS: Final = re.compile(
    rb"test result: (?:ok|FAILED)\. \d+ passed; \d+ failed; (\d+) ignored;"
)
SOURCE_PATHS: Final = (
    "kernel/engine/src/review_packet.rs",
    "kernel/engine/src/local_commit.rs",
    "kernel/engine/src/repository_safety.rs",
    "kernel/engine/src/lib.rs",
    "platforms/linux/src/local_commit.rs",
    "platforms/linux/src/repository_safety.rs",
    "platforms/linux/src/lib.rs",
    "schemas/runtime/local-review-packet.schema.json",
    "schemas/runtime/examples/local-review-packet.valid.json",
    "schemas/runtime/logical-commit-plan.schema.json",
    "schemas/runtime/examples/logical-commit-plan.valid.json",
    "schemas/runtime/pinned-commit-signer.schema.json",
    "schemas/runtime/examples/pinned-commit-signer.valid.json",
    "schemas/runtime/candidate-tree-plan.schema.json",
    "schemas/runtime/examples/candidate-tree-plan.valid.json",
    "schemas/runtime/candidate-tree-receipt.schema.json",
    "schemas/runtime/examples/candidate-tree-receipt.valid.json",
    "schemas/runtime/local-commit-plan.schema.json",
    "schemas/runtime/examples/local-commit-plan.valid.json",
    "schemas/runtime/manual-commit-approval-receipt.schema.json",
    "schemas/runtime/examples/manual-commit-approval-receipt.valid.json",
    "schemas/runtime/local-commit-receipt.schema.json",
    "schemas/runtime/examples/local-commit-receipt.valid.json",
    "scripts/validate_planning_schemas.mjs",
    "tests/test_planning_schemas.mjs",
    "docs/verification/sprint-47-security-corpus.json",
    "tests/test_sprint_47_security_corpus.py",
    "scripts/effect_boundary.py",
    "tests/test_effect_boundary.py",
    "docs/architecture/local-review-and-commit-boundary.md",
    "docs/guides/reviewing-and-creating-local-commits.md",
    "docs/security/repository-safety.md",
    "docs/verification/sprint-47-local-results.md",
    "README.md",
    "scripts/sprint_47_evidence.py",
    "tests/test_sprint_47_evidence.py",
)
COMMANDS: Final = (
    (
        "review-packet-unit",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine",
            "review_packet::tests", "--lib", "--locked",
        ),
    ),
    (
        "local-commit-kernel-unit",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine",
            "local_commit::tests", "--lib", "--locked",
        ),
    ),
    (
        "repository-safety-unit",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine",
            "repository_safety::tests", "--lib", "--locked",
        ),
    ),
    (
        "linux-local-commit-native",
        (
            "cargo", "test", "-p", "agentmage-platform-linux",
            "local_commit::tests", "--lib", "--locked",
        ),
    ),
    (
        "kernel-library",
        ("cargo", "test", "-p", "agentmage-kernel-engine", "--lib", "--locked"),
    ),
    (
        "linux-library",
        ("cargo", "test", "-p", "agentmage-platform-linux", "--lib", "--locked"),
    ),
    (
        "security-corpus",
        ("python3", "-m", "unittest", "tests.test_sprint_47_security_corpus"),
    ),
    ("runtime-schema-contract", ("node", "--test", "tests/test_planning_schemas.mjs")),
    (
        "kernel-strict-clippy",
        (
            "cargo", "clippy", "-p", "agentmage-kernel-engine",
            "--all-targets", "--locked", "--", "-D", "warnings",
        ),
    ),
    (
        "linux-strict-clippy",
        (
            "cargo", "clippy", "-p", "agentmage-platform-linux",
            "--all-targets", "--all-features", "--locked", "--", "-D", "warnings",
        ),
    ),
    ("documentation-gate", ("npm", "run", "docs:check")),
    ("product-ci-contract", ("python3", "scripts/product_ci.py", "--check")),
    ("supply-chain", ("python3", "scripts/supply_chain.py")),
    ("evidence-tests", ("python3", "-m", "unittest", "tests.test_sprint_47_evidence")),
)
FOCUSED_COMMANDS: Final = tuple(identifier for identifier, _ in COMMANDS[:4])
ARTIFACTS: Final = (
    ("trusted-git", Path("/usr/bin/git")),
    ("trusted-openpgp-signer", Path("/usr/bin/gpg")),
)
SECURITY_REQUIREMENTS: Final = [
    "SR-GOV-005", "SR-GOV-010", "SR-ACC-002", "SR-ACC-007",
    "SR-SUP-002", "SR-SUP-005", "SR-TST-010", "SR-TST-011",
    "SR-GIT-001", "SR-GIT-002", "SR-GIT-003", "SR-GIT-004", "SR-GIT-007",
]
IMPLEMENTED: Final = {
    "complete_review_packet_contract": True,
    "nine_review_modes": True,
    "duplicate_and_low_confidence_evidence": True,
    "six_logical_commit_purposes": True,
    "unrelated_change_exclusion": True,
    "external_pinned_signer_report": True,
    "agent_owned_temporary_index_contract": True,
    "exact_manual_commit_approval": True,
    "one_shot_git_commit_mediation": True,
    "native_linux_candidate_fixture": True,
    "native_linux_openpgp_signed_commit_fixture": True,
    "exact_commit_and_signature_verification": True,
    "compare_and_swap_owned_task_branch": True,
    "automatic_publication_and_destructive_git_denied": True,
    "closed_runtime_records": True,
    "production_review_commit_coordinator": False,
    "production_approved_signer": False,
    "protected_manual_approval_channel": False,
    "native_signer_process_tree_campaign": False,
    "native_cross_platform_acceptance": False,
    "trusted_package_execution": False,
    "independent_review": False,
    "manual_fuzzing": False,
}
BLOCKERS: Final = [
    {"code": "UPSTREAM-SPRINT-46-BLOCKED", "owner": "47.1"},
    {"code": "PRODUCT-REVIEW-COMMIT-COORDINATOR-ABSENT", "owner": "47.1.1"},
    {"code": "PRODUCTION-APPROVED-SIGNER-ABSENT", "owner": "47.1.1.6"},
    {"code": "PROTECTED-MANUAL-APPROVAL-CHANNEL-ABSENT", "owner": "47.1.1.7"},
    {"code": "NATIVE-SIGNER-PROCESS-TREE-CAMPAIGN-ABSENT", "owner": "47.1.3.4"},
    {"code": "NATIVE-CROSS-PLATFORM-ACCEPTANCE-ABSENT", "owner": "47.1.3.5"},
    {"code": "TRUSTED-INSTALLED-PACKAGE-EXECUTION-ABSENT", "owner": "47.1.3.5"},
    {"code": "INDEPENDENT-LOCAL-COMMIT-REVIEW-ABSENT", "owner": "47.1.3.5"},
    {"code": "MANUAL-FUZZING-DEFERRED", "owner": "SR-TST-011"},
]


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def file_digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def git_file(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, check=False,
        capture_output=True, timeout=30,
    )
    if result.returncode != 0:
        raise ValueError(f"committed Sprint 47 source is absent: {path}")
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
        "gpg": version("gpg", "--version"),
    }


def native_artifacts() -> list[dict[str, Any]]:
    records = []
    for identifier, configured_path in ARTIFACTS:
        path = configured_path.resolve(strict=True)
        stat = path.stat()
        if (
            not path.is_file()
            or configured_path.is_symlink()
            or stat.st_size <= 0
            or stat.st_uid != 0
            or stat.st_mode & 0o022
            or not os.access(path, os.X_OK)
        ):
            raise ValueError(f"native Sprint 47 artifact unavailable: {identifier}")
        records.append({
            "id": identifier,
            "name": path.name,
            "size": stat.st_size,
            "sha256": file_digest(path),
            "owner_is_root": True,
            "group_or_world_writable": False,
            "executable": True,
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
        "review_mode_count": 9 if local_pass else None,
        "commit_purpose_count": 6 if local_pass else None,
        "runtime_record_count": 8 if local_pass else None,
        "adversarial_corpus_case_count": 28 if local_pass else None,
        "native_candidate_tree_fixture_passed": local_pass,
        "native_signed_commit_fixture_passed": local_pass,
        "unauthorized_effect_acceptance_count": 0 if local_pass else None,
        "production_review_commit_coordinator": False,
        "production_approved_signer": False,
        "protected_manual_approval_channel": False,
        "native_signer_process_tree_campaign": False,
        "native_cross_platform_acceptance": False,
        "trusted_package_execution": False,
        "independent_review": False,
        "manual_fuzzing": False,
    }


def expected_summary(local_pass: bool = True) -> dict[str, Any]:
    return {
        "local_sprint_47_contract_passed": local_pass,
        "sprint_status": "BLOCKED",
        "upstream_sprint_46_closed": False,
        "production_review_commit_coordinator_active": False,
        "production_approved_signer_present": False,
        "protected_manual_approval_channel_present": False,
        "native_signer_process_tree_campaign_complete": False,
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
            item.get("owner_is_root") is True
            and item.get("group_or_world_writable") is False
            and item.get("executable") is True
            for item in artifacts
        )
    )
    return {
        "schema_version": 1,
        "record_type": "sprint_47_local_evidence",
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
        or item.get("owner_is_root") is not True
        or item.get("group_or_world_writable") is not False
        or item.get("executable") is not True
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
    if verify_ancestry and REVISION.fullmatch(revision) and not revision_is_ancestor(revision):
        failures.append("report source revision is not an ancestor of current HEAD")
    encoded = json.dumps(report, sort_keys=True).lower()
    for prohibited in (
        "credential_value", "secret_value", "private_key", "raw_output",
        "stdout_content", "stderr_content", "remote_url", "repository_path",
        "keyring_path", "key_fingerprint",
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
    print("Sprint 47 local evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
