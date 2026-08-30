from __future__ import annotations

import json
import plistlib
import shutil
import tempfile
import unittest
from pathlib import Path

from scripts.macos_kernel_host_source_contract import (
    EXPECTED_FAILURES,
    ROOT,
    SOURCE_PATHS,
    build_report,
    validate_sources,
)


class MacOSKernelHostSourceContractTests(unittest.TestCase):
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

    def test_report_is_source_only_and_preserves_every_blocker(self) -> None:
        report = build_report()
        self.assertEqual(report["status"], "partial-source-only-blocked-macos")
        self.assertEqual(
            report["contract"]["startup_refusal_count"], len(EXPECTED_FAILURES)
        )
        self.assertTrue(all(value is False for value in report["execution"].values()))
        self.assertTrue(all(value is False for value in report["claims"].values()))
        self.assertEqual(len(report["remaining_blockers"]), 5)
        json.dumps(report)

    def test_missing_hardened_runtime_observation_is_rejected(self) -> None:
        failures = self.mutate(
            "platforms/macos/Sources/AgentMageMacOSPlatform/KernelHostAdmission.swift",
            "kSecCodeInfoRuntimeVersion",
            "removedRuntimeVersionKey",
        )
        self.assertTrue(any("kSecCodeInfoRuntimeVersion" in item for item in failures))

    def test_native_status_promotion_is_rejected(self) -> None:
        failures = self.mutate(
            "platforms/macos/Sources/AgentMageMacOSPlatform/PlatformBoundary.swift",
            'implementationStatus = "blocked-macos"',
            'implementationStatus = "verified"',
        )
        self.assertTrue(any("promoted" in item for item in failures))

    def test_missing_refusal_mutation_is_rejected(self) -> None:
        failures = self.mutate(
            "platforms/macos/Tests/AgentMageMacOSPlatformTests/KernelHostAdmissionTests.swift",
            "cases.append((item, .wrongArchitecture))",
            "// removed architecture mutation",
        )
        self.assertTrue(any("mutation matrix" in item for item in failures))

    def test_extra_network_entitlement_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.copy_inputs(root)
            path = (
                root
                / "platforms/macos/Configuration/KernelHost.contract-fixture.entitlements"
            )
            entitlements = plistlib.loads(path.read_bytes())
            entitlements["com.apple.security.network.client"] = True
            path.write_bytes(plistlib.dumps(entitlements, sort_keys=True))
            failures = validate_sources(root)
        self.assertTrue(any("minimal closure" in item for item in failures))
        self.assertTrue(any("forbidden authority" in item for item in failures))

    def test_private_or_release_identity_is_rejected(self) -> None:
        failures = self.mutate(
            "platforms/macos/Configuration/KernelHost.contract-fixture.entitlements",
            "group.com.example.agentmage.contractfixture",
            "group.dev.agentmage.release",
        )
        self.assertTrue(any("minimal closure" in item for item in failures))


if __name__ == "__main__":
    unittest.main()
