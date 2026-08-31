from __future__ import annotations

import json
import plistlib
import shutil
import tempfile
import unittest
from pathlib import Path

from scripts.macos_metal_inference_source_contract import (
    METAL_INFERENCE_FAILURES,
    ROOT,
    SOURCE_PATHS,
    build_report,
    validate_sources,
)


class MacOSMetalInferenceSourceContractTests(unittest.TestCase):
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

    def test_report_preserves_native_runtime_and_review_blockers(self) -> None:
        report = build_report()
        self.assertEqual(report["status"], "partial-source-only-blocked-macos")
        self.assertEqual(
            report["contract"]["failure_count"], len(METAL_INFERENCE_FAILURES)
        )
        self.assertTrue(all(value is False for value in report["execution"].values()))
        self.assertTrue(all(value is False for value in report["claims"].values()))
        self.assertEqual(len(report["remaining_blockers"]), 8)
        json.dumps(report)

    def test_request_authority_or_profile_weakening_is_rejected(self) -> None:
        protocol = (
            "platforms/macos/Sources/AgentMageMacOSPlatform/"
            "MacOSMetalInferenceProtocol.swift"
        )
        failures = self.mutate(
            protocol,
            "Set(dictionary.keys) == keys",
            "Set(dictionary.keys).isSuperset(of: keys)",
        )
        self.assertTrue(any("dictionary.keys" in item for item in failures))
        failures = self.mutate(
            protocol,
            "public let profileIdentifier: String",
            "public let workspaceBookmark: String",
        )
        self.assertTrue(any("workspace" in item for item in failures))

    def test_descriptor_digest_and_metal_checks_are_required(self) -> None:
        service = (
            "platforms/macos/Sources/AgentMageMacOSPlatform/"
            "MacOSMetalInferenceService.swift"
        )
        failures = self.mutate(service, "flags & O_ACCMODE", "flags")
        self.assertTrue(any("O_ACCMODE" in item for item in failures))
        failures = self.mutate(
            service,
            "observedDigest == expected.artifactSHA256",
            "true",
        )
        self.assertTrue(any("observedDigest" in item for item in failures))
        failures = self.mutate(
            service,
            "MTLCreateSystemDefaultDevice()",
            "nil",
        )
        self.assertTrue(any("MTLCreateSystemDefaultDevice" in item for item in failures))

    def test_all_non_sandbox_entitlements_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.copy_inputs(root)
            path = root / SOURCE_PATHS[0]
            for key in (
                "com.apple.security.network.client",
                "com.apple.security.application-groups",
                "com.apple.security.files.bookmarks.app-scope",
                "keychain-access-groups",
                "com.apple.security.cs.allow-jit",
            ):
                entitlements = plistlib.loads(path.read_bytes())
                entitlements[key] = True
                path.write_bytes(plistlib.dumps(entitlements, sort_keys=True))
                failures = validate_sources(root)
                self.assertTrue(any("entitlement" in item for item in failures))
                path.write_bytes(
                    plistlib.dumps(
                        {"com.apple.security.app-sandbox": True},
                        sort_keys=True,
                    )
                )

    def test_native_status_promotion_is_rejected(self) -> None:
        failures = self.mutate(
            "platforms/macos/Sources/AgentMageMacOSPlatform/PlatformBoundary.swift",
            'metalInferenceSourceStatus = "implemented-source-unverified"',
            'metalInferenceSourceStatus = "verified"',
        )
        self.assertTrue(any("explicitly unverified" in item for item in failures))


if __name__ == "__main__":
    unittest.main()
