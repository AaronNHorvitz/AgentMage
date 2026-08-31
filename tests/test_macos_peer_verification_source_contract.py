from __future__ import annotations

import json
import shutil
import tempfile
import unittest
from pathlib import Path

from scripts.macos_peer_verification_source_contract import (
    OBSERVATION_FAILURES,
    PEER_FAILURES,
    ROOT,
    SOURCE_PATHS,
    build_report,
    validate_sources,
)


class MacOSPeerVerificationSourceContractTests(unittest.TestCase):
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

    def test_report_preserves_every_native_and_release_blocker(self) -> None:
        report = build_report()
        self.assertEqual(report["status"], "partial-source-only-blocked-macos")
        self.assertEqual(report["contract"]["peer_refusal_count"], len(PEER_FAILURES))
        self.assertEqual(
            report["contract"]["observation_refusal_count"],
            len(OBSERVATION_FAILURES),
        )
        self.assertTrue(all(value is False for value in report["execution"].values()))
        self.assertTrue(all(value is False for value in report["claims"].values()))
        self.assertEqual(len(report["remaining_blockers"]), 5)
        json.dumps(report)

    def test_audit_token_lookup_removal_is_rejected(self) -> None:
        failures = self.mutate(
            "platforms/macos/Sources/AgentMageMacOSPlatform/MacOSPeerVerification.swift",
            "LOCAL_PEERTOKEN",
            "removedPeerTokenSocketOption",
        )
        self.assertTrue(any("LOCAL_PEERTOKEN" in item for item in failures))
        failures = self.mutate(
            "platforms/macos/Sources/AgentMageMacOSPlatform/MacOSPeerVerification.swift",
            "kSecGuestAttributeAudit",
            "removedAuditGuestAttribute",
        )
        self.assertTrue(any("kSecGuestAttributeAudit" in item for item in failures))

    def test_designated_requirement_or_verified_route_removal_is_rejected(self) -> None:
        failures = self.mutate(
            "platforms/macos/Sources/AgentMageMacOSPlatform/MacOSPeerVerification.swift",
            "SecRequirementCreateWithString",
            "removedRequirementCompiler",
        )
        self.assertTrue(any("SecRequirementCreateWithString" in item for item in failures))
        failures = self.mutate(
            "platforms/macos/Sources/AgentMageMacOSPlatform/MacOSBridgeProtocol.swift",
            "verifiedPeer: VerifiedMacOSBridgePeer",
            "removedVerifiedPeer: VerifiedMacOSBridgePeer",
        )
        self.assertTrue(any("verified peer" in item for item in failures))

    def test_missing_peer_mutation_is_rejected(self) -> None:
        failures = self.mutate(
            "platforms/macos/Tests/AgentMageMacOSPlatformTests/MacOSPeerVerificationTests.swift",
            "cases.append((item, .auditTokenMissing))",
            "// removed audit-token mutation",
        )
        self.assertTrue(any("mutation matrix" in item for item in failures))

    def test_native_status_promotion_is_rejected(self) -> None:
        failures = self.mutate(
            "platforms/macos/Sources/AgentMageMacOSPlatform/PlatformBoundary.swift",
            'peerVerificationSourceStatus = "implemented-source-unverified"',
            'peerVerificationSourceStatus = "verified"',
        )
        self.assertTrue(any("explicitly unverified" in item for item in failures))


if __name__ == "__main__":
    unittest.main()
