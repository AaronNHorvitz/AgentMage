#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 40 evidence."""

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
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-40/local-evidence-report.json"
CANDIDATE_DIR: Final = ROOT / "target/sprint-40-v0.3-candidate"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
IGNORED_TESTS: Final = re.compile(
    rb"test result: (?:ok|FAILED)\. \d+ passed; \d+ failed; (\d+) ignored;"
)
SOURCE_PATHS: Final = (
    "release/v0.3-write-pack-manifest.json",
    "scripts/v0_3_write_release_gate.py",
    "tests/test_v0_3_write_release_gate.py",
    "docs/guides/controlled-writes.md",
    "docs/release/v0.3-capability-matrix.md",
    "docs/release/v0.3-acceptance-and-recovery-bundle.md",
    "docs/release/release-notes-v0.3.0-draft.md",
    "docs/verification/sprint-40-local-results.md",
    "scripts/sprint_40_evidence.py",
    "tests/test_sprint_40_evidence.py",
)
COMMANDS: Final = (
    (
        "release-rust-build",
        (
            "cargo", "build", "--release", "-p", "agentmage-host", "-p",
            "agentmage-platform-linux-inference", "--bins", "--locked",
        ),
    ),
    (
        "release-vscode-build",
        ("npm", "run", "build", "--workspace", "@agentmage/vscode-shell"),
    ),
    (
        "unsigned-v0.3-candidate",
        (
            "python3", "scripts/package_candidate.py", "--output",
            "target/sprint-40-v0.3-candidate", "--version", "0.3.0",
        ),
    ),
    (
        "write-approval-tests",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine", "write_approval",
            "--locked",
        ),
    ),
    (
        "write-transaction-tests",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine", "write_transaction",
            "--locked",
        ),
    ),
    (
        "filesystem-contract-tests",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine", "filesystem_control",
            "--locked",
        ),
    ),
    (
        "write-recovery-tests",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine", "write_recovery",
            "--locked",
        ),
    ),
    (
        "linux-filesystem-tests",
        (
            "cargo", "test", "-p", "agentmage-platform-linux", "filesystem_control",
            "--locked",
        ),
    ),
    (
        "knowledge-write-tests",
        ("cargo", "test", "-p", "agentmage-capability-knowledge", "--locked"),
    ),
    (
        "host-knowledge-tests",
        ("cargo", "test", "-p", "agentmage-host", "knowledge_write", "--locked"),
    ),
    ("product-gate", ("npm", "run", "product:check")),
    ("v0.3-release-gate", ("python3", "scripts/v0_3_write_release_gate.py")),
    ("documentation-gate", ("npm", "run", "docs:check")),
    ("supply-chain", ("python3", "scripts/supply_chain.py")),
    ("evidence-tests", ("python3", "-m", "unittest", "tests.test_sprint_40_evidence")),
)
FOCUSED_COMMANDS: Final = (
    "write-approval-tests",
    "write-transaction-tests",
    "filesystem-contract-tests",
    "write-recovery-tests",
    "linux-filesystem-tests",
    "knowledge-write-tests",
    "host-knowledge-tests",
)
SECURITY_REQUIREMENTS: Final = [
    "SR-ACC-001", "SR-ACC-002", "SR-ACC-003", "SR-ACC-004", "SR-ACC-005",
    "SR-ACC-006", "SR-ACC-007", "SR-ACC-008", "SR-DAT-001", "SR-DAT-002",
    "SR-DAT-003", "SR-DAT-004", "SR-DAT-010", "SR-DAT-011", "SR-DAT-012",
    "SR-OPS-001", "SR-OPS-002", "SR-OPS-003", "SR-OPS-004", "SR-OPS-005",
    "SR-OPS-006", "SR-OPS-007", "SR-TST-004", "SR-TST-005", "SR-TST-007",
    "SR-TST-011", "SR-TST-012", "SR-CIV-003", "SR-CIV-004",
]
IMPLEMENTED: Final = {
    "hash_bound_blocked_write_pack_manifest": True,
    "exact_v0_3_capability_delta": True,
    "write_operator_and_recovery_guides": True,
    "v0_3_draft_release_documents": True,
    "unsigned_linux_deb_candidate_exercised": True,
    "unsigned_linux_rpm_candidate_exercised": True,
    "unsigned_vscode_candidate_exercised": True,
    "read_only_effective_profile_preserved": True,
    "prohibited_capability_closure": True,
    "write_profile_product_registered": False,
    "signed_v0_3_packages": False,
    "published_v0_3_packages": False,
    "complete_cross_platform_write_acceptance": False,
    "complete_lifecycle_acceptance": False,
    "independent_release_review": False,
    "manual_fuzzing_complete": False,
    "g_v0_3_closed": False,
}
BLOCKERS: Final = [
    {"code": "UPSTREAM-SPRINTS-35-THROUGH-39-BLOCKED", "owner": "40.1"},
    {"code": "WRITE-PROFILE-NOT-PRODUCT-REGISTERED", "owner": "40.1.1"},
    {"code": "SPRINT-40-NATIVE-CROSS-PLATFORM-WRITE-ACCEPTANCE-ABSENT", "owner": "40.1.3.1"},
    {"code": "SPRINT-40-COMPLETE-LIFECYCLE-CAMPAIGN-ABSENT", "owner": "40.1.3.3"},
    {"code": "TRUSTED-PACKAGE-LAUNCHER-ENVIRONMENT-ABSENT", "owner": "40.1.3.5"},
    {"code": "SIGNED-V0.3-PACKAGES-ABSENT", "owner": "40.1.2.1"},
    {"code": "INDEPENDENT-V0.3-RELEASE-DECISION-ABSENT", "owner": "40.1.3.5"},
    {"code": "MANUAL-FUZZING-DEFERRED", "owner": "SR-TST-004"},
]


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def git_file(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, check=False,
        capture_output=True, timeout=30,
    )
    if result.returncode != 0:
        raise ValueError(f"committed Sprint 40 source is absent: {path}")
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
        "rpmbuild": version("rpmbuild", "--version"),
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
        if identifier in FOCUSED_COMMANDS:
            matches = IGNORED_TESTS.findall(output)
            blocking_skip_count = sum(int(value) for value in matches) if matches else -1
        records.append({
            "id": identifier,
            "argv": list(argv),
            "exit_code": code,
            "output_sha256": digest(output),
            "blocking_skip_count": blocking_skip_count,
        })
    return records


def candidate_artifacts() -> list[dict[str, Any]]:
    expected = (
        ("deb", "agentmage_0.3.0_amd64.deb"),
        ("rpm", "agentmage-0.3.0-1.fc44.x86_64.rpm"),
        ("vsix", "agentmage-vscode-0.3.0.vsix"),
    )
    artifacts = []
    for kind, name in expected:
        path = CANDIDATE_DIR / name
        if not path.is_file() or path.is_symlink() or path.stat().st_size <= 0:
            raise ValueError(f"unsigned candidate artifact unavailable: {kind}")
        artifacts.append({
            "kind": kind,
            "name": name,
            "size": path.stat().st_size,
            "sha256": digest(path.read_bytes()),
            "status": "unsigned-local-candidate",
            "retained_in_repository": False,
            "published": False,
            "signed": False,
        })
    return artifacts


def build_report(revision: str, commands: list[dict[str, Any]], artifacts: list[dict[str, Any]]) -> dict[str, Any]:
    focused = [item for item in commands if item["id"] in FOCUSED_COMMANDS]
    local_pass = (
        all(item["exit_code"] == 0 for item in commands)
        and len(focused) == len(FOCUSED_COMMANDS)
        and all(item.get("blocking_skip_count") == 0 for item in focused)
        and [item.get("kind") for item in artifacts] == ["deb", "rpm", "vsix"]
    )
    return {
        "schema_version": 1,
        "record_type": "sprint_40_local_evidence",
        "source_revision": revision,
        "source_sha256": {path: digest(git_file(revision, path)) for path in SOURCE_PATHS},
        "environment": environment_manifest(),
        "commands": commands,
        "unsigned_candidate_artifacts": artifacts,
        "security_requirement_ids": SECURITY_REQUIREMENTS,
        "implemented_contracts": IMPLEMENTED,
        "verification_evidence": {
            "focused_local_contracts": local_pass,
            "focused_blocking_skip_count": 0 if local_pass else None,
            "upstream_sprints_35_through_39": False,
            "write_profile_product_registration": False,
            "native_cross_platform_write_acceptance": False,
            "complete_lifecycle_acceptance": False,
            "trusted_package_launcher_environment": False,
            "signed_packages": False,
            "independent_release_decision": False,
            "manual_fuzzing": False,
        },
        "blockers": BLOCKERS,
        "summary": {
            "local_v0_3_candidate_contract_passed": local_pass,
            "sprint_status": "BLOCKED",
            "upstream_dependencies_passed": False,
            "write_profile_active": False,
            "cross_platform_acceptance_passed": False,
            "lifecycle_acceptance_passed": False,
            "signed_packages_present": False,
            "independent_release_decision_present": False,
            "manual_fuzzing_complete": False,
            "network_access_enabled": False,
            "g_v0_3_closed": False,
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
    if any(
        item.get("exit_code") != 0
        or not SHA256.fullmatch(str(item.get("output_sha256", "")))
        for item in commands
    ):
        failures.append("command result invalid")
    for identifier in FOCUSED_COMMANDS:
        focused = next((item for item in commands if item.get("id") == identifier), None)
        if focused is None or focused.get("blocking_skip_count") != 0:
            failures.append(f"focused skipped, suppressed, or unavailable check: {identifier}")
    artifacts = report.get("unsigned_candidate_artifacts", [])
    if [item.get("kind") for item in artifacts] != ["deb", "rpm", "vsix"]:
        failures.append("candidate artifact inventory drift")
    for item in artifacts:
        if (
            not SHA256.fullmatch(str(item.get("sha256", "")))
            or not isinstance(item.get("size"), int)
            or item.get("size", 0) <= 0
            or item.get("status") != "unsigned-local-candidate"
            or item.get("retained_in_repository") is not False
            or item.get("published") is not False
            or item.get("signed") is not False
        ):
            failures.append("candidate artifact claim invalid")
    expected_summary = {
        "local_v0_3_candidate_contract_passed": True,
        "sprint_status": "BLOCKED",
        "upstream_dependencies_passed": False,
        "write_profile_active": False,
        "cross_platform_acceptance_passed": False,
        "lifecycle_acceptance_passed": False,
        "signed_packages_present": False,
        "independent_release_decision_present": False,
        "manual_fuzzing_complete": False,
        "network_access_enabled": False,
        "g_v0_3_closed": False,
        "release_approval": False,
    }
    if report.get("summary") != expected_summary:
        failures.append("summary or release claim drift")
    expected_verification = {
        "focused_local_contracts": True,
        "focused_blocking_skip_count": 0,
        "upstream_sprints_35_through_39": False,
        "write_profile_product_registration": False,
        "native_cross_platform_write_acceptance": False,
        "complete_lifecycle_acceptance": False,
        "trusted_package_launcher_environment": False,
        "signed_packages": False,
        "independent_release_decision": False,
        "manual_fuzzing": False,
    }
    if report.get("verification_evidence") != expected_verification:
        failures.append("verification boundary drift")
    sources = report.get("source_sha256", {})
    if set(sources) != set(SOURCE_PATHS) or any(
        not SHA256.fullmatch(str(value)) for value in sources.values()
    ):
        failures.append("source inventory invalid")
    if verify_current and REVISION.fullmatch(revision):
        for path in SOURCE_PATHS:
            try:
                if sources.get(path) != digest(git_file(revision, path)):
                    failures.append(f"source hash drift: {path}")
            except ValueError as error:
                failures.append(str(error))
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--record", action="store_true")
    parser.add_argument("--revision")
    parser.add_argument("--verify", type=Path)
    arguments = parser.parse_args()
    if arguments.verify:
        report = json.loads(arguments.verify.read_text(encoding="utf-8"))
        failures = validate_report(report)
        if failures:
            print("Sprint 40 evidence validation failed:")
            for failure in failures:
                print(f"- {failure}")
            return 1
        print("Sprint 40 evidence validated")
        return 0
    if not arguments.record or not arguments.revision:
        parser.error("--record requires --revision")
    report = build_report(arguments.revision, run_commands(), candidate_artifacts())
    failures = validate_report(report)
    if failures:
        print("Sprint 40 evidence recording failed:")
        for failure in failures:
            print(f"- {failure}")
        return 1
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(OUTPUT.relative_to(ROOT))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
