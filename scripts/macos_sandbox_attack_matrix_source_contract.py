#!/usr/bin/env python3
"""Retain the source-only S-008-ST01 hostile-access matrix contract."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.macos_release_runner_source_contract import COMPONENTS
from scripts.macos_sandbox_attack_results import ATTACK_CLASSES, expected_attack_pairs

REPORT_PATH = ROOT / (
    "artifacts/sprints/sprint-8/story-8.1/"
    "macos-sandbox-attack-matrix-source-contract.json"
)
UPSTREAM_REPORT_PATH = ROOT / (
    "artifacts/sprints/sprint-8/story-8.1/"
    "macos-sandbox-attack-results-source-contract.json"
)
UPSTREAM_REPORT_SHA256 = (
    "d430776b93603fa8b58ae77d6d20cfc2c407d85e6ae33d294f1a983f912d95e1"
)
SOURCE_PATHS = (
    "packaging/macos/SANDBOX-ATTACK-MATRIX.md",
    "platforms/macos/Tests/AgentMageMacOSPlatformTests/MacOSSandboxAttackMatrixTests.swift",
    "packaging/macos/SANDBOX-ATTACK-RESULTS.md",
    "scripts/macos_sandbox_attack_results.py",
    "tests/test_macos_sandbox_attack_results.py",
    "artifacts/sprints/sprint-8/story-8.1/macos-sandbox-attack-results-source-contract.json",
    "scripts/macos_sandbox_attack_matrix_source_contract.py",
    "tests/test_macos_sandbox_attack_matrix_source_contract.py",
)
ZERO_DISPOSITIONS = (
    "unauthorizedAccessCount == 0",
    "unauthorizedByteCount == 0",
    "networkConnectionCount == 0",
    "networkByteCount == 0",
    "descendantProcessCount == 0",
    "residueCount == 0",
    "!result.canaryObserved",
    "!result.workspaceModified",
    "!result.authorityBroadened",
)


def git(*arguments: str, root: Path = ROOT, binary: bool = False) -> str | bytes:
    output = subprocess.run(
        ["git", *arguments], cwd=root, check=True, capture_output=True
    ).stdout
    return output if binary else output.decode().strip()


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def validate_upstream(root: Path = ROOT) -> list[str]:
    path = root / UPSTREAM_REPORT_PATH.relative_to(ROOT)
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return ["task 8.1.2.4 sandbox source report is unavailable"]
    failures: list[str] = []
    if sha256_file(path) != UPSTREAM_REPORT_SHA256:
        failures.append("task 8.1.2.4 sandbox source report digest changed")
    if (
        not isinstance(value, dict)
        or value.get("record_type")
        != "macos-sandbox-attack-results-source-contract"
        or value.get("task_id") != "8.1.2.4"
        or value.get("status") != "prepared-source-only-blocked-macos"
    ):
        failures.append("task 8.1.2.4 sandbox source report identity changed")
        return failures
    contract = value.get("contract", {})
    if (
        contract.get("profile_count") != len(COMPONENTS)
        or contract.get("attack_class_count") != len(ATTACK_CLASSES)
        or contract.get("attack_case_count") != len(expected_attack_pairs())
    ):
        failures.append("task 8.1.2.4 sandbox matrix closure changed")
    if any(value.get("execution", {}).values()) or any(value.get("claims", {}).values()):
        failures.append("task 8.1.2.4 sandbox source report overclaims execution")
    return failures


def validate_sources(root: Path = ROOT) -> list[str]:
    failures = [
        f"missing source input: {relative}"
        for relative in SOURCE_PATHS
        if not (root / relative).is_file()
    ]
    if failures:
        return failures
    failures.extend(validate_upstream(root))
    swift = (
        root
        / "platforms/macos/Tests/AgentMageMacOSPlatformTests/"
        "MacOSSandboxAttackMatrixTests.swift"
    ).read_text(encoding="utf-8")
    evidence = (root / "scripts/macos_sandbox_attack_results.py").read_text(
        encoding="utf-8"
    )
    tests = (
        root / "tests/test_macos_sandbox_attack_matrix_source_contract.py"
    ).read_text(encoding="utf-8")
    documentation = (root / "packaging/macos/SANDBOX-ATTACK-MATRIX.md").read_text(
        encoding="utf-8"
    )
    swift_lines = [line.strip() for line in swift.splitlines()]
    component_terms = {
        "kernel_host": 'case kernelHost = "kernel_host"',
        "vscode_bridge": 'case vscodeBridge = "vscode_bridge"',
        "xpc_tool_helper": 'case xpcToolHelper = "xpc_tool_helper"',
        "metal_inference_service": (
            'case metalInferenceService = "metal_inference_service"'
        ),
    }
    attack_terms = {
        "ambient-home": 'case ambientHome = "ambient-home"',
        "device": "case device",
        "process": "case process",
        "environment": "case environment",
        "credential": "case credential",
        "network": "case network",
        "workspace-write": 'case workspaceWrite = "workspace-write"',
        "grant": "case grant",
        "cross-user": 'case crossUser = "cross-user"',
    }
    for component in COMPONENTS:
        if swift_lines.count(component_terms[component]) != 1:
            failures.append(f"Swift sandbox component is not exact: {component}")
    for attack_class in ATTACK_CLASSES:
        if swift_lines.count(attack_terms[attack_class]) != 1:
            failures.append(f"Swift sandbox attack class is not exact: {attack_class}")
    for term in (
        "SandboxAttackComponent.allCases.flatMap",
        "SandboxAttackClass.allCases.map",
        "SandboxAttackCampaign.expectedCases.count == 36",
        "Set(SandboxAttackCampaign.expectedCases.map(\\.identifier)).count == 36",
        "let result = try probe.attempt(attack)",
        "result.caseIdentifier == attack.identifier",
        "UnsafeAttackMutation.allCases.count == 11",
    ):
        if term not in swift:
            failures.append(f"Swift sandbox attack campaign missing term: {term}")
    for term in ZERO_DISPOSITIONS:
        if term not in swift:
            failures.append(f"Swift sandbox zero-result guard missing: {term}")
    evidence_terms = {
        "expected_attack_pairs()": 6,
        '"passed-zero-unauthorized-access"': 2,
        '"unauthorized_access_count": 0': 3,
        '"authority_broadened": False': 2,
        '"native_operations_executed_by_ingestor": False': 2,
    }
    for term, count in evidence_terms.items():
        if evidence.count(term) != count:
            failures.append(f"sandbox evidence verifier missing term: {term}")
    if "Five closed tests" not in tests:
        failures.append("sandbox matrix source mutation suite is not closed")
    for statement in (
        "Cartesian product is exactly 36 cases",
        "test-local probe interface grants no production authority",
        "No native hostile-access campaign",
    ):
        if statement not in documentation:
            failures.append(f"sandbox matrix procedure missing statement: {statement}")
    return failures


def resolve_revision(source_revision: str, root: Path = ROOT) -> tuple[str, str]:
    revision = str(git("rev-parse", source_revision, root=root))
    tree = str(git("rev-parse", f"{revision}^{{tree}}", root=root))
    for relative in SOURCE_PATHS:
        committed = git("show", f"{revision}:{relative}", root=root, binary=True)
        if committed != (root / relative).read_bytes():
            raise ValueError(f"reviewed sandbox attack matrix source changed: {relative}")
    return revision, tree


def build_report(
    root: Path = ROOT, *, source_revision: str = "HEAD"
) -> dict[str, Any]:
    failures = validate_sources(root)
    if failures:
        raise ValueError("; ".join(failures))
    revision, tree = resolve_revision(source_revision, root)
    return {
        "schema_version": 1,
        "record_type": "macos-sandbox-attack-matrix-source-contract",
        "task_id": "8.1.3.2",
        "status": "prepared-source-only-blocked-macos",
        "source_revision": revision,
        "source_tree": tree,
        "source_files": [
            {"path": relative, "sha256": sha256_file(root / relative)}
            for relative in SOURCE_PATHS
        ],
        "upstream_source_contract": {
            "task_id": "8.1.2.4",
            "path": str(UPSTREAM_REPORT_PATH.relative_to(ROOT)),
            "sha256": UPSTREAM_REPORT_SHA256,
        },
        "contract": {
            "component_count": len(COMPONENTS),
            "attack_class_count": len(ATTACK_CLASSES),
            "attack_case_count": len(expected_attack_pairs()),
            "zero_result_guard_count": len(ZERO_DISPOSITIONS),
            "unsafe_result_mutation_count": 11,
            "accepted_unauthorized_case_count": 0,
            "native_operation_authority": False,
            "filesystem_authority": False,
            "device_authority": False,
            "process_authority": False,
            "credential_authority": False,
            "network_authority": False,
            "workspace_authority": False,
            "grant_authority": False,
            "cross_user_authority": False,
            "attack_authority": False,
            "support_promotion_authority": False,
        },
        "execution": {
            "swift_tests_performed": False,
            "native_attack_campaign_observed": False,
            "signed_components_executed": False,
            "active_sandbox_profiles_observed": False,
            "protected_logs_observed": False,
            "independent_review_performed": False,
        },
        "claims": {
            "task_complete": False,
            "native_sandbox_results_exist": False,
            "release_candidate_exists": False,
            "macos_support": False,
        },
        "remaining_blockers": [
            "No external Apple Silicon Swift 6 execution of the exact 36-case matrix exists.",
            "No installed signed host, bridge, tool helper, or inference service has been attacked.",
            "No native zero-access, zero-byte, zero-authority, and zero-residue results or protected logs exist.",
            "No production release bundle and active four-component sandbox profile set exists.",
            "No independent reviewer has reconciled the native hostile-access evidence.",
        ],
    }


def canonical_bytes(value: dict[str, Any]) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    try:
        if arguments.write:
            report = build_report(source_revision=arguments.source_revision)
            REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
            REPORT_PATH.write_bytes(canonical_bytes(report))
        else:
            retained = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
            revision = retained.get("source_revision")
            if not isinstance(revision, str):
                raise ValueError("retained sandbox matrix contract has no revision")
            expected = canonical_bytes(build_report(source_revision=revision))
            if REPORT_PATH.read_bytes() != expected:
                raise ValueError("sandbox attack matrix source contract is stale")
    except (OSError, subprocess.CalledProcessError, ValueError, json.JSONDecodeError) as error:
        print(f"macOS sandbox attack matrix source verification failed: {error}")
        return 1
    print("macOS sandbox attack matrix source contract passed without native promotion")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
