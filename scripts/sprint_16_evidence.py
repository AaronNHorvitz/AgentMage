#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 16 evidence."""

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
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-16/local-evidence-report.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
SOURCE_PATHS: Final = (
    "capabilities/read-only/src/catalog.rs",
    "capabilities/read-only/src/protocol.rs",
    "capabilities/read-only/src/worker.rs",
    "capabilities/read-only/src/bin/agentmage-read-only-worker.rs",
    "kernel/contracts/src/workspace_snapshot.rs",
    "kernel/engine/src/tooling.rs",
    "kernel/engine/src/authority_transaction.rs",
    "platforms/linux/src/sandbox.rs",
    "shells/host/src/linux_coding_runtime.rs",
    "shells/host/src/linux_read.rs",
    "shells/host/src/protocol.rs",
    "docs/architecture/read-only-tool-protocol.md",
    "docs/verification/sprint-16-local-results.md",
    "scripts/sprint_16_evidence.py",
    "tests/test_sprint_16_evidence.py",
)
COMMANDS: Final = (
    (
        "closed-tool-pack",
        (
            "cargo",
            "test",
            "-p",
            "agentmage-capability-read-only",
            "--all-targets",
            "--locked",
        ),
    ),
    (
        "kernel-tool-authority",
        ("cargo", "test", "-p", "agentmage-kernel-engine", "--locked"),
    ),
    (
        "linux-sandbox-contract",
        ("cargo", "test", "-p", "agentmage-platform-linux", "--locked"),
    ),
    (
        "authenticated-host-flow",
        ("cargo", "test", "-p", "agentmage-host", "--locked"),
    ),
    ("effect-boundary", ("python", "scripts/effect_boundary.py")),
)
SECURITY_REQUIREMENTS: Final = [
    "SR-PLT-003",
    "SR-PLT-004",
    "SR-ACC-001",
    "SR-ACC-002",
    "SR-ACC-003",
    "SR-ACC-004",
    "SR-ACC-005",
    "SR-ACC-006",
    "SR-AI-005",
    "SR-TST-002",
    "SR-TST-004",
    "SR-TST-006",
    "RV-03",
    "RV-04",
]
BLOCKERS: Final = [
    {"code": "PACKAGED-ROOT-OWNED-LINUX-WORKER-NOT-INSTALLED", "owner": "16.1.1.5"},
    {"code": "MACOS-XPC-WORKER-EVIDENCE-MISSING", "owner": "16.1.1.5"},
    {"code": "LIVE-WORKER-ATTACK-MATRIX-INCOMPLETE", "owner": "16.1.3.3"},
    {"code": "WORKER-CANCEL-TIMEOUT-KILL-CRASH-CAMPAIGN-INCOMPLETE", "owner": "16.1.3.4"},
    {"code": "INDEPENDENT-WORKER-REVIEW-NOT-RETAINED", "owner": "16.1.3.5"},
]


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
        raise ValueError(f"committed Sprint 16 source is absent: {relative}")
    return result.stdout


def run_commands() -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for identifier, argv in COMMANDS:
        executable = shutil.which(argv[0])
        if executable is None:
            exit_code, output = 127, b"executable-unavailable"
        else:
            result = subprocess.run(
                (executable, *argv[1:]),
                cwd=ROOT,
                check=False,
                capture_output=True,
                timeout=900,
            )
            exit_code, output = result.returncode, result.stdout + result.stderr
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
        "record_type": "sprint_16_local_evidence",
        "source_revision": source_revision,
        "source_sha256": {
            path: sha256_bytes(git_file(source_revision, path)) for path in SOURCE_PATHS
        },
        "commands": commands,
        "security_requirement_ids": SECURITY_REQUIREMENTS,
        "implemented_contracts": {
            "catalog_tools": 10,
            "write_capable_tools": 0,
            "result_outcomes": 8,
            "hard_limit_classes": 6,
            "closed_schema_failure_classes": 7,
            "linux_fixed_input_mounts": 2,
            "generic_host_projection_manifest": True,
            "one_use_workspace_read_grant": True,
            "hash_verified_result": True,
            "repeat_and_call_depth_guard": True,
            "one_receipt_per_launched_attempt": True,
            "sensitive_output_withheld_before_model_context": True,
        },
        "platform_evidence": {
            "linux_contract_tests": local_pass,
            "linux_packaged_live_worker": False,
            "linux_live_attack_matrix": False,
            "macos_xpc_worker": False,
        },
        "verification_evidence": {
            "all_tool_golden_results": local_pass,
            "all_tool_schema_matrix": local_pass,
            "sealed_projection_tests": local_pass,
            "host_preview_and_cancellation": local_pass,
            "live_cleanup_campaign": False,
            "model_context_disclosure_redaction": True,
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
    commands = report.get("commands", [])
    if [item.get("id") for item in commands] != [item[0] for item in COMMANDS]:
        failures.append("command inventory drift")
    if any(
        item.get("exit_code") != 0
        or not SHA256.fullmatch(str(item.get("output_sha256", "")))
        for item in commands
    ):
        failures.append("command result invalid")
    if report.get("implemented_contracts") != {
        "catalog_tools": 10,
        "write_capable_tools": 0,
        "result_outcomes": 8,
        "hard_limit_classes": 6,
        "closed_schema_failure_classes": 7,
        "linux_fixed_input_mounts": 2,
        "generic_host_projection_manifest": True,
        "one_use_workspace_read_grant": True,
        "hash_verified_result": True,
        "repeat_and_call_depth_guard": True,
        "one_receipt_per_launched_attempt": True,
        "sensitive_output_withheld_before_model_context": True,
    }:
        failures.append("implemented-contract inventory drift")
    expected_summary = {
        "local_contract_passed": True,
        "sprint_status": "BLOCKED",
        "release_approval": False,
    }
    if report.get("summary") != expected_summary:
        failures.append("summary overclaim or local failure")
    platform = report.get("platform_evidence", {})
    verification = report.get("verification_evidence", {})
    for field in ("linux_packaged_live_worker", "linux_live_attack_matrix", "macos_xpc_worker"):
        if platform.get(field) is not False:
            failures.append(f"platform overclaim: {field}")
    for field in ("live_cleanup_campaign", "independent_review"):
        if verification.get(field) is not False:
            failures.append(f"verification overclaim: {field}")
    if verification.get("model_context_disclosure_redaction") is not True:
        failures.append("model-context disclosure evidence drift")
    if verify_current and REVISION.fullmatch(revision):
        for path in SOURCE_PATHS:
            digest = str(report.get("source_sha256", {}).get(path, ""))
            if not SHA256.fullmatch(digest) or digest != sha256_bytes(git_file(revision, path)):
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
        OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
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
