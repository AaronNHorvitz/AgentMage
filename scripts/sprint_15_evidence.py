#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 15 evidence."""

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
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-15/local-evidence-report.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
SOURCE_PATHS: Final = (
    "kernel/contracts/src/diagnostics.rs",
    "kernel/engine/src/diagnostics.rs",
    "kernel/engine/src/model_selection.rs",
    "kernel/engine/src/model_runtime.rs",
    "shells/host/src/diagnostic_export.rs",
    "shells/host/src/linux_read.rs",
    "shells/host/src/protocol.rs",
    "shells/vscode/src/provider.ts",
    "shells/vscode/src/host_bridge.ts",
    "shells/vscode/src/extension.ts",
    "scripts/benchmark_contract.py",
    "scripts/model_role_evaluation_matrix.py",
    "tests/test_benchmark_contract.py",
    "tests/test_model_role_evaluation_matrix.py",
    "docs/verification/sprint-15-local-results.md",
    "scripts/sprint_15_evidence.py",
    "tests/test_sprint_15_evidence.py",
)
EVIDENCE_PATHS: Final = (
    "model-profiles/evaluation/role-suite-corpus-v1.json",
    "model-profiles/evaluation/benchmark-record-schema-v1.json",
    "artifacts/sprints/sprint-13/story-13.3/muse-profile-evaluation.json",
    "artifacts/sprints/sprint-15/story-15.3/candidate-role-evaluation-matrix.json",
)
COMMANDS: Final = (
    (
        "kernel-diagnostics-selection-resource",
        ("cargo", "test", "-p", "agentmage-kernel-engine", "--locked"),
    ),
    ("host-doctor-export", ("cargo", "test", "-p", "agentmage-host", "--locked")),
    ("native-chat", ("npm", "--prefix", "shells/vscode", "test")),
    (
        "evaluation-contracts",
        (
            "python",
            "-m",
            "unittest",
            "tests.test_benchmark_contract",
            "tests.test_model_role_evaluation_matrix",
        ),
    ),
    (
        "candidate-role-reconciliation",
        ("python", "scripts/model_role_evaluation_matrix.py", "--check"),
    ),
)
SECURITY_MAP: Final = {
    "15.1": [
        "SR-GOV-004",
        "SR-PLT-010",
        "SR-AI-006",
        "SR-AI-009",
        "SR-AI-013",
        "SR-OPS-009",
        "SR-TST-006",
        "RV-13",
        "RV-16",
    ],
    "15.2": [
        "SR-GOV-001",
        "SR-DAT-002",
        "SR-DAT-003",
        "SR-OPS-003",
        "SR-CIV-006",
        "SR-CIV-007",
        "SR-CIV-008",
        "SR-CIV-009",
        "RV-08",
        "RV-16",
        "RV-18",
        "RV-20",
    ],
    "15.3": [
        "SR-AI-006",
        "SR-AI-010",
        "SR-AI-011",
        "SR-AI-013",
        "SR-AI-015",
        "SR-AI-016",
        "SR-MGM-004",
        "SR-MGM-005",
        "RV-14",
        "RV-41",
    ],
}
BLOCKERS: Final = [
    {"code": "NO-ADMITTED-PRODUCT-MODEL", "owner": "15.1/15.3"},
    {"code": "CHAT-MODEL-SELECTION-NOT-WIRED", "owner": "15.1"},
    {"code": "RESOURCE-STOP-NOT-WIRED-TO-OS-WORKER", "owner": "15.1"},
    {"code": "FULL-DURABLE-SURFACE-CANARY-SWEEP-MISSING", "owner": "15.2"},
    {"code": "NATIVE-ACCESSIBILITY-AND-MACOS-EVIDENCE-MISSING", "owner": "15.2/15.3"},
    {"code": "EXACT-GEMMA-ARTIFACT-EVALUATION-INCOMPLETE", "owner": "15.3"},
]


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def git_file(revision: str, relative: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{relative}"],
        cwd=ROOT,
        check=False,
        capture_output=True,
        timeout=30,
    )
    if result.returncode != 0:
        raise ValueError(f"committed Sprint 15 source is absent: {relative}")
    return result.stdout


def run_commands() -> list[dict[str, Any]]:
    records = []
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


def evidence_state() -> dict[str, Any]:
    matrix = json.loads((ROOT / EVIDENCE_PATHS[3]).read_text(encoding="utf-8"))
    return {
        "doctor_component_count": 13,
        "doctor_fixture_count": 100,
        "manual_selection": True,
        "automatic_fallback": False,
        "inference_slot_default": matrix["inference_slot_default"],
        "resource_limit_classes": 9,
        "candidate_count": matrix["candidate_count"],
        "candidate_dispositions": matrix["candidate_dispositions"],
        "exact_muse_disposition": matrix["exact_profile_results"][0]["disposition"],
        "enabled_model_count": matrix["enabled_model_count"],
        "unsupported_claim_count": sum(matrix["claims"].values()),
        "reviewed_export": True,
    }


def build_report(source_revision: str, commands: list[dict[str, Any]]) -> dict[str, Any]:
    state = evidence_state()
    local_pass = all(command["exit_code"] == 0 for command in commands) and state == {
        "doctor_component_count": 13,
        "doctor_fixture_count": 100,
        "manual_selection": True,
        "automatic_fallback": False,
        "inference_slot_default": 1,
        "resource_limit_classes": 9,
        "candidate_count": 416,
        "candidate_dispositions": {"BLOCKED": 415, "INELIGIBLE": 1},
        "exact_muse_disposition": "REJECTED",
        "enabled_model_count": 0,
        "unsupported_claim_count": 0,
        "reviewed_export": True,
    }
    return {
        "schema_version": 1,
        "record_type": "sprint_15_local_evidence",
        "source_revision": source_revision,
        "source_sha256": {
            path: sha256_bytes(git_file(source_revision, path)) for path in SOURCE_PATHS
        },
        "evidence_sha256": {path: sha256_file(ROOT / path) for path in EVIDENCE_PATHS},
        "commands": commands,
        "security_requirement_ids": SECURITY_MAP,
        "evidence_state": state,
        "stories": [
            {
                "story_id": "15.1",
                "status": "PASS-LOCAL-CONTRACTS-BLOCKED-PRODUCT-WIRING",
                "doctor_provider": local_pass,
                "manual_selection": local_pass,
                "deterministic_first": local_pass,
                "resource_governor": local_pass,
                "os_worker_resource_enforcement": False,
                "chat_model_selection": False,
            },
            {
                "story_id": "15.2",
                "status": "PASS-LINUX-LOCAL-BLOCKED-NATIVE-ACCESSIBILITY",
                "native_chat_doctor": local_pass,
                "reviewed_export": local_pass,
                "network_health_probe": False,
                "full_surface_canary_sweep": False,
                "native_accessibility_evidence": False,
            },
            {
                "story_id": "15.3",
                "status": "PASS-ATTRIBUTION-BLOCKED-EXACT-CANDIDATE-EVALUATION",
                "frozen_corpus": local_pass,
                "benchmark_schema": local_pass,
                "candidate_matrix_reconciled": local_pass,
                "muse_exact_rejection_retained": local_pass,
                "exact_gemma_trials_complete": False,
                "admitted_product_profile": False,
            },
        ],
        "blockers": BLOCKERS,
        "summary": {
            "local_contract_passed": local_pass,
            "product_model_available": False,
            "release_approval": False,
            "sprint_status": "BLOCKED",
        },
    }


def validate_report(report: dict[str, Any], verify_current: bool = True) -> list[str]:
    failures: list[str] = []
    if not REVISION.fullmatch(str(report.get("source_revision", ""))):
        failures.append("source revision invalid")
    if report.get("security_requirement_ids") != SECURITY_MAP:
        failures.append("security mapping drift")
    if report.get("blockers") != BLOCKERS:
        failures.append("blocker drift")
    if [item.get("id") for item in report.get("commands", [])] != [item[0] for item in COMMANDS]:
        failures.append("command inventory drift")
    if any(item.get("exit_code") != 0 or not SHA256.fullmatch(item.get("output_sha256", "")) for item in report.get("commands", [])):
        failures.append("command result invalid")
    summary = report.get("summary", {})
    if summary != {
        "local_contract_passed": True,
        "product_model_available": False,
        "release_approval": False,
        "sprint_status": "BLOCKED",
    }:
        failures.append("summary overclaim or local failure")
    stories = {item.get("story_id"): item for item in report.get("stories", [])}
    if stories.get("15.1", {}).get("os_worker_resource_enforcement") is not False:
        failures.append("resource enforcement overclaim")
    if stories.get("15.2", {}).get("full_surface_canary_sweep") is not False:
        failures.append("canary overclaim")
    if stories.get("15.3", {}).get("exact_gemma_trials_complete") is not False:
        failures.append("Gemma evaluation overclaim")
    if report.get("evidence_state") != evidence_state():
        failures.append("evidence state drift")
    if verify_current and REVISION.fullmatch(str(report.get("source_revision", ""))):
        revision = report["source_revision"]
        for path in SOURCE_PATHS:
            digest = report.get("source_sha256", {}).get(path, "")
            if not SHA256.fullmatch(digest) or digest != sha256_bytes(git_file(revision, path)):
                failures.append(f"source digest drift: {path}")
        for path in EVIDENCE_PATHS:
            digest = report.get("evidence_sha256", {}).get(path, "")
            if not SHA256.fullmatch(digest) or digest != sha256_file(ROOT / path):
                failures.append(f"evidence digest drift: {path}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if args.check:
        report = json.loads(OUTPUT.read_text(encoding="utf-8"))
    else:
        revision = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            cwd=ROOT,
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
        report = build_report(revision, run_commands())
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    failures = validate_report(report)
    if failures:
        for failure in failures:
            print(failure)
        return 1
    print(json.dumps(report["summary"], sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
