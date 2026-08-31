#!/usr/bin/env python3
"""Validate and retain the source-only macOS stateless XPC tool-helper boundary."""

from __future__ import annotations

import argparse
import hashlib
import json
import plistlib
import subprocess
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
REPORT_PATH = ROOT / (
    "artifacts/sprints/sprint-8/story-8.1/"
    "macos-tool-helper-source-contract.json"
)
SOURCE_PATHS = (
    "platforms/macos/Configuration/XPCToolHelper.contract-fixture.entitlements",
    "platforms/macos/README.md",
    "platforms/macos/Sources/AgentMageMacOSPlatform/MacOSToolHelperProtocol.swift",
    "platforms/macos/Sources/AgentMageMacOSPlatform/MacOSToolHelperService.swift",
    "platforms/macos/Sources/AgentMageMacOSPlatform/PlatformBoundary.swift",
    "platforms/macos/Tests/AgentMageMacOSPlatformTests/MacOSToolHelperTests.swift",
    "platforms/macos/Tests/AgentMageMacOSPlatformTests/PlatformBoundaryTests.swift",
    "release/platform-manifests/macos/v1/contract-fixture.json",
    "scripts/macos_tool_helper_source_contract.py",
    "tests/test_macos_tool_helper_source_contract.py",
)
TOOL_HELPER_FAILURES = (
    "macos.tool-helper.malformed-envelope",
    "macos.tool-helper.unsupported-schema",
    "macos.tool-helper.invalid-identity",
    "macos.tool-helper.grant-not-consumed",
    "macos.tool-helper.wrong-operation",
    "macos.tool-helper.invalid-tool",
    "macos.tool-helper.invalid-bookmark",
    "macos.tool-helper.request-limit-exceeded",
    "macos.tool-helper.request-digest-mismatch",
    "macos.tool-helper.invalid-deadline",
    "macos.tool-helper.expired",
    "macos.tool-helper.replay",
    "macos.tool-helper.result-limit-exceeded",
    "macos.tool-helper.result-encoding-failed",
    "macos.tool-helper.helper-identity-rejected",
    "macos.tool-helper.host-identity-rejected",
    "macos.tool-helper.connection-rejected",
    "macos.tool-helper.resource-limit-failed",
    "macos.tool-helper.scratch-creation-failed",
    "macos.tool-helper.scratch-validation-failed",
    "macos.tool-helper.scratch-cleanup-failed",
    "macos.tool-helper.bookmark-resolution-failed",
    "macos.tool-helper.stale-bookmark",
    "macos.tool-helper.security-scope-denied",
    "macos.tool-helper.workspace-validation-failed",
    "macos.tool-helper.workspace-descriptor-failed",
    "macos.tool-helper.executor-failed",
    "macos.tool-helper.timeout",
    "macos.tool-helper.cancelled",
)


def git(*arguments: str, root: Path = ROOT, binary: bool = False) -> str | bytes:
    output = subprocess.run(
        ["git", *arguments], cwd=root, check=True, capture_output=True
    ).stdout
    return output if binary else output.decode().strip()


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def validate_sources(root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    for relative in SOURCE_PATHS:
        if not (root / relative).is_file():
            failures.append(f"missing source input: {relative}")
    if failures:
        return failures

    protocol = (
        root
        / "platforms/macos/Sources/AgentMageMacOSPlatform/MacOSToolHelperProtocol.swift"
    ).read_text(encoding="utf-8")
    service = (
        root
        / "platforms/macos/Sources/AgentMageMacOSPlatform/MacOSToolHelperService.swift"
    ).read_text(encoding="utf-8")
    tests = (
        root
        / "platforms/macos/Tests/AgentMageMacOSPlatformTests/MacOSToolHelperTests.swift"
    ).read_text(encoding="utf-8")
    boundary = (
        root
        / "platforms/macos/Sources/AgentMageMacOSPlatform/PlatformBoundary.swift"
    ).read_text(encoding="utf-8")
    readme = (root / "platforms/macos/README.md").read_text(encoding="utf-8")
    manifest = json.loads(
        (root / "release/platform-manifests/macos/v1/contract-fixture.json").read_text(
            encoding="utf-8"
        )
    )

    protocol_terms = (
        'observation.grantState == "consumed"',
        'observation.operation == "workspace_read"',
        "Set(dictionary.keys) == keys",
        "agentMageMacOSToolRequestMaximumBytes = 64 * 1024",
        "agentMageMacOSToolResultMaximumBytes = 4 * 1024 * 1024",
        "agentMageMacOSToolMaximumDeadlineMilliseconds: UInt64 = 15_000",
        "agentMageMacOSToolScratchMaximumBytes: UInt64 = 16 * 1024 * 1024",
        "agentMageMacOSToolScratchMaximumEntries = 256",
        "private var consumed = false",
        "throw MacOSToolHelperFailure.replay",
        "SHA256.hash(data: data)",
    )
    for term in protocol_terms:
        if term not in protocol:
            failures.append(f"tool-helper protocol missing required term: {term}")

    service_terms = (
        "NSXPCListener.service()",
        "NSXPCInterface(with: MacOSToolXPCProtocol.self)",
        "SecCodeCopySelf",
        "SecCodeCopyGuestWithAttributes",
        "kSecGuestAttributePid",
        "SecRequirementCreateWithString",
        "SecCodeCheckValidity",
        "connection.processIdentifier",
        "connection.effectiveUserIdentifier",
        "connection.effectiveGroupIdentifier",
        ".withSecurityScope",
        ".withoutUI",
        ".withoutMounting",
        ".withoutImplicitStartAccessing",
        "startAccessingSecurityScopedResource()",
        "stopAccessingSecurityScopedResource()",
        "O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC",
        "mkdtemp",
        "chmod(url.path, S_IRWXU)",
        "lstat(url.path",
        "RLIMIT_CORE",
        "RLIMIT_NOFILE",
        "RLIMIT_NPROC",
        "RLIMIT_CPU",
        "RLIMIT_AS",
        "RLIMIT_FSIZE",
        "try scratch.validateBudget()",
        "try scratch.closeAndRemove()",
        "_exit(124)",
        "workspaceRootFileDescriptor: workspace.rootFileDescriptor",
        "scratchFileDescriptor: scratch.fileDescriptor",
    )
    for term in service_terms:
        if term not in service:
            failures.append(f"tool-helper service missing required term: {term}")
    exact_counts = {
        "O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC": 2,
        "try connectionGate.consume()": 1,
        "try gate.consume()": 1,
        "_exit(124)": 1,
        "_exit(125)": 2,
    }
    for term, count in exact_counts.items():
        if service.count(term) != count:
            failures.append(f"tool-helper service term count changed: {term}")
    for prohibited in (
        "URLSession",
        "NWConnection",
        "Network.framework",
        "workspace_write",
        "SecItemCopyMatching",
    ):
        if prohibited in service:
            failures.append(f"tool-helper service contains prohibited authority: {prohibited}")

    failure_enum = protocol.split("public enum MacOSToolHelperFailure", 1)[1].split(
        "public struct MacOSToolInvocation", 1
    )[0]
    for refusal in TOOL_HELPER_FAILURES:
        if failure_enum.count(f'"{refusal}"') != 1:
            failures.append(f"tool-helper refusal is not exact: {refusal}")
    if "MacOSToolHelperFailure.allCases.count == 29" not in tests:
        failures.append("tool-helper failure taxonomy count is not tested")
    if "#expect(cases.count == 13)" not in tests:
        failures.append("tool-helper invocation mutation matrix is not closed")
    if tests.count("cases.append((item, .") != 13:
        failures.append("tool-helper invocation mutations are incomplete")
    if 'toolHelperSourceStatus = "implemented-source-unverified"' not in boundary:
        failures.append("tool-helper source status is not explicitly unverified")

    entitlement_path = (
        root
        / "platforms/macos/Configuration/XPCToolHelper.contract-fixture.entitlements"
    )
    entitlements = plistlib.loads(entitlement_path.read_bytes())
    expected_entitlements = {
        "com.apple.security.app-sandbox": True,
        "com.apple.security.files.bookmarks.app-scope": True,
    }
    if entitlements != expected_entitlements:
        failures.append("XPC helper entitlement closure is not the exact two-key sandbox")
    if manifest["entitlements"]["xpc_tool_helper"] != list(expected_entitlements):
        failures.append("release manifest XPC helper entitlement closure changed")
    for prohibited in (
        "com.apple.security.network.client",
        "com.apple.security.network.server",
        "com.apple.security.files.user-selected.read-write",
        "com.apple.security.application-groups",
        "com.apple.security.inherit",
        "keychain-access-groups",
    ):
        if prohibited in entitlements:
            failures.append(f"XPC helper entitlement grants prohibited authority: {prohibited}")

    for statement in (
        "already-consumed `WorkspaceRead` grant",
        "deliberately does not use sandbox inheritance, an App Group, network,",
        "no signed XPC target,",
        "Native compilation, signing, execution, and support remain `BLOCKED-MACOS`",
    ):
        if statement not in readme:
            failures.append(f"README missing tool-helper non-promotion statement: {statement}")
    return failures


def resolve_revision(source_revision: str, root: Path = ROOT) -> tuple[str, str]:
    revision = str(git("rev-parse", source_revision, root=root))
    tree = str(git("rev-parse", f"{revision}^{{tree}}", root=root))
    for relative in SOURCE_PATHS:
        committed = git("show", f"{revision}:{relative}", root=root, binary=True)
        if committed != (root / relative).read_bytes():
            raise ValueError(f"reviewed macOS tool-helper source changed: {relative}")
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
        "record_type": "macos-tool-helper-source-contract",
        "task_id": "8.1.1.5",
        "related_open_task_id": "16.1.1.5",
        "status": "partial-source-only-blocked-macos",
        "source_revision": revision,
        "source_tree": tree,
        "source_files": [
            {"path": relative, "sha256": sha256_file(root / relative)}
            for relative in SOURCE_PATHS
        ],
        "contract": {
            "failure_count": len(TOOL_HELPER_FAILURES),
            "xpc_connections_per_process": 1,
            "invocations_per_connection": 1,
            "grant_state": "consumed",
            "grant_operation": "workspace_read",
            "bookmarks_per_invocation": 1,
            "bookmark_authority": "read-only-app-scoped",
            "request_maximum_bytes": 64 * 1024,
            "result_maximum_bytes": 4 * 1024 * 1024,
            "wall_deadline_milliseconds": 15_000,
            "scratch_maximum_bytes": 16 * 1024 * 1024,
            "scratch_maximum_entries": 256,
            "resource_limits": [
                "address-space",
                "core-file",
                "cpu-time",
                "file-size",
                "open-files",
                "process-count",
            ],
            "helper_entitlements": [
                "com.apple.security.app-sandbox",
                "com.apple.security.files.bookmarks.app-scope",
            ],
            "network_entitlement": False,
            "workspace_write_entitlement": False,
            "keychain_entitlement": False,
            "app_group_entitlement": False,
            "sandbox_inheritance_entitlement": False,
        },
        "execution": {
            "swift_build_performed": False,
            "swift_tests_performed": False,
            "apple_silicon_execution_performed": False,
            "signed_xpc_service_executed": False,
            "app_sandbox_observed": False,
            "native_bookmark_resolved": False,
            "native_resource_limits_observed": False,
            "native_attack_campaign_executed": False,
            "native_lifecycle_campaign_executed": False,
        },
        "claims": {
            "task_complete": False,
            "cross_story_tool_task_complete": False,
            "workspace_write_authority": False,
            "network_authority": False,
            "macos_support": False,
            "package_or_release": False,
        },
        "remaining_blockers": [
            "Apple Silicon Swift 6 build and tests have not run.",
            "No signed embedded XPC service target or release-derived identity exists.",
            "No native XPC host connection has exercised the designated-requirement admission.",
            "The shared Rust read-only executor is not composed with the XPC descriptor boundary.",
            "Native App Sandbox, bookmark, resource, attack, timeout, crash, and cleanup campaigns have not run.",
            "Independent critical-boundary review has not been retained.",
            "Physical MacBook Pro M5 qualification has not run.",
        ],
    }


def canonical_bytes(report: dict[str, Any]) -> bytes:
    return (json.dumps(report, indent=2, sort_keys=True) + "\n").encode()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    try:
        if arguments.write:
            revision = arguments.source_revision
        else:
            retained = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
            revision = retained.get("source_revision")
            if not isinstance(revision, str):
                raise ValueError("retained report has no source revision")
        expected = canonical_bytes(build_report(source_revision=revision))
    except (OSError, subprocess.CalledProcessError, ValueError, json.JSONDecodeError) as error:
        print(f"macOS tool-helper source contract failed: {error}")
        return 1
    if arguments.write:
        REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
        REPORT_PATH.write_bytes(expected)
    elif REPORT_PATH.read_bytes() != expected:
        print("macOS tool-helper source report is missing or stale")
        return 1
    print("macOS stateless XPC tool-helper source contract passed without native promotion")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
