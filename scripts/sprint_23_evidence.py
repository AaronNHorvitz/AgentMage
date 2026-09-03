#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 23 evidence."""

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
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-23/local-evidence-report.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
SOURCE_PATHS: Final = (
    "docs/architecture/native-chat-accessibility.md",
    "docs/architecture/native-chat-model-discovery.md",
    "docs/verification/native-chat-accessibility-conformance-v0.1.md",
    "docs/verification/sprint-23-local-results.md",
    "kernel/contracts/src/model_discovery.rs",
    "kernel/engine/src/model_discovery.rs",
    "model-profiles/exact-profile-catalog.json",
    "scripts/package_candidate.py",
    "shells/host/src/linux_bootstrap.rs",
    "shells/host/src/linux_read.rs",
    "shells/host/src/main.rs",
    "shells/host/src/model_catalog_bootstrap.rs",
    "shells/host/src/package_verify.rs",
    "shells/host/src/protocol.rs",
    "shells/vscode/src/model_discovery.ts",
    "shells/vscode/src/provider.ts",
    "shells/vscode/src/runtime_transport.ts",
    "shells/vscode/test/model_discovery.test.ts",
    "shells/vscode/test/provider.test.ts",
    "scripts/sprint_23_evidence.py",
    "tests/test_sprint_23_evidence.py",
)
COMMANDS: Final = (
    ("model-discovery-tests", ("cargo", "test", "-p", "agentmage-kernel-engine",
                               "model_discovery", "--locked")),
    ("host-tests", ("cargo", "test", "-p", "agentmage-host", "--locked")),
    ("signed-catalog-bootstrap-tests", ("cargo", "test", "-p", "agentmage-host",
                                          "--bin", "agentmage-host", "--locked")),
    ("vscode-tests", ("npm", "run", "test", "--workspace", "@agentmage/vscode-shell")),
    ("vscode-lint", ("npm", "run", "lint", "--workspace", "@agentmage/vscode-shell")),
    ("product-gate", ("npm", "run", "product:check")),
    ("evidence-tests", ("python3", "-m", "unittest", "tests.test_sprint_23_evidence")),
    ("strict-local-source", ("python3", "scripts/strict_local_source_audit.py")),
    ("supply-chain", ("python3", "scripts/supply_chain.py")),
)
SECURITY_REQUIREMENTS: Final = [
    "SR-PLT-005", "SR-PLT-006", "SR-ACC-007", "SR-DAT-003", "SR-OPS-001",
    "SR-OPS-003", "SR-TST-004", "SR-AI-013", "SR-AI-015", "SR-MGM-001",
    "SR-CIV-006", "SR-CIV-007", "SR-CIV-008", "SR-CIV-009", "SR-TST-008",
]
BLOCKERS: Final = [
    {"code": "CURRENT-ACTIVATION-AND-NATIVE-EXPOSURE-ABSENT", "owner": "23.3.1.1"},
    {"code": "PRODUCTION-MODEL-ROUTE-ABSENT", "owner": "23.1.1.3"},
    {"code": "REQUEST-PHASE-PRESERVATION-MATRIX-ABSENT", "owner": "23.3.2.3"},
    {"code": "INSTALLED-VSCODE-NATIVE-EVIDENCE-ABSENT", "owner": "23.1.3.1"},
    {"code": "LINUX-ACCESSIBILITY-EVIDENCE-ABSENT", "owner": "23.2.2.2"},
    {"code": "MACOS-ACCESSIBILITY-EVIDENCE-ABSENT", "owner": "23.2.2.2"},
    {"code": "HANDOFF-PREVIEW-DEFERRED-TO-SPRINT-24", "owner": "23.2.1.1"},
    {"code": "INDEPENDENT-SPRINT-23-REVIEW-ABSENT", "owner": "23.1.3.4"},
]
IMPLEMENTED: Final = {
    "stable_vscode_provider_registration": True,
    "authenticated_discovery_transport": True,
    "exact_profile_projection": True,
    "exact_selection_revalidation": True,
    "profile_family_prerequisite": False,
    "automatic_model_substitution": False,
    "structured_final_chat_parts": True,
    "ordered_progress_streaming": True,
    "verified_output_streaming": True,
    "validated_local_display_links": True,
    "complete_textual_native_indicators": True,
    "signed_catalog_production_bootstrap": True,
    "production_model_inference": False,
    "model_token_streaming": False,
    "exact_tokenizer_counting": False,
    "native_accessibility_conformance": False,
}


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def git_file(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, check=False,
        capture_output=True, timeout=30,
    )
    if result.returncode != 0:
        raise ValueError(f"committed Sprint 23 source is absent: {path}")
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
                capture_output=True, timeout=1200,
            )
            code, output = result.returncode, result.stdout + result.stderr
        records.append({
            "id": identifier, "argv": list(argv), "exit_code": code,
            "output_sha256": digest(output),
        })
    return records


def build_report(revision: str, commands: list[dict[str, Any]]) -> dict[str, Any]:
    local_pass = all(item["exit_code"] == 0 for item in commands)
    return {
        "schema_version": 1,
        "record_type": "sprint_23_local_evidence",
        "source_revision": revision,
        "source_sha256": {path: digest(git_file(revision, path)) for path in SOURCE_PATHS},
        "environment": environment_manifest(),
        "commands": commands,
        "security_requirement_ids": SECURITY_REQUIREMENTS,
        "implemented_contracts": IMPLEMENTED,
        "verification_evidence": {
            "exact_picker_fixture_matrix": local_pass,
            "exact_identity_mutation_matrix": local_pass,
            "authenticated_host_transport": local_pass,
            "structured_chat_output": local_pass,
            "closed_display_link_grammar": local_pass,
            "complete_product_local_gate": local_pass,
            "signed_catalog_production_integration": local_pass,
            "production_model_invocation": False,
            "complete_request_phase_no_fallback": False,
            "installed_native_vscode_workflow": False,
            "linux_native_accessibility": False,
            "macos_native_accessibility": False,
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
    if report.get("implemented_contracts") != IMPLEMENTED:
        failures.append("implemented contract drift")
    if report.get("blockers") != BLOCKERS:
        failures.append("blocker drift")
    commands = report.get("commands", [])
    if [item.get("id") for item in commands] != [item[0] for item in COMMANDS]:
        failures.append("command inventory drift")
    if any(item.get("exit_code") != 0 or not SHA256.fullmatch(
            str(item.get("output_sha256", ""))) for item in commands):
        failures.append("command result invalid")
    if report.get("summary") != {
        "local_contract_passed": True, "sprint_status": "BLOCKED", "release_approval": False,
    }:
        failures.append("summary overclaim or local failure")
    verification = report.get("verification_evidence", {})
    if verification.get("signed_catalog_production_integration") is not True:
        failures.append("signed catalog bootstrap evidence missing")
    for field in (
        "production_model_invocation",
        "complete_request_phase_no_fallback", "installed_native_vscode_workflow",
        "linux_native_accessibility", "macos_native_accessibility", "independent_review",
    ):
        if verification.get(field) is not False:
            failures.append(f"verification overclaim: {field}")
    for field in (
        "profile_family_prerequisite", "automatic_model_substitution",
        "production_model_inference", "model_token_streaming", "exact_tokenizer_counting",
        "native_accessibility_conformance",
    ):
        if report.get("implemented_contracts", {}).get(field) is not False:
            failures.append(f"implementation overclaim: {field}")
    environment = report.get("environment", {})
    if set(environment) != {"system", "release", "machine", "python", "rustc", "cargo", "node", "npm"}:
        failures.append("environment manifest drift")
    elif any(not isinstance(value, str) or not value or len(value) > 256
             for value in environment.values()):
        failures.append("environment manifest invalid")
    sources = report.get("source_sha256", {})
    if set(sources) != set(SOURCE_PATHS):
        failures.append("source inventory drift")
    elif verify_current and REVISION.fullmatch(revision):
        for path in SOURCE_PATHS:
            if sources[path] != digest(git_file(revision, path)):
                failures.append(f"source digest drift: {path}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    args = parser.parse_args()
    if args.write:
        revision = subprocess.run(
            ["git", "rev-parse", args.source_revision], cwd=ROOT, check=True,
            capture_output=True, text=True,
        ).stdout.strip()
        report = build_report(revision, run_commands())
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    else:
        report = json.loads(OUTPUT.read_text(encoding="utf-8"))
    failures = validate_report(report)
    if failures:
        print("\n".join(failures))
        return 1
    print(json.dumps(report["summary"], sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
