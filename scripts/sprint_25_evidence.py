#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 25 readiness evidence."""

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
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-25/local-evidence-report.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
SOURCE_PATHS: Final = (
    "docs/release/README.md",
    "docs/release/capability-matrix.md",
    "docs/release/maintainer-guide.md",
    "docs/release/operator-guide.md",
    "docs/release/release-notes-v0.1.0-draft.md",
    "docs/support/incident-response-v0.1.md",
    "docs/support/manual-patch-and-disable-v0.1.md",
    "docs/verification/sprint-25-local-results.md",
    "shells/host/src/linux_bootstrap.rs",
    "shells/host/src/package_verify.rs",
    "scripts/package_candidate.py",
    "scripts/package_lifecycle.py",
    "scripts/package_release_lifecycle.py",
    "scripts/sprint_25_evidence.py",
    "tests/test_sprint_25_evidence.py",
)
COMMANDS: Final = (
    ("product-gate", ("npm", "run", "product:check")),
    ("documentation-gate", ("npm", "run", "docs:check")),
    ("package-lifecycle", ("npm", "run", "phase11:check")),
    ("release-signing-mechanics", ("npm", "run", "release-signing:test")),
    ("manual-patch-metadata", ("python3", "scripts/manual_patch_metadata.py")),
    ("manual-patch-verifier", ("python3", "scripts/manual_patch_verifier.py")),
    ("emergency-disable-policy", ("python3", "scripts/emergency_disable_policy.py")),
    (
        "patch-schema-tests",
        (
            "node", "--test", "tests/test_manual_patch_schema.mjs",
            "tests/test_emergency_disable_schema.mjs",
        ),
    ),
    (
        "patch-behavior-tests",
        (
            "python3", "-m", "unittest", "tests.test_manual_patch_metadata",
            "tests.test_manual_patch_verifier", "tests.test_emergency_disable_policy",
        ),
    ),
    ("strict-local-source", ("python3", "scripts/strict_local_source_audit.py")),
    ("supply-chain", ("python3", "scripts/supply_chain.py")),
    ("evidence-tests", ("python3", "-m", "unittest", "tests.test_sprint_25_evidence")),
)
REQUIREMENT_FAMILIES: Final = [
    "SR-GOV-*", "SR-PLT-*", "SR-ACC-*", "SR-DAT-*", "SR-NET-*",
    "SR-SUP-*", "SR-AI-*", "SR-OPS-*", "SR-TST-*", "SR-CIV-*",
]
BLOCKERS: Final = [
    {"code": "OWNING-SPRINT-PRODUCTION-EVIDENCE-INCOMPLETE", "owner": "25.1.1.1"},
    {"code": "PRODUCTION-SIGNER-AND-TRUST-ROOT-ABSENT", "owner": "25.1.2.1"},
    {"code": "SIGNED-REFERENCE-PLATFORM-PACKAGES-ABSENT", "owner": "25.1.2.1"},
    {"code": "INSTALLED-NATIVE-VSCODE-WORKFLOW-INCOMPLETE", "owner": "25.1.2.2"},
    {"code": "RELEASE-ACCEPTANCE-BUNDLE-INCOMPLETE", "owner": "25.1.2.3"},
    {"code": "FINAL-PUBLISHED-RELEASE-NOTES-ABSENT", "owner": "25.1.2.4"},
    {"code": "FEDORA-THREE-RUN-CLEAN-LIFECYCLE-ABSENT", "owner": "25.1.3.4"},
    {"code": "UBUNTU-THREE-RUN-CLEAN-LIFECYCLE-ABSENT", "owner": "25.1.3.4"},
    {"code": "MACOS-M5-THREE-RUN-CLEAN-LIFECYCLE-ABSENT", "owner": "25.1.3.4"},
    {"code": "COMPLETE-RV-01-THROUGH-RV-22-EXECUTION-ABSENT", "owner": "25.1.3.6"},
    {"code": "INDEPENDENT-RV-21-TABLETOP-ABSENT", "owner": "25.2.2.1"},
    {"code": "PRODUCTION-RV-22-EXERCISE-ABSENT", "owner": "25.2.2.2"},
    {"code": "MANUAL-FUZZ-CAMPAIGN-DEFERRED", "owner": "166.1.2.4"},
    {"code": "INDEPENDENT-RELEASE-REVIEW-ABSENT", "owner": "25.1.3.6"},
]
IMPLEMENTED: Final = {
    "operator_and_maintainer_guides": True,
    "capability_and_limitation_matrix": True,
    "draft_release_notes": True,
    "incident_and_support_runbooks": True,
    "deterministic_unsigned_candidate": True,
    "detached_signature_mechanics": True,
    "synthetic_manual_patch_and_disable_fixtures": True,
    "complete_seven_payload_linux_manifest_closure": True,
    "production_signer_and_trust_root": False,
    "signed_release_packages": False,
    "supported_v0_1_release": False,
    "native_cross_platform_acceptance": False,
    "independent_incident_tabletop": False,
    "production_manual_patch_exercise": False,
}


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def git_file(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, check=False,
        capture_output=True, timeout=30,
    )
    if result.returncode != 0:
        raise ValueError(f"committed Sprint 25 source is absent: {path}")
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
                capture_output=True, timeout=1800,
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
        "record_type": "sprint_25_local_readiness_evidence",
        "source_revision": revision,
        "source_sha256": {path: digest(git_file(revision, path)) for path in SOURCE_PATHS},
        "environment": environment_manifest(),
        "commands": commands,
        "requirement_families": REQUIREMENT_FAMILIES,
        "implemented_readiness": IMPLEMENTED,
        "verification_evidence": {
            "complete_local_product_gate": local_pass,
            "documentation_and_traceability_gate": local_pass,
            "deterministic_candidate_lifecycle": local_pass,
            "synthetic_detached_signing_mechanics": local_pass,
            "synthetic_manual_patch_and_disable_controls": local_pass,
            "strict_local_source_and_supply_chain_checks": local_pass,
            "production_signer_and_trust_root": False,
            "signed_reference_platform_packages": False,
            "native_cross_platform_workflow": False,
            "complete_release_verification_protocols": False,
            "independent_incident_tabletop": False,
            "production_manual_patch_exercise": False,
            "independent_release_review": False,
        },
        "blockers": BLOCKERS,
        "summary": {
            "local_readiness_checks_passed": local_pass,
            "sprint_status": "BLOCKED",
            "release_approval": False,
            "supported_release": False,
        },
    }


def validate_report(report: dict[str, Any], verify_current: bool = True) -> list[str]:
    failures: list[str] = []
    revision = str(report.get("source_revision", ""))
    if not REVISION.fullmatch(revision):
        failures.append("source revision invalid")
    if report.get("requirement_families") != REQUIREMENT_FAMILIES:
        failures.append("requirement family mapping drift")
    if report.get("implemented_readiness") != IMPLEMENTED:
        failures.append("implemented readiness drift")
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
    if report.get("summary") != {
        "local_readiness_checks_passed": True,
        "sprint_status": "BLOCKED",
        "release_approval": False,
        "supported_release": False,
    }:
        failures.append("summary overclaim or local failure")
    verification = report.get("verification_evidence", {})
    for field in (
        "production_signer_and_trust_root", "signed_reference_platform_packages",
        "native_cross_platform_workflow", "complete_release_verification_protocols",
        "independent_incident_tabletop", "production_manual_patch_exercise",
        "independent_release_review",
    ):
        if verification.get(field) is not False:
            failures.append(f"verification overclaim: {field}")
    for field in (
        "production_signer_and_trust_root", "signed_release_packages",
        "supported_v0_1_release", "native_cross_platform_acceptance",
        "independent_incident_tabletop", "production_manual_patch_exercise",
    ):
        if report.get("implemented_readiness", {}).get(field) is not False:
            failures.append(f"readiness overclaim: {field}")
    environment = report.get("environment", {})
    expected_environment = {
        "system", "release", "machine", "python", "rustc", "cargo", "node", "npm",
    }
    if set(environment) != expected_environment:
        failures.append("environment manifest drift")
    elif any(
        not isinstance(value, str) or not value or len(value) > 256
        for value in environment.values()
    ):
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
        OUTPUT.write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8",
        )
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
