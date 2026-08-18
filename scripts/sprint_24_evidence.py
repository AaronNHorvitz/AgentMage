#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 24 evidence."""

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
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-24/local-evidence-report.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
SOURCE_PATHS: Final = (
    "docs/architecture/local-codex-handoff-boundary.md",
    "docs/verification/sprint-24-local-results.md",
    "kernel/contracts/src/handoff.rs",
    "kernel/engine/src/context_management.rs",
    "kernel/engine/src/handoff.rs",
    "kernel/engine/src/lib.rs",
    "security/strict-local-source-policy.json",
    "shells/host/src/linux_read.rs",
    "shells/host/src/protocol.rs",
    "shells/vscode/src/extension.ts",
    "shells/vscode/src/handoff.ts",
    "shells/vscode/src/host_bootstrap.ts",
    "shells/vscode/src/host_bridge.ts",
    "shells/vscode/src/provider.ts",
    "shells/vscode/test/handoff.test.ts",
    "shells/vscode/test/host_bridge.test.ts",
    "shells/vscode/test/provider.test.ts",
    "scripts/sprint_24_evidence.py",
    "tests/test_sprint_24_evidence.py",
)
COMMANDS: Final = (
    ("kernel-handoff-tests", ("cargo", "test", "-p", "agentmage-kernel-engine", "handoff", "--locked")),
    ("host-handoff-tests", ("cargo", "test", "-p", "agentmage-host", "handoff", "--locked")),
    ("vscode-tests", ("npm", "run", "test", "--workspace", "@agentmage/vscode-shell")),
    ("vscode-lint", ("npm", "run", "lint", "--workspace", "@agentmage/vscode-shell")),
    ("product-gate", ("npm", "run", "product:check")),
    ("evidence-tests", ("python3", "-m", "unittest", "tests.test_sprint_24_evidence")),
    ("strict-local-source", ("python3", "scripts/strict_local_source_audit.py")),
    ("effect-boundary", ("python3", "scripts/effect_boundary.py")),
    ("supply-chain", ("python3", "scripts/supply_chain.py")),
)
SECURITY_REQUIREMENTS: Final = [
    "SR-ACC-007", "SR-ACC-008", "SR-DAT-002", "SR-DAT-003", "SR-NET-002",
    "SR-AI-004", "SR-AI-008", "SR-AI-010", "SR-OPS-001", "SR-OPS-003",
    "SR-CIV-003", "SR-CIV-004", "SR-CIV-009",
]
BLOCKERS: Final = [
    {"code": "PRODUCTION-HOST-HANDOFF-ACTIVATION-ABSENT", "owner": "24.1.3.2"},
    {"code": "INSTALLED-VSCODE-HANDOFF-EVIDENCE-ABSENT", "owner": "24.2.2.3"},
    {"code": "LINUX-ACCESSIBILITY-EVIDENCE-ABSENT", "owner": "24.2.2.4"},
    {"code": "WINDOWS-ACCESSIBILITY-EVIDENCE-ABSENT", "owner": "24.2.2.4"},
    {"code": "MACOS-ACCESSIBILITY-EVIDENCE-ABSENT", "owner": "24.2.2.4"},
    {"code": "LIVE-HANDOFF-ZERO-EGRESS-EVIDENCE-ABSENT", "owner": "24.1.3.2"},
    {"code": "INDEPENDENT-SPRINT-24-REVIEW-ABSENT", "owner": "24.1.3.4"},
]
IMPLEMENTED: Final = {
    "content_addressed_packet_contract": True,
    "complete_disclosure_schema": True,
    "secret_hidden_unrelated_content_denial": True,
    "explicit_non_public_acknowledgment": True,
    "pre_render_source_policy_redaction_revalidation": True,
    "authenticated_local_transport": True,
    "independent_extension_digest_verification": True,
    "prohibited_action_local_receipts": True,
    "canonical_session_composition_contract": True,
    "automatic_external_delivery": False,
    "codex_invocation": False,
    "clipboard_or_interface_control": False,
    "installed_production_session_activation": False,
}


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def git_file(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, check=False,
        capture_output=True, timeout=30,
    )
    if result.returncode != 0:
        raise ValueError(f"committed Sprint 24 source is absent: {path}")
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
        "system": platform.system(), "release": platform.release(),
        "machine": platform.machine(), "python": platform.python_version(),
        "rustc": version("rustc", "--version"), "cargo": version("cargo", "--version"),
        "node": version("node", "--version"), "npm": version("npm", "--version"),
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
        "record_type": "sprint_24_local_evidence",
        "source_revision": revision,
        "source_sha256": {path: digest(git_file(revision, path)) for path in SOURCE_PATHS},
        "environment": environment_manifest(),
        "commands": commands,
        "security_requirement_ids": SECURITY_REQUIREMENTS,
        "implemented_contracts": IMPLEMENTED,
        "verification_evidence": {
            "exact_disclosure_and_packet_matrix": local_pass,
            "secret_redaction_and_injection_matrix": local_pass,
            "source_policy_and_expiry_drift_matrix": local_pass,
            "prohibited_action_receipt_matrix": local_pass,
            "authenticated_host_transport": local_pass,
            "independent_extension_digest_checks": local_pass,
            "complete_product_local_gate": local_pass,
            "canonical_session_composition_contract": local_pass,
            "installed_production_session_activation": False,
            "installed_native_vscode_workflow": False,
            "live_handoff_zero_egress_observation": False,
            "linux_native_accessibility": False,
            "windows_native_accessibility": False,
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
    for field in (
        "installed_production_session_activation", "installed_native_vscode_workflow",
        "live_handoff_zero_egress_observation", "linux_native_accessibility",
        "windows_native_accessibility", "macos_native_accessibility", "independent_review",
    ):
        if verification.get(field) is not False:
            failures.append(f"verification overclaim: {field}")
    for field in (
        "automatic_external_delivery", "codex_invocation", "clipboard_or_interface_control",
        "installed_production_session_activation",
    ):
        if report.get("implemented_contracts", {}).get(field) is not False:
            failures.append(f"implementation overclaim: {field}")
    if verification.get("canonical_session_composition_contract") is not True:
        failures.append("canonical session composition evidence missing")
    if report.get("implemented_contracts", {}).get(
            "canonical_session_composition_contract") is not True:
        failures.append("canonical session composition contract missing")
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
