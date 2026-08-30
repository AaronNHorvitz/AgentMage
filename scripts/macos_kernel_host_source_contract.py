#!/usr/bin/env python3
"""Validate and retain the source-only macOS kernel-host admission contract."""

from __future__ import annotations

import argparse
import hashlib
import json
import plistlib
import subprocess
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
REPORT_PATH = ROOT / "artifacts/sprints/sprint-8/story-8.1/macos-kernel-host-source-contract.json"
SOURCE_PATHS = (
    "platforms/macos/Configuration/KernelHost.contract-fixture.entitlements",
    "platforms/macos/README.md",
    "platforms/macos/Sources/AgentMageMacOSPlatform/KernelHostAdmission.swift",
    "platforms/macos/Sources/AgentMageMacOSPlatform/PlatformBoundary.swift",
    "platforms/macos/Tests/AgentMageMacOSPlatformTests/KernelHostAdmissionTests.swift",
    "platforms/macos/Tests/AgentMageMacOSPlatformTests/PlatformBoundaryTests.swift",
    "scripts/macos_kernel_host_source_contract.py",
    "tests/test_macos_kernel_host_source_contract.py",
)
EXPECTED_ENTITLEMENTS: dict[str, Any] = {
    "com.apple.security.app-sandbox": True,
    "com.apple.security.application-groups": [
        "group.com.example.agentmage.contractfixture"
    ],
    "com.apple.security.files.bookmarks.app-scope": True,
    "com.apple.security.files.user-selected.read-only": True,
    "keychain-access-groups": [
        "AAAAAAAAAA.com.example.agentmage.contractfixture.keys"
    ],
}
EXPECTED_FAILURES = (
    "macos-host.wrong-architecture",
    "macos-host.unsupported-operating-system",
    "macos-host.invalid-code-signature",
    "macos-host.hardened-runtime-missing",
    "macos-host.wrong-bundle-identifier",
    "macos-host.wrong-team-identifier",
    "macos-host.app-sandbox-missing",
    "macos-host.wrong-app-group",
    "macos-host.app-scoped-bookmarks-missing",
    "macos-host.user-selected-read-only-missing",
    "macos-host.wrong-keychain-access-group",
    "macos-host.entitlement-closure-mismatch",
)
REQUIRED_SOURCE_TERMS = (
    "SecCodeCopySelf",
    "SecCodeCheckValidity",
    "SecCodeCopySigningInformation",
    "kSecCodeInfoEntitlementsDict",
    "kSecCodeInfoRuntimeVersion",
    "kSecCodeInfoIdentifier",
    "kSecCodeInfoTeamIdentifier",
    "#if arch(arm64)",
    "operatingSystem.majorVersion >= 15",
    "unexpectedEntitlementKeys",
)
FORBIDDEN_ENTITLEMENTS = (
    "com.apple.security.network.client",
    "com.apple.security.network.server",
    "com.apple.security.files.user-selected.read-write",
    "com.apple.security.files.all",
    "com.apple.security.cs.allow-jit",
    "com.apple.security.cs.disable-library-validation",
)


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git(*arguments: str, root: Path = ROOT) -> str:
    return subprocess.run(
        ["git", *arguments],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()


def validate_sources(root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    for relative in SOURCE_PATHS:
        if not (root / relative).is_file():
            failures.append(f"missing source input: {relative}")
    if failures:
        return failures

    source = (
        root
        / "platforms/macos/Sources/AgentMageMacOSPlatform/KernelHostAdmission.swift"
    ).read_text(encoding="utf-8")
    tests = (
        root
        / "platforms/macos/Tests/AgentMageMacOSPlatformTests/KernelHostAdmissionTests.swift"
    ).read_text(encoding="utf-8")
    boundary = (
        root
        / "platforms/macos/Sources/AgentMageMacOSPlatform/PlatformBoundary.swift"
    ).read_text(encoding="utf-8")
    readme = (root / "platforms/macos/README.md").read_text(encoding="utf-8")

    for term in REQUIRED_SOURCE_TERMS:
        if term not in source:
            failures.append(f"kernel-host source missing required term: {term}")
    refusal_enum = source.split(
        "public enum KernelHostAdmissionFailure", 1
    )[1].split("public struct VerifiedKernelHostIdentity", 1)[0]
    for failure in EXPECTED_FAILURES:
        if refusal_enum.count(f'"{failure}"') != 1:
            failures.append(f"startup refusal is not declared exactly once: {failure}")
    if "cases.count == KernelHostAdmissionFailure.allCases.count" not in tests:
        failures.append("Swift mutation matrix is not closed over every refusal class")
    if tests.count("cases.append((item, .") != len(EXPECTED_FAILURES):
        failures.append("Swift mutation matrix does not independently exercise each refusal")
    if 'implementationStatus = "blocked-macos"' not in boundary:
        failures.append("macOS implementation status was promoted without native evidence")
    if 'kernelHostSourceStatus = "implemented-source-unverified"' not in boundary:
        failures.append("kernel-host source status is not explicitly unverified")
    for statement in (
        "Native compilation, signing, execution, and support remain `BLOCKED-MACOS`",
        "not a signing or release input",
    ):
        if statement not in readme:
            failures.append(f"README missing non-promotion statement: {statement}")

    entitlement_path = (
        root
        / "platforms/macos/Configuration/KernelHost.contract-fixture.entitlements"
    )
    try:
        entitlements = plistlib.loads(entitlement_path.read_bytes())
    except (OSError, plistlib.InvalidFileException) as error:
        failures.append(f"cannot parse kernel-host entitlement fixture: {error}")
        entitlements = None
    if entitlements != EXPECTED_ENTITLEMENTS:
        failures.append("kernel-host entitlement fixture is not the exact minimal closure")
    entitlement_text = entitlement_path.read_text(encoding="utf-8")
    for forbidden in FORBIDDEN_ENTITLEMENTS:
        if forbidden in entitlement_text:
            failures.append(f"kernel-host entitlement fixture grants forbidden authority: {forbidden}")
    if "com.example" not in entitlement_text or "AAAAAAAAAA" not in entitlement_text:
        failures.append("kernel-host entitlement fixture is not visibly synthetic")
    return failures


def resolve_revision(source_revision: str, root: Path = ROOT) -> tuple[str, str]:
    revision = git("rev-parse", source_revision, root=root)
    tree = git("rev-parse", f"{revision}^{{tree}}", root=root)
    for relative in SOURCE_PATHS:
        committed = subprocess.run(
            ["git", "show", f"{revision}:{relative}"],
            cwd=root,
            check=True,
            capture_output=True,
        ).stdout
        if committed != (root / relative).read_bytes():
            raise ValueError(f"reviewed macOS kernel-host source changed: {relative}")
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
        "record_type": "macos-kernel-host-source-contract",
        "task_id": "8.1.1.1",
        "status": "partial-source-only-blocked-macos",
        "source_revision": revision,
        "source_tree": tree,
        "source_files": [
            {"path": relative, "sha256": sha256_file(root / relative)}
            for relative in SOURCE_PATHS
        ],
        "contract": {
            "minimum_macos_major_version": 15,
            "architecture": "arm64",
            "startup_refusal_count": len(EXPECTED_FAILURES),
            "requested_entitlement_count": len(EXPECTED_ENTITLEMENTS),
            "requested_entitlements": sorted(EXPECTED_ENTITLEMENTS),
            "forbidden_entitlements": list(FORBIDDEN_ENTITLEMENTS),
            "verified_identity_constructible_only_after_admission": True,
            "security_framework_observer_present": True,
        },
        "execution": {
            "swift_toolchain_available": False,
            "swift_build_performed": False,
            "swift_tests_performed": False,
            "apple_silicon_execution_performed": False,
            "signing_performed": False,
            "app_sandbox_execution_performed": False,
        },
        "claims": {
            "macos_implementation_complete": False,
            "macos_support": False,
            "package_or_release": False,
        },
        "remaining_blockers": [
            "Apple Silicon Swift 6 build and test have not run.",
            "Developer ID signing and Hardened Runtime observation have not run.",
            "App Sandbox enforcement and entitlement inspection have not run.",
            "The shared Rust kernel has not been integrated into a signed macOS host executable.",
            "Physical MacBook Pro M5 qualification has not run.",
        ],
    }


def canonical_bytes(report: dict[str, Any]) -> bytes:
    return (json.dumps(report, indent=2, sort_keys=True) + "\n").encode("utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    try:
        if arguments.write:
            source_revision = arguments.source_revision
        else:
            retained = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
            source_revision = retained.get("source_revision")
            if not isinstance(source_revision, str):
                raise ValueError("retained report has no source revision")
        expected = canonical_bytes(build_report(source_revision=source_revision))
    except (OSError, subprocess.CalledProcessError, ValueError) as error:
        print(f"macOS kernel-host source contract failed: {error}")
        return 1
    if arguments.write:
        REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
        REPORT_PATH.write_bytes(expected)
    elif REPORT_PATH.read_bytes() != expected:
        print("macOS kernel-host source report is missing or stale")
        return 1
    print("macOS kernel-host source contract passed without native or release promotion")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
