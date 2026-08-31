#!/usr/bin/env python3
"""Validate and retain the source-only macOS workspace bookmark lifecycle."""

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
    "macos-workspace-bookmark-source-contract.json"
)
SOURCE_PATHS = (
    "platforms/macos/Configuration/KernelHost.contract-fixture.entitlements",
    "platforms/macos/README.md",
    "platforms/macos/Sources/AgentMageMacOSPlatform/MacOSWorkspaceBookmarks.swift",
    "platforms/macos/Sources/AgentMageMacOSPlatform/PlatformBoundary.swift",
    "platforms/macos/Tests/AgentMageMacOSPlatformTests/MacOSWorkspaceBookmarksTests.swift",
    "platforms/macos/Tests/AgentMageMacOSPlatformTests/PlatformBoundaryTests.swift",
    "scripts/macos_workspace_bookmark_source_contract.py",
    "tests/test_macos_workspace_bookmark_source_contract.py",
)
BOOKMARK_FAILURES = (
    "macos.workspace.invalid-bookmark-identifier",
    "macos.workspace.picker-cancelled",
    "macos.workspace.wrong-selection-count",
    "macos.workspace.non-file-url",
    "macos.workspace.not-directory",
    "macos.workspace.symbolic-link",
    "macos.workspace.alias-file",
    "macos.workspace.resource-identity-unavailable",
    "macos.workspace.resource-identity-changed",
    "macos.workspace.volume-identity-changed",
    "macos.workspace.noncanonical-name",
    "macos.workspace.case-collision",
    "macos.workspace.resource-limit-exceeded",
    "macos.workspace.bookmark-creation-failed",
    "macos.workspace.malformed-bookmark-record",
    "macos.workspace.duplicate-bookmark",
    "macos.workspace.bookmark-not-found",
    "macos.workspace.bookmark-load-failed",
    "macos.workspace.bookmark-resolution-failed",
    "macos.workspace.stale-bookmark-refresh-failed",
    "macos.workspace.security-scope-denied",
    "macos.workspace.bookmark-revocation-failed",
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

    source = (
        root
        / "platforms/macos/Sources/AgentMageMacOSPlatform/MacOSWorkspaceBookmarks.swift"
    ).read_text(encoding="utf-8")
    tests = (
        root
        / "platforms/macos/Tests/AgentMageMacOSPlatformTests/MacOSWorkspaceBookmarksTests.swift"
    ).read_text(encoding="utf-8")
    boundary = (
        root
        / "platforms/macos/Sources/AgentMageMacOSPlatform/PlatformBoundary.swift"
    ).read_text(encoding="utf-8")
    readme = (root / "platforms/macos/README.md").read_text(encoding="utf-8")

    required_terms = (
        "import AppKit",
        "NSOpenPanel()",
        "panel.canChooseDirectories = true",
        "panel.canChooseFiles = false",
        "panel.allowsMultipleSelection = false",
        "panel.resolvesAliases = false",
        ".withSecurityScope, .securityScopeAllowOnlyReadAccess",
        "relativeTo: nil",
        ".withoutUI",
        ".withoutMounting",
        ".withoutImplicitStartAccessing",
        "startAccessingSecurityScopedResource()",
        "stopAccessingSecurityScopedResource()",
        "kSecClassGenericPassword",
        "kSecAttrAccessGroup",
        "kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly",
        "SecItemAdd",
        "SecItemCopyMatching",
        "SecItemUpdate",
        "SecItemDelete",
        ".fileResourceIdentifierKey",
        ".volumeIdentifierKey",
        ".isAliasFileKey",
        ".isSymbolicLinkKey",
        "precomposedStringWithCanonicalMapping",
        "folding(options: [.caseInsensitive]",
    )
    for term in required_terms:
        if term not in source:
            failures.append(f"workspace bookmark source missing required term: {term}")
    exact_counts = {
        ".withSecurityScope, .securityScopeAllowOnlyReadAccess": 2,
        "relativeTo: nil": 3,
        "stopAccessingSecurityScopedResource()": 2,
    }
    for term, count in exact_counts.items():
        if source.count(term) != count:
            failures.append(f"workspace bookmark source term count changed: {term}")
    failure_enum = source.split(
        "public enum MacOSWorkspaceBookmarkFailure", 1
    )[1].split("struct MacOSWorkspaceObservation", 1)[0]
    for refusal in BOOKMARK_FAILURES:
        if failure_enum.count(f'"{refusal}"') != 1:
            failures.append(f"workspace bookmark refusal is not exact: {refusal}")
    if "#expect(cases.count == 11)" not in tests:
        failures.append("workspace observation mutation matrix is not closed")
    if tests.count("cases.append((item, .") != 11:
        failures.append("workspace observation mutations are incomplete")
    if "MacOSWorkspaceBookmarkFailure.allCases.count == 22" not in tests:
        failures.append("workspace failure taxonomy count is not tested")
    if 'workspaceBookmarkSourceStatus = "implemented-source-unverified"' not in boundary:
        failures.append("workspace bookmark source status is not explicitly unverified")
    for statement in (
        "app-scoped read-only security-scoped bookmarks",
        "no native picker, Keychain, bookmark, move,",
        "Native compilation, signing, execution, and support remain `BLOCKED-MACOS`",
    ):
        if statement not in readme:
            failures.append(f"README missing bookmark non-promotion statement: {statement}")

    entitlement_path = (
        root
        / "platforms/macos/Configuration/KernelHost.contract-fixture.entitlements"
    )
    entitlements = plistlib.loads(entitlement_path.read_bytes())
    if entitlements.get("com.apple.security.files.bookmarks.app-scope") is not True:
        failures.append("kernel-host app-scoped bookmark entitlement is missing")
    if entitlements.get("com.apple.security.files.user-selected.read-only") is not True:
        failures.append("kernel-host read-only picker entitlement is missing")
    if "com.apple.security.files.user-selected.read-write" in entitlements:
        failures.append("kernel-host bookmark fixture grants workspace write authority")
    return failures


def resolve_revision(source_revision: str, root: Path = ROOT) -> tuple[str, str]:
    revision = str(git("rev-parse", source_revision, root=root))
    tree = str(git("rev-parse", f"{revision}^{{tree}}", root=root))
    for relative in SOURCE_PATHS:
        committed = git("show", f"{revision}:{relative}", root=root, binary=True)
        if committed != (root / relative).read_bytes():
            raise ValueError(f"reviewed macOS workspace bookmark source changed: {relative}")
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
        "record_type": "macos-workspace-bookmark-source-contract",
        "task_id": "8.1.1.4",
        "related_open_task_id": "6.1.1.6",
        "status": "partial-source-only-blocked-macos",
        "source_revision": revision,
        "source_tree": tree,
        "source_files": [
            {"path": relative, "sha256": sha256_file(root / relative)}
            for relative in SOURCE_PATHS
        ],
        "contract": {
            "failure_count": len(BOOKMARK_FAILURES),
            "maximum_bookmark_bytes": 64 * 1024,
            "maximum_root_entries": 4_096,
            "picker_selection_count": 1,
            "picker_directories_only": True,
            "picker_resolves_aliases": False,
            "bookmark_scope": "app-scoped",
            "bookmark_authority": "read-only",
            "bookmark_store": "keychain-access-group",
            "resolution_ui_allowed": False,
            "resolution_mounting_allowed": False,
            "stale_bookmark_refresh_after_revalidation": True,
            "security_scope_start_stop_balanced": True,
        },
        "execution": {
            "swift_build_performed": False,
            "swift_tests_performed": False,
            "apple_silicon_execution_performed": False,
            "native_picker_executed": False,
            "native_keychain_executed": False,
            "native_bookmark_created": False,
            "native_bookmark_resolved": False,
            "native_stale_refresh_executed": False,
            "native_revocation_executed": False,
        },
        "claims": {
            "task_complete": False,
            "cross_story_path_task_complete": False,
            "workspace_write_authority": False,
            "macos_support": False,
            "package_or_release": False,
        },
        "remaining_blockers": [
            "Apple Silicon Swift 6 build and tests have not run.",
            "NSOpenPanel and Powerbox selection have not executed in App Sandbox.",
            "Keychain insert, load, stale replacement, and revocation have not executed.",
            "Native bookmark move, reboot, alias, collision, and mount-change campaigns have not run.",
            "The shared Rust path adapter is not integrated with the resolved macOS scope.",
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
        print(f"macOS workspace bookmark source contract failed: {error}")
        return 1
    if arguments.write:
        REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
        REPORT_PATH.write_bytes(expected)
    elif REPORT_PATH.read_bytes() != expected:
        print("macOS workspace bookmark source report is missing or stale")
        return 1
    print("macOS workspace bookmark source contract passed without native promotion")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
