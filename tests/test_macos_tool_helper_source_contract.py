from __future__ import annotations

import json
import plistlib
import shutil
import tempfile
import unittest
from pathlib import Path

from scripts.macos_tool_helper_source_contract import (
    ROOT,
    SOURCE_PATHS,
    TOOL_HELPER_FAILURES,
    build_report,
    validate_sources,
)


class MacOSToolHelperSourceContractTests(unittest.TestCase):
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

    def test_report_preserves_every_native_and_cross_story_blocker(self) -> None:
        report = build_report()
        self.assertEqual(report["status"], "partial-source-only-blocked-macos")
        self.assertEqual(report["related_open_task_id"], "16.1.1.5")
        self.assertEqual(report["contract"]["failure_count"], len(TOOL_HELPER_FAILURES))
        self.assertTrue(all(value is False for value in report["execution"].values()))
        self.assertTrue(all(value is False for value in report["claims"].values()))
        self.assertEqual(len(report["remaining_blockers"]), 7)
        json.dumps(report)

    def test_consumed_grant_and_one_shot_weakening_is_rejected(self) -> None:
        protocol = (
            "platforms/macos/Sources/AgentMageMacOSPlatform/"
            "MacOSToolHelperProtocol.swift"
        )
        failures = self.mutate(
            protocol,
            'observation.grantState == "consumed"',
            'observation.grantState == "issued"',
        )
        self.assertTrue(any("grantState" in item for item in failures))
        failures = self.mutate(
            protocol,
            "private var consumed = false",
            "private var consumed = true",
        )
        self.assertTrue(any("consumed" in item for item in failures))

    def test_no_follow_or_resource_bound_removal_is_rejected(self) -> None:
        service = (
            "platforms/macos/Sources/AgentMageMacOSPlatform/"
            "MacOSToolHelperService.swift"
        )
        failures = self.mutate(service, "O_NOFOLLOW", "removedNoFollow")
        self.assertTrue(any("O_NOFOLLOW" in item for item in failures))
        failures = self.mutate(service, "RLIMIT_AS", "removedAddressSpaceLimit")
        self.assertTrue(any("RLIMIT_AS" in item for item in failures))

    def test_network_inherit_app_group_and_write_entitlements_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.copy_inputs(root)
            path = root / (
                "platforms/macos/Configuration/"
                "XPCToolHelper.contract-fixture.entitlements"
            )
            for key in (
                "com.apple.security.network.client",
                "com.apple.security.inherit",
                "com.apple.security.application-groups",
                "com.apple.security.files.user-selected.read-write",
            ):
                entitlements = plistlib.loads(path.read_bytes())
                entitlements[key] = True
                path.write_bytes(plistlib.dumps(entitlements, sort_keys=True))
                failures = validate_sources(root)
                self.assertTrue(any("entitlement" in item for item in failures))
                path.write_bytes(
                    plistlib.dumps(
                        {
                            "com.apple.security.app-sandbox": True,
                            "com.apple.security.files.bookmarks.app-scope": True,
                        },
                        sort_keys=True,
                    )
                )

    def test_native_status_promotion_is_rejected(self) -> None:
        failures = self.mutate(
            "platforms/macos/Sources/AgentMageMacOSPlatform/PlatformBoundary.swift",
            'toolHelperSourceStatus = "implemented-source-unverified"',
            'toolHelperSourceStatus = "verified"',
        )
        self.assertTrue(any("explicitly unverified" in item for item in failures))


if __name__ == "__main__":
    unittest.main()
