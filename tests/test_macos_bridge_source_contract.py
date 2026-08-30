from __future__ import annotations

import shutil
import tempfile
import unittest
from pathlib import Path

from scripts.macos_bridge_source_contract import (
    BRIDGE_FAILURES,
    ROOT,
    SOCKET_FAILURES,
    SOURCE_PATHS,
    validate_sources,
)


class MacOSBridgeSourceContractTests(unittest.TestCase):
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

    def test_protocol_version_or_domain_drift_is_rejected(self) -> None:
        failures = self.mutate(
            "platforms/macos/Sources/AgentMageMacOSPlatform/MacOSBridgeProtocol.swift",
            "agentMageMacOSIPCProtocolVersion: UInt32 = 1",
            "agentMageMacOSIPCProtocolVersion: UInt32 = 2",
        )
        self.assertTrue(any("protocol" in item for item in failures))
        failures = self.mutate(
            "platforms/macos/Sources/AgentMageMacOSPlatform/MacOSBridgeProtocol.swift",
            "agentmage-macos-ipc-auth-v1",
            "agentmage-macos-ipc-auth-v2",
        )
        self.assertTrue(any("auth-v1" in item for item in failures))

    def test_socket_mode_broadening_is_rejected(self) -> None:
        failures = self.mutate(
            "platforms/macos/Sources/AgentMageMacOSPlatform/AppGroupSocketBoundary.swift",
            "observation.socketMode == 0o600",
            "observation.socketMode == 0o660",
        )
        self.assertTrue(any("socket boundary" in item for item in failures))

    def test_missing_socket_mutation_is_rejected(self) -> None:
        failures = self.mutate(
            "platforms/macos/Tests/AgentMageMacOSPlatformTests/MacOSBridgeProtocolTests.swift",
            "cases.append((item, .outsideAppGroup))",
            "// removed outside-App-Group mutation",
        )
        self.assertTrue(any("mutation matrix" in item for item in failures))

    def test_failure_closures_are_exact(self) -> None:
        self.assertEqual(len(BRIDGE_FAILURES), 9)
        self.assertEqual(len(SOCKET_FAILURES), 8)


if __name__ == "__main__":
    unittest.main()
