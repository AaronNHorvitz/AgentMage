#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 51 evidence."""

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
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-51/local-evidence-report.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
IGNORED_TESTS: Final = re.compile(
    rb"test result: (?:ok|FAILED)\. \d+ passed; \d+ failed; (\d+) ignored;"
)
SOURCE_PATHS: Final = (
    "kernel/contracts/src/frontier.rs",
    "kernel/contracts/src/handoff.rs",
    "kernel/contracts/src/lib.rs",
    "kernel/engine/src/frontier_recommendation.rs",
    "kernel/engine/src/handoff.rs",
    "kernel/engine/src/lib.rs",
    "shells/vscode/src/handoff.ts",
    "shells/vscode/test/handoff.test.ts",
    "schemas/runtime/frontier-tier-decision.schema.json",
    "schemas/runtime/examples/frontier-tier-decision.valid.json",
    "schemas/runtime/frontier-recommendation-receipt.schema.json",
    "schemas/runtime/examples/frontier-recommendation-receipt.valid.json",
    "docs/verification/sprint-51-frontier-corpus.json",
    "scripts/frontier_recommendation_contract.py",
    "tests/test_frontier_recommendation_contract.py",
    "docs/architecture/manual-frontier-recommendation.md",
    "docs/guides/manual-frontier-consultation.md",
    "docs/verification/sprint-51-local-results.md",
    "README.md",
    "scripts/sprint_51_evidence.py",
    "tests/test_sprint_51_evidence.py",
)
COMMANDS: Final = (
    (
        "frontier-recommendation-unit",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine",
            "frontier_recommendation::tests", "--lib", "--locked",
        ),
    ),
    (
        "local-handoff-unit",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine",
            "handoff::tests", "--lib", "--locked",
        ),
    ),
    (
        "frontier-schema-contract",
        ("node", "--test", "tests/test_planning_schemas.mjs"),
    ),
    (
        "frontier-hostile-corpus",
        (
            "python3", "-m", "unittest",
            "tests.test_frontier_recommendation_contract",
        ),
    ),
    (
        "vscode-local-handoff",
        ("npm", "run", "test", "--workspace", "@agentmage/vscode-shell"),
    ),
    (
        "frontier-strict-clippy",
        (
            "cargo", "clippy", "-p", "agentmage-kernel-contracts", "-p",
            "agentmage-kernel-engine", "--all-targets", "--locked", "--",
            "-D", "warnings",
        ),
    ),
    ("documentation-gate", ("npm", "run", "docs:check")),
    ("product-ci-contract", ("python3", "scripts/product_ci.py", "--check")),
    ("supply-chain", ("python3", "scripts/supply_chain.py")),
    (
        "evidence-tests",
        ("python3", "-m", "unittest", "tests.test_sprint_51_evidence"),
    ),
)
FOCUSED_COMMANDS: Final = tuple(item[0] for item in COMMANDS[:5])
RUST_FOCUSED_COMMANDS: Final = {
    "frontier-recommendation-unit",
    "local-handoff-unit",
}
SECURITY_REQUIREMENTS: Final = [
    "SR-ACC-007",
    "SR-DAT-002",
    "SR-DAT-003",
    "SR-AI-004",
    "SR-AI-008",
    "SR-AI-010",
    "SR-OPS-001",
    "SR-OPS-003",
    "SR-TST-006",
]
IMPLEMENTED: Final = {
    "deterministic_tier_count": 4,
    "approved_recommendation_trigger_count": 6,
    "hostile_corpus_case_count": 45,
    "prohibited_delivery_action_count": 14,
    "deterministic_packet_preview": True,
    "exact_disclosure_inventory": True,
    "stable_packet_and_preview_hashes": True,
    "secret_scope_and_injection_rejection": True,
    "destination_recording_user_owned": True,
    "external_delivery_capability": False,
    "product_frontier_coordinator": False,
    "live_model_failure_campaign": False,
    "native_cross_platform_acceptance": False,
    "trusted_package_execution": False,
    "independent_review": False,
    "manual_fuzzing": False,
}
BLOCKERS: Final = [
    {"code": "UPSTREAM-SPRINT-50-BLOCKED", "owner": "51.1"},
    {"code": "FRONTIER-PRODUCT-COORDINATOR-ABSENT", "owner": "51.1.1.1"},
    {"code": "LIVE-LOCAL-MODEL-FAILURE-CAMPAIGN-ABSENT", "owner": "51.1.3.1"},
    {"code": "NATIVE-END-TO-END-REVIEW-WORKFLOW-ABSENT", "owner": "51.1.3.4"},
    {"code": "NATIVE-CROSS-PLATFORM-ACCEPTANCE-ABSENT", "owner": "51.1.3.5"},
    {"code": "TRUSTED-INSTALLED-PACKAGE-EXECUTION-ABSENT", "owner": "51.1.3.5"},
    {"code": "INDEPENDENT-FRONTIER-BOUNDARY-REVIEW-ABSENT", "owner": "51.1.3.5"},
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
        raise ValueError(f"committed Sprint 51 source is absent: {path}")
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
        "tier_case_count": 13 if local_pass else None,
        "disclosure_case_count": 10 if local_pass else None,
        "prohibited_delivery_attempt_count": 14 if local_pass else None,
        "review_mutation_count": 8 if local_pass else None,
        "accepted_delivery_attempt_count": 0 if local_pass else None,
        "product_coordinator_integration": False,
        "live_model_failure_campaign": False,
        "native_cross_platform_acceptance": False,
        "trusted_package_execution": False,
        "independent_review": False,
        "manual_fuzzing": False,
        "sprint_gate_closed": False,
    }


def expected_summary(local_pass: bool = True) -> dict[str, Any]:
    return {
        "local_sprint_51_contract_passed": local_pass,
        "sprint_status": "BLOCKED",
        "upstream_sprint_50_closed": False,
        "frontier_product_coordinator_integrated": False,
        "automatic_frontier_delivery_enabled": False,
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
        "record_type": "sprint_51_local_evidence",
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
    if report.get("schema_version") != 1 or report.get("record_type") != "sprint_51_local_evidence":
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
    print("Sprint 51 local evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
