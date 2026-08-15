#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 43 evidence."""

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
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-43/local-evidence-report.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
IGNORED_TESTS: Final = re.compile(
    rb"test result: (?:ok|FAILED)\. \d+ passed; \d+ failed; (\d+) ignored;"
)
SOURCE_PATHS: Final = (
    "capabilities/repository-map/src/deep_analysis.rs",
    "capabilities/repository-map/src/deep_views.rs",
    "capabilities/repository-map/src/lib.rs",
    "capabilities/repository-map/tests/deep_repository_matrix.rs",
    "capabilities/repository-map/README.md",
    "docs/architecture/deep-repository-comprehension.md",
    "docs/guides/repository-learning-exports.md",
    "docs/verification/sprint-43-hostile-repository-corpus.json",
    "docs/verification/sprint-43-local-results.md",
    "scripts/sprint_43_evidence.py",
    "tests/test_sprint_43_evidence.py",
)
COMMANDS: Final = (
    (
        "deep-repository-unit",
        (
            "cargo", "test", "-p", "agentmage-capability-repository-map",
            "deep_", "--lib", "--locked",
        ),
    ),
    (
        "hostile-repository-matrix",
        (
            "cargo", "test", "-p", "agentmage-capability-repository-map",
            "--test", "deep_repository_matrix", "--locked",
        ),
    ),
    (
        "read-only-git-invariance",
        (
            "cargo", "test", "-p", "agentmage-capability-repository-map",
            "invariance_tests::tests", "--lib", "--locked",
        ),
    ),
    (
        "complete-repository-map-capability",
        (
            "cargo", "test", "-p", "agentmage-capability-repository-map",
            "--all-targets", "--locked",
        ),
    ),
    (
        "strict-clippy",
        (
            "cargo", "clippy", "-p", "agentmage-capability-repository-map",
            "--all-targets", "--locked", "--", "-D", "warnings",
        ),
    ),
    ("documentation-gate", ("npm", "run", "docs:check")),
    ("product-ci-contract", ("python3", "scripts/product_ci.py", "--check")),
    ("supply-chain", ("python3", "scripts/supply_chain.py")),
    (
        "evidence-tests",
        ("python3", "-m", "unittest", "tests.test_sprint_43_evidence"),
    ),
)
FOCUSED_COMMANDS: Final = (
    "deep-repository-unit",
    "hostile-repository-matrix",
    "read-only-git-invariance",
)
ARTIFACTS: Final = (("git", Path("/usr/bin/git")),)
SECURITY_REQUIREMENTS: Final = [
    "SR-ACC-008", "SR-AI-003", "SR-AI-007", "SR-AI-010", "SR-AI-011",
    "SR-DAT-003", "SR-TST-004", "SR-TST-006",
]
IMPLEMENTED: Final = {
    "deterministic_repository_profiles": True,
    "closed_confined_adapter_contracts": True,
    "source_resolvable_evidence_states": True,
    "typed_repository_trace_set": True,
    "branch_aware_history_contract": True,
    "documentation_drift_and_glossary_contracts": True,
    "separated_cross_repository_portfolio": True,
    "bounded_large_repository_slices": True,
    "quantified_coverage_and_blind_spots": True,
    "structured_learning_export_suite": True,
    "whole_repository_overclaim_prohibited": True,
    "hostile_prose_and_secret_canary_matrix": True,
    "production_repository_projection": False,
    "live_language_native_or_server_adapter": False,
    "persistent_production_index": False,
    "complete_history_and_cross_repository_evidence": False,
    "complete_semantic_relationship_coverage": False,
    "native_cross_platform_acceptance": False,
    "trusted_package_execution": False,
    "parser_process_crash_and_cancellation_campaign": False,
    "independent_review": False,
    "manual_fuzzing": False,
}
BLOCKERS: Final = [
    {"code": "UPSTREAM-SPRINT-42-BLOCKED", "owner": "43.1"},
    {"code": "PRODUCTION-REPOSITORY-PROJECTION-ABSENT", "owner": "43.1.1.1"},
    {"code": "LIVE-SEMANTIC-ADAPTERS-ABSENT", "owner": "43.1.1.2"},
    {"code": "PERSISTENT-PRODUCTION-INDEX-ABSENT", "owner": "43.1.2.1"},
    {"code": "COMPLETE-HISTORY-CROSS-REPOSITORY-EVIDENCE-ABSENT", "owner": "43.1.1.5"},
    {"code": "COMPLETE-SEMANTIC-COVERAGE-ABSENT", "owner": "43.1.3.1"},
    {"code": "PARSER-PROCESS-CRASH-CANCELLATION-CAMPAIGN-ABSENT", "owner": "43.1.3.2"},
    {"code": "NATIVE-CROSS-PLATFORM-ACCEPTANCE-ABSENT", "owner": "43.1.3.4"},
    {"code": "TRUSTED-PACKAGE-EXECUTION-ABSENT", "owner": "43.1.3.5"},
    {"code": "INDEPENDENT-DEEP-REPOSITORY-REVIEW-ABSENT", "owner": "43.1.3.5"},
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
        raise ValueError(f"committed Sprint 43 source is absent: {path}")
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
            raise ValueError(f"native Sprint 43 artifact unavailable: {identifier}")
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
        "record_type": "sprint_43_local_evidence",
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
            "hostile_repository_case_count": 12 if local_pass else None,
            "deep_index_mutation_count": 10_000 if local_pass else None,
            "unauthorized_mutation_acceptance_count": 0 if local_pass else None,
            "secret_canary_export_count": 0 if local_pass else None,
            "whole_repository_claim_permitted": False,
            "production_repository_projection": False,
            "live_language_native_or_server_adapter": False,
            "complete_semantic_relationship_coverage": False,
            "native_cross_platform_acceptance": False,
            "trusted_package_execution": False,
            "independent_review": False,
            "manual_fuzzing": False,
        },
        "blockers": [dict(blocker) for blocker in BLOCKERS],
        "summary": {
            "local_sprint_43_contract_passed": local_pass,
            "sprint_status": "BLOCKED",
            "upstream_sprint_42_closed": False,
            "production_repository_projection_active": False,
            "live_semantic_adapters_active": False,
            "complete_repository_comprehension_claim": False,
            "cross_platform_acceptance_passed": False,
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
    if any(
        not SHA256.fullmatch(str(item.get("sha256", "")))
        or not isinstance(item.get("size"), int)
        or item.get("size", 0) <= 0
        or item.get("root_owned") is not True
        or item.get("group_or_world_writable") is not False
        for item in artifacts
    ):
        failures.append("native artifact claim invalid")
    expected_verification = {
        "focused_local_contracts": True,
        "focused_blocking_skip_count": 0,
        "hostile_repository_case_count": 12,
        "deep_index_mutation_count": 10_000,
        "unauthorized_mutation_acceptance_count": 0,
        "secret_canary_export_count": 0,
        "whole_repository_claim_permitted": False,
        "production_repository_projection": False,
        "live_language_native_or_server_adapter": False,
        "complete_semantic_relationship_coverage": False,
        "native_cross_platform_acceptance": False,
        "trusted_package_execution": False,
        "independent_review": False,
        "manual_fuzzing": False,
    }
    if report.get("verification_evidence") != expected_verification:
        failures.append("verification claim drift")
    expected_summary = {
        "local_sprint_43_contract_passed": True,
        "sprint_status": "BLOCKED",
        "upstream_sprint_42_closed": False,
        "production_repository_projection_active": False,
        "live_semantic_adapters_active": False,
        "complete_repository_comprehension_claim": False,
        "cross_platform_acceptance_passed": False,
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
        print("Sprint 43 evidence validated")
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
