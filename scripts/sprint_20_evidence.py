#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 20 evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import subprocess
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-20/local-evidence-report.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
SOURCE_PATHS: Final = (
    "TASKS.md",
    "docs/architecture/evidence-state-assignment.md",
    "docs/architecture/kernel-contract-reference.md",
    "docs/architecture/reusable-runtime-coordinator.md",
    "docs/verification/sprint-20-local-results.md",
    "kernel/contracts/src/claim.rs",
    "kernel/contracts/src/lib.rs",
    "kernel/contracts/src/runtime_run.rs",
    "kernel/contracts/src/serialization.rs",
    "kernel/engine/src/authority.rs",
    "kernel/engine/src/evidence_state.rs",
    "kernel/engine/src/lib.rs",
    "kernel/engine/src/runtime_answer.rs",
    "kernel/engine/src/runtime_coordinator.rs",
    "kernel/engine/src/runtime_loop.rs",
    "kernel/engine/src/runtime_loop_tests.rs",
    "scripts/sprint_20_evidence.py",
    "shells/host/src/cli.rs",
    "shells/vscode/src/runtime_transport.ts",
    "shells/vscode/test/provider.test.ts",
    "shells/vscode/test/runtime_transport.test.ts",
    "tests/test_sprint_20_evidence.py",
)
COMMANDS: Final = (
    (
        "evidence-state-tests",
        (
            "cargo",
            "test",
            "-p",
            "agentmage-kernel-engine",
            "evidence_state::tests",
            "--locked",
        ),
    ),
    (
        "runtime-answer-integration-tests",
        (
            "cargo",
            "test",
            "-p",
            "agentmage-kernel-engine",
            "story_20_successful_answers_require_exact_kernel_assigned_inference_provenance",
            "--locked",
        ),
    ),
    (
        "vscode-answer-evidence-tests",
        ("npm", "--prefix", "shells/vscode", "test"),
    ),
    (
        "evidence-state-clippy",
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
    (
        "evidence-script-tests",
        ("python3", "-m", "unittest", "tests.test_sprint_20_evidence"),
    ),
    ("effect-boundary", ("python3", "scripts/effect_boundary.py")),
    ("strict-local-source", ("python3", "scripts/strict_local_source_audit.py")),
    ("supply-chain", ("python3", "scripts/supply_chain.py")),
)
SECURITY_REQUIREMENTS: Final = [
    "SR-AI-003",
    "SR-AI-007",
    "SR-AI-010",
    "SR-AI-011",
    "SR-OPS-001",
    "SR-OPS-002",
    "SR-OPS-003",
    "SR-OPS-004",
    "SR-OPS-005",
    "SR-TST-010",
]
BLOCKERS: Final = [
    {"code": "LIVE-CITATION-RESOLUTION-NOT-IMPLEMENTED", "owner": "20.1.3.4"},
    {"code": "DURABLE-RECEIPT-CHAIN-NOT-IMPLEMENTED", "owner": "20.1.3.4"},
    {"code": "KEYED-INTEGRITY-ANCHOR-NOT-IMPLEMENTED", "owner": "20.1.3.4"},
    {"code": "CLOCK-ANOMALY-EVIDENCE-NOT-IMPLEMENTED", "owner": "20.1.3.4"},
    {"code": "INDEPENDENT-SPRINT-20-REVIEW-NOT-RETAINED", "owner": "20.1.3.3"},
]
IMPLEMENTED_CONTRACTS: Final = {
    "material_claim_state_count": 4,
    "unknown_blocked_reason_count": 8,
    "observed_requires_authorized_observe_receipt": True,
    "observed_requires_exact_source_identity": True,
    "derived_requires_registered_method": True,
    "derived_requires_observed_inputs": True,
    "inferred_requires_citations": True,
    "inferred_requires_exact_model_runtime_manifest": True,
    "successful_answer_requires_evidence_assignment": True,
    "successful_answer_assignment_state": "inferred",
    "successful_answer_assignment_granularity": "complete_rendered_answer",
    "model_confidence_is_evidence_state": False,
    "assignment_authority": False,
    "filesystem_authority": False,
    "process_authority": False,
    "network_authority": False,
    "completion_authority": False,
}


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def git_file(revision: str, relative: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{relative}"],
        cwd=ROOT,
        check=False,
        capture_output=True,
        timeout=30,
    )
    if result.returncode != 0:
        raise ValueError(f"committed Sprint 20 source is absent: {relative}")
    return result.stdout


def run_command(argv: tuple[str, ...], timeout: int = 900) -> tuple[int, bytes]:
    executable = shutil.which(argv[0])
    if executable is None:
        return 127, b"executable-unavailable"
    result = subprocess.run(
        (executable, *argv[1:]),
        cwd=ROOT,
        check=False,
        capture_output=True,
        timeout=timeout,
    )
    return result.returncode, result.stdout + result.stderr


def run_commands() -> list[dict[str, Any]]:
    records = []
    for identifier, argv in COMMANDS:
        exit_code, output = run_command(argv)
        records.append(
            {
                "id": identifier,
                "argv": list(argv),
                "exit_code": exit_code,
                "output_sha256": sha256_bytes(output),
            }
        )
    return records


def build_report(source_revision: str, commands: list[dict[str, Any]]) -> dict[str, Any]:
    local_pass = all(command["exit_code"] == 0 for command in commands)
    return {
        "schema_version": 1,
        "record_type": "sprint_20_local_evidence",
        "source_revision": source_revision,
        "source_sha256": {
            path: sha256_bytes(git_file(source_revision, path)) for path in SOURCE_PATHS
        },
        "commands": commands,
        "security_requirement_ids": SECURITY_REQUIREMENTS,
        "implemented_contracts": IMPLEMENTED_CONTRACTS,
        "verification_evidence": {
            "positive_state_matrix": local_pass,
            "invalid_prohibited_boundary_matrix": local_pass,
            "dependency_failure_and_cancellation_matrix": local_pass,
            "authority_side_effect_absence": local_pass,
            "model_confidence_injection_rejected": local_pass,
            "production_answer_assignment_integration": local_pass,
            "receipt_chain_verification": False,
            "citation_resolver_output": False,
            "independent_review": False,
        },
        "blockers": BLOCKERS,
        "summary": {
            "local_contract_passed": local_pass,
            "sprint_status": "BLOCKED",
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
    if report.get("blockers") != BLOCKERS:
        failures.append("blocker drift")
    if report.get("implemented_contracts") != IMPLEMENTED_CONTRACTS:
        failures.append("implemented-contract inventory drift")
    commands = report.get("commands", [])
    if [item.get("id") for item in commands] != [item[0] for item in COMMANDS]:
        failures.append("command inventory drift")
    if any(
        item.get("exit_code") != 0
        or not SHA256.fullmatch(str(item.get("output_sha256", "")))
        for item in commands
    ):
        failures.append("command result invalid")
    expected_summary = {
        "local_contract_passed": True,
        "sprint_status": "BLOCKED",
        "release_approval": False,
    }
    if report.get("summary") != expected_summary:
        failures.append("summary overclaim or local failure")
    verification = report.get("verification_evidence", {})
    for field in (
        "receipt_chain_verification",
        "citation_resolver_output",
        "independent_review",
    ):
        if verification.get(field) is not False:
            failures.append(f"verification overclaim: {field}")
    for field in (
        "assignment_authority",
        "filesystem_authority",
        "process_authority",
        "network_authority",
        "completion_authority",
        "model_confidence_is_evidence_state",
    ):
        if report.get("implemented_contracts", {}).get(field) is not False:
            failures.append(f"authority or confidence overclaim: {field}")
    source_hashes = report.get("source_sha256", {})
    if set(source_hashes) != set(SOURCE_PATHS):
        failures.append("source inventory drift")
    elif verify_current and REVISION.fullmatch(revision):
        for path in SOURCE_PATHS:
            digest = str(source_hashes.get(path, ""))
            if not SHA256.fullmatch(digest) or digest != sha256_bytes(
                git_file(revision, path)
            ):
                failures.append(f"source digest drift: {path}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    args = parser.parse_args()
    if args.write:
        revision = subprocess.run(
            ["git", "rev-parse", args.source_revision],
            cwd=ROOT,
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
        report = build_report(revision, run_commands())
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
    else:
        report = json.loads(OUTPUT.read_text(encoding="utf-8"))
    failures = validate_report(report)
    if failures:
        for failure in failures:
            print(failure)
        return 1
    print(json.dumps(report["summary"], sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
