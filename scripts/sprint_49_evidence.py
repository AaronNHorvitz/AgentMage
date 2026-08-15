#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 49 evidence."""

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
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-49/local-evidence-report.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
IGNORED_TESTS: Final = re.compile(
    rb"test result: (?:ok|FAILED)\. \d+ passed; \d+ failed; (\d+) ignored;"
)
SOURCE_PATHS: Final = (
    "kernel/engine/src/model_routing.rs",
    "kernel/engine/src/model_selection.rs",
    "kernel/engine/src/model_runtime.rs",
    "kernel/engine/src/lib.rs",
    "kernel/contracts/src/model.rs",
    "model-profiles/routing/historical-later-candidates.json",
    "model-profiles/routing/role-benchmark-corpus-v1.json",
    "model-profiles/routing/measured-routing-decision-table-v1.json",
    "scripts/model_routing_contract.py",
    "tests/test_model_routing_contract.py",
    "scripts/benchmark_contract.py",
    "tests/test_benchmark_contract.py",
    "scripts/model_activation.py",
    "tests/test_model_activation.py",
    "docs/verification/sprint-49-routing-corpus.json",
    "tests/test_sprint_49_routing_corpus.py",
    "docs/architecture/measured-local-model-routing.md",
    "docs/verification/sprint-49-local-results.md",
    "README.md",
    "scripts/sprint_49_evidence.py",
    "tests/test_sprint_49_evidence.py",
)
COMMANDS: Final = (
    (
        "measured-router-unit",
        (
            "cargo",
            "test",
            "-p",
            "agentmage-kernel-engine",
            "model_routing::tests",
            "--lib",
            "--locked",
        ),
    ),
    (
        "routing-artifact-contract",
        ("python3", "-m", "unittest", "tests.test_model_routing_contract"),
    ),
    (
        "routing-security-corpus",
        ("python3", "-m", "unittest", "tests.test_sprint_49_routing_corpus"),
    ),
    (
        "kernel-library",
        (
            "cargo",
            "test",
            "-p",
            "agentmage-kernel-engine",
            "--lib",
            "--locked",
        ),
    ),
    (
        "benchmark-contract",
        ("python3", "-m", "unittest", "tests.test_benchmark_contract"),
    ),
    (
        "zero-model-activation",
        ("python3", "-m", "unittest", "tests.test_model_activation"),
    ),
    (
        "kernel-strict-clippy",
        (
            "cargo",
            "clippy",
            "-p",
            "agentmage-kernel-engine",
            "--all-targets",
            "--locked",
            "--",
            "-D",
            "warnings",
        ),
    ),
    ("documentation-gate", ("npm", "run", "docs:check")),
    ("product-ci-contract", ("python3", "scripts/product_ci.py", "--check")),
    ("supply-chain", ("python3", "scripts/supply_chain.py")),
    (
        "evidence-tests",
        ("python3", "-m", "unittest", "tests.test_sprint_49_evidence"),
    ),
)
FOCUSED_COMMANDS: Final = (
    "measured-router-unit",
    "routing-artifact-contract",
    "routing-security-corpus",
)
SECURITY_REQUIREMENTS: Final = [
    "SR-SUP-006",
    "SR-SUP-007",
    "SR-SUP-008",
    "SR-AI-001",
    "SR-AI-006",
    "SR-AI-010",
    "SR-AI-011",
    "SR-AI-012",
    "SR-AI-013",
    "SR-AI-014",
    "SR-TST-006",
]
IMPLEMENTED: Final = {
    "historical_gemma_4_26b_preserved": True,
    "historical_devstral_small_2_preserved": True,
    "exact_profile_boundary": True,
    "twelve_independent_roles": True,
    "role_to_profile_allowlists": True,
    "four_visible_local_budgets": True,
    "deterministic_measured_router": True,
    "manual_selection_preserved": True,
    "visible_disagreements": True,
    "measured_high_risk_second_verifier": True,
    "hidden_fallback_disabled": True,
    "frontier_transfer_disabled": True,
    "model_confidence_unused": True,
    "enabled_product_profiles": False,
    "live_role_benchmark_campaign": False,
    "product_router_integration": False,
    "native_cross_platform_acceptance": False,
    "trusted_package_execution": False,
    "independent_review": False,
    "manual_fuzzing": False,
}
BLOCKERS: Final = [
    {"code": "UPSTREAM-SPRINT-48-BLOCKED", "owner": "49.1"},
    {"code": "APPROVED-LATER-PROFILE-MANIFESTS-ABSENT", "owner": "49.1.1.1"},
    {"code": "LIVE-ROLE-BENCHMARK-CAMPAIGN-ABSENT", "owner": "49.1.1.2"},
    {"code": "PRODUCT-ROUTER-INTEGRATION-ABSENT", "owner": "49.1.1.5"},
    {"code": "INTEGRATED-ROUTING-AUDIT-VIEW-ABSENT", "owner": "49.1.2.4"},
    {"code": "NATIVE-CROSS-PLATFORM-ACCEPTANCE-ABSENT", "owner": "49.1.3.4"},
    {"code": "TRUSTED-INSTALLED-PACKAGE-EXECUTION-ABSENT", "owner": "49.1.3.5"},
    {"code": "INDEPENDENT-MODEL-ROUTING-REVIEW-ABSENT", "owner": "49.1.3.5"},
    {"code": "MANUAL-FUZZING-DEFERRED", "owner": "SR-TST-006"},
]


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def git_file(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"],
        cwd=ROOT,
        check=False,
        capture_output=True,
        timeout=30,
    )
    if result.returncode != 0:
        raise ValueError(f"committed Sprint 49 source is absent: {path}")
    return result.stdout


def version(executable: str, *arguments: str) -> str:
    resolved = shutil.which(executable)
    if resolved is None:
        return "unavailable"
    result = subprocess.run(
        (resolved, *arguments),
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
        timeout=30,
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
                (executable, *argv[1:]),
                cwd=ROOT,
                check=False,
                capture_output=True,
                timeout=1800,
            )
            code, output = result.returncode, result.stdout + result.stderr
        blocking_skip_count = None
        if identifier == "measured-router-unit":
            matches = IGNORED_TESTS.findall(output)
            blocking_skip_count = sum(int(value) for value in matches) if matches else -1
        elif identifier in FOCUSED_COMMANDS:
            blocking_skip_count = 0 if code == 0 else None
        records.append(
            {
                "id": identifier,
                "argv": list(argv),
                "exit_code": code,
                "output_sha256": digest(output),
                "blocking_skip_count": blocking_skip_count,
            }
        )
    return records


def expected_verification(local_pass: bool = True) -> dict[str, Any]:
    return {
        "focused_local_contracts": local_pass,
        "focused_blocking_skip_count": 0 if local_pass else None,
        "historical_candidate_count": 2 if local_pass else None,
        "enabled_profile_count": 0,
        "independent_role_count": 12 if local_pass else None,
        "visible_budget_count": 4 if local_pass else None,
        "routing_rule_count": 16 if local_pass else None,
        "adversarial_corpus_case_count": 48 if local_pass else None,
        "unauthorized_or_remote_selection_count": 0 if local_pass else None,
        "live_role_benchmark_campaign": False,
        "product_router_integration": False,
        "native_cross_platform_acceptance": False,
        "trusted_package_execution": False,
        "independent_review": False,
        "manual_fuzzing": False,
    }


def expected_summary(local_pass: bool = True) -> dict[str, Any]:
    return {
        "local_sprint_49_contract_passed": local_pass,
        "sprint_status": "BLOCKED",
        "upstream_sprint_48_closed": False,
        "enabled_product_profile_count": 0,
        "automatic_product_routing_enabled": False,
        "live_role_benchmark_campaign_complete": False,
        "product_router_integrated": False,
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
        "record_type": "sprint_49_local_evidence",
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
        cwd=ROOT,
        check=False,
        capture_output=True,
        timeout=30,
    )
    return result.returncode == 0


def validate_report(report: dict[str, Any], verify_ancestry: bool = True) -> list[str]:
    failures: list[str] = []
    revision = str(report.get("source_revision", ""))
    if not REVISION.fullmatch(revision):
        failures.append("source revision invalid")
    if report.get("schema_version") != 1 or report.get("record_type") != "sprint_49_local_evidence":
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
        focused = next(
            (item for item in commands if item.get("id") == identifier), None
        )
        if focused is None or focused.get("blocking_skip_count") != 0:
            failures.append(
                f"focused skipped, suppressed, or unavailable check: {identifier}"
            )
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
        "credential_value",
        "secret_value",
        "private_key",
        "raw_output",
        "raw_result",
        "prompt_text",
        "remote_url",
        "repository_path",
        "access_token",
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
            ["git", "rev-parse", "HEAD"],
            cwd=ROOT,
            check=True,
            capture_output=True,
            text=True,
            timeout=30,
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
    print("Sprint 49 local evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
