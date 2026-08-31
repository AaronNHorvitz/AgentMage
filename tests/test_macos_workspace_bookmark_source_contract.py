from __future__ import annotations

import json
import plistlib
import shutil
import tempfile
import unittest
from pathlib import Path

from scripts.macos_workspace_bookmark_source_contract import (
    BOOKMARK_FAILURES,
    ROOT,
    SOURCE_PATHS,
    build_report,
    validate_sources,
)


class MacOSWorkspaceBookmarkSourceContractTests(unittest.TestCase):
    def copy_inputs(self, destination: Path) -> None:
        for relative in SOURCE_PATHS:
            target = destination / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(ROOT / relative, target)

    def mutate(self, relative: str, old: str, new: str) -> list[str]:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.copy_inputs(root)
            path = root / relative
            text = path.read_text(encoding="utf-8")
            self.assertIn(old, text)
            path.write_text(text.replace(old, new, 1), encoding="utf-8")
            return validate_sources(root)

    def test_canonical_sources_pass(self) -> None:
        self.assertEqual(validate_sources(), [])

    def test_report_preserves_native_and_cross_story_blockers(self) -> None:
        report = build_report()
        self.assertEqual(report["status"], "partial-source-only-blocked-macos")
        self.assertEqual(report["related_open_task_id"], "6.1.1.6")
        self.assertEqual(report["contract"]["failure_count"], len(BOOKMARK_FAILURES))
        self.assertTrue(all(value is False for value in report["execution"].values()))
        self.assertTrue(all(value is False for value in report["claims"].values()))
        self.assertEqual(len(report["remaining_blockers"]), 6)
        json.dumps(report)

    def test_read_only_or_app_scope_weakening_is_rejected(self) -> None:
        source = "platforms/macos/Sources/AgentMageMacOSPlatform/MacOSWorkspaceBookmarks.swift"
        failures = self.mutate(
            source,
            ".withSecurityScope, .securityScopeAllowOnlyReadAccess",
            ".withSecurityScope",
        )
        self.assertTrue(any("securityScopeAllowOnlyReadAccess" in item for item in failures))
        failures = self.mutate(source, "relativeTo: nil", "relativeTo: selected")
        self.assertTrue(any("relativeTo: nil" in item for item in failures))

    def test_picker_alias_or_resolution_mounting_weakening_is_rejected(self) -> None:
        source = "platforms/macos/Sources/AgentMageMacOSPlatform/MacOSWorkspaceBookmarks.swift"
        failures = self.mutate(
            source, "panel.resolvesAliases = false", "panel.resolvesAliases = true"
        )
        self.assertTrue(any("resolvesAliases" in item for item in failures))
        failures = self.mutate(source, ".withoutMounting", ".withSecurityScope")
        self.assertTrue(any("withoutMounting" in item for item in failures))

    def test_keychain_or_balanced_scope_removal_is_rejected(self) -> None:
        source = "platforms/macos/Sources/AgentMageMacOSPlatform/MacOSWorkspaceBookmarks.swift"
        failures = self.mutate(source, "SecItemAdd", "removedKeychainInsert")
        self.assertTrue(any("SecItemAdd" in item for item in failures))
        failures = self.mutate(
            source,
            "stopAccessingSecurityScopedResource()",
            "removedStopAccessingSecurityScopedResource()",
        )
        self.assertTrue(any("stopAccessing" in item for item in failures))

    def test_read_write_entitlement_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.copy_inputs(root)
            path = root / (
                "platforms/macos/Configuration/"
                "KernelHost.contract-fixture.entitlements"
            )
            entitlements = plistlib.loads(path.read_bytes())
            entitlements["com.apple.security.files.user-selected.read-write"] = True
            path.write_bytes(plistlib.dumps(entitlements, sort_keys=True))
            failures = validate_sources(root)
        self.assertTrue(any("write authority" in item for item in failures))

    def test_native_status_promotion_is_rejected(self) -> None:
        failures = self.mutate(
            "platforms/macos/Sources/AgentMageMacOSPlatform/PlatformBoundary.swift",
            'workspaceBookmarkSourceStatus = "implemented-source-unverified"',
            'workspaceBookmarkSourceStatus = "verified"',
        )
        self.assertTrue(any("explicitly unverified" in item for item in failures))


if __name__ == "__main__":
    unittest.main()
