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

try:
    from scripts import sprint_16_linux_worker_evidence as worker_evidence
except ModuleNotFoundError:
    import sprint_16_linux_worker_evidence as worker_evidence

ROOT: Final = Path(__file__).resolve().parents[1]
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-16/local-evidence-report.json"
INSTALLED_WORKER_OUTPUT: Final = (
    ROOT / "artifacts/sprints/sprint-16/installed-linux-worker-matrix.json"
)
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
SOURCE_PATHS: Final = (
    "capabilities/read-only/Cargo.toml",
    "capabilities/read-only/src/bin/agentmage-read-only-lifecycle-fixture.rs",
    "capabilities/read-only/src/catalog.rs",
    "capabilities/read-only/src/protocol.rs",
    "capabilities/read-only/src/worker.rs",
    "capabilities/read-only/src/bin/agentmage-read-only-worker.rs",
    "kernel/contracts/src/workspace_snapshot.rs",
    "kernel/engine/src/tooling.rs",
    "kernel/engine/src/authority_transaction.rs",
    "packaging/README.md",
    "packaging/linux/README.md",
    "packaging/linux/agentmage.spec.in",
    "packaging/linux/agentmage-release.spec.in",
    "platforms/linux/src/lib.rs",
    "platforms/linux/src/sandbox.rs",
    "scripts/package_candidate.py",
    "scripts/package_lifecycle.py",
    "scripts/linux_package_lifecycle_evidence.py",
    "scripts/linux_docker_prerequisite_evidence.py",
    "scripts/sprint_16_linux_worker_evidence.py",
    "scripts/sprint_16_linux_worker_guest.py",
    "shells/host/src/linux_coding_runtime.rs",
    "shells/host/src/linux_read.rs",
    "shells/host/src/package_verify.rs",
    "shells/host/src/protocol.rs",
    "docs/architecture/read-only-tool-protocol.md",
    "docs/verification/sprint-16-local-results.md",
    "scripts/sprint_16_evidence.py",
    "tests/test_package_candidate.py",
    "tests/test_package_lifecycle.py",
    "tests/test_linux_package_lifecycle_evidence.py",
    "tests/test_linux_docker_prerequisite_evidence.py",
    "tests/test_sprint_16_evidence.py",
    "tests/test_sprint_16_linux_worker_evidence.py",
)
COMMANDS: Final = (
    (
        "package-worker-payload",
        (
            "python",
            "-m",
            "unittest",
            "tests.test_package_candidate",
            "tests.test_package_lifecycle",
            "tests.test_linux_package_lifecycle_evidence",
            "tests.test_linux_docker_prerequisite_evidence",
        ),
    ),
    (
        "installed-linux-worker-operation-matrix",
        ("python", "scripts/sprint_16_linux_worker_evidence.py"),
    ),
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
    {"code": "MACOS-XPC-WORKER-EVIDENCE-MISSING", "owner": "16.1.1.5"},
    {"code": "MACOS-LIVE-WORKER-ATTACK-MATRIX-MISSING", "owner": "16.1.3.3"},
    {"code": "MACOS-WORKER-LIFECYCLE-CAMPAIGN-MISSING", "owner": "16.1.3.4"},
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


def installed_worker_summary(value: Any, encoded: bytes) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ValueError("installed worker matrix must be an object")
    targets = value.get("targets")
    if (
        value.get("status")
        != "pass-installed-linux-worker-operation-attack-lifecycle-matrix"
        or not isinstance(targets, list)
        or [target.get("target_id") for target in targets]
        != ["fedora-44-x86_64", "ubuntu-26.04-x86_64"]
        or value.get("verified_operations") != worker_evidence.VERIFIED_OPERATIONS
        or value.get("linux_attack_matrix_complete") is not True
        or value.get("linux_lifecycle_campaign_complete") is not True
        or any(
            target.get("guest_result", {}).get("attack_cases")
            != worker_evidence.ATTACK_CASES
            for target in targets
        )
        or any(
            target.get("guest_result", {}).get("lifecycle_cases")
            != worker_evidence.LIFECYCLE_CASES
            for target in targets
        )
    ):
        raise ValueError("installed worker matrix is not the admitted subset")
    return {
        "artifact": "artifacts/sprints/sprint-16/installed-linux-worker-matrix.json",
        "artifact_sha256": sha256_bytes(encoded),
        "source_revision": value.get("source_revision"),
        "target_ids": [target["target_id"] for target in targets],
        "verified_operations": value["verified_operations"],
        "linux_attack_cases": worker_evidence.ATTACK_CASES,
        "linux_lifecycle_cases": worker_evidence.LIFECYCLE_CASES,
    }


def load_installed_worker_summary() -> dict[str, Any]:
    encoded = INSTALLED_WORKER_OUTPUT.read_bytes()
    return installed_worker_summary(json.loads(encoded), encoded)


def build_report(
    source_revision: str,
    commands: list[dict[str, Any]],
    installed_worker: dict[str, Any],
) -> dict[str, Any]:
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
            "packaged_worker_payload_declared": True,
            "installed_linux_worker_operation_matrix": installed_worker,
        },
        "platform_evidence": {
            "linux_contract_tests": local_pass,
            "linux_packaged_live_worker": True,
            "linux_complete_operation_matrix": True,
            "linux_live_attack_matrix": True,
            "linux_live_lifecycle_campaign": True,
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
        "packaged_worker_payload_declared": True,
        "installed_linux_worker_operation_matrix": report.get(
            "implemented_contracts", {}
        ).get(
            "installed_linux_worker_operation_matrix"
        ),
    }:
        failures.append("implemented-contract inventory drift")
    installed_worker = report.get("implemented_contracts", {}).get(
        "installed_linux_worker_operation_matrix", {}
    )
    if (
        installed_worker.get("artifact")
        != "artifacts/sprints/sprint-16/installed-linux-worker-matrix.json"
        or not SHA256.fullmatch(str(installed_worker.get("artifact_sha256", "")))
        or not REVISION.fullmatch(str(installed_worker.get("source_revision", "")))
        or installed_worker.get("target_ids")
        != ["fedora-44-x86_64", "ubuntu-26.04-x86_64"]
        or installed_worker.get("verified_operations")
        != worker_evidence.VERIFIED_OPERATIONS
        or installed_worker.get("linux_attack_cases") != worker_evidence.ATTACK_CASES
        or installed_worker.get("linux_lifecycle_cases")
        != worker_evidence.LIFECYCLE_CASES
    ):
        failures.append("installed worker operation-matrix evidence drift")
    expected_summary = {
        "local_contract_passed": True,
        "sprint_status": "BLOCKED",
        "release_approval": False,
    }
    if report.get("summary") != expected_summary:
        failures.append("summary overclaim or local failure")
    platform = report.get("platform_evidence", {})
    verification = report.get("verification_evidence", {})
    if platform.get("linux_packaged_live_worker") is not True:
        failures.append("installed Linux worker evidence drift")
    if platform.get("linux_complete_operation_matrix") is not True:
        failures.append("installed Linux complete operation matrix drift")
    if platform.get("linux_live_attack_matrix") is not True:
        failures.append("installed Linux attack matrix drift")
    if platform.get("linux_live_lifecycle_campaign") is not True:
        failures.append("installed Linux lifecycle campaign drift")
    for field in ("macos_xpc_worker",):
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
        report = build_report(revision, run_commands(), load_installed_worker_summary())
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
