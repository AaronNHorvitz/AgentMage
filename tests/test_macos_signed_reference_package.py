from __future__ import annotations

import hashlib
import json
import os
import tempfile
import unittest
from pathlib import Path

from scripts.macos_release_runner_source_contract import EXPECTED_ENTITLEMENTS
from scripts.macos_signed_reference_package import (
    ROOT,
    SOURCE_PATHS,
    build_source_report,
    validate_reference_bundle,
    validate_release_policy,
    validate_sources,
)


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


class MacOSSignedReferencePackageTests(unittest.TestCase):
    """Seven closed tests for source, identity, digest, lifecycle, and file safety."""

    def bundle(self, root: Path) -> tuple[Path, Path, Path, Path]:
        previous = root / "previous.pkg"
        previous.write_bytes(b"previous signed package")
        package = root / "AgentMage-1.2.3.pkg"
        package.write_bytes(b"synthetic signed package bytes")
        package_sha256 = digest(package.read_bytes())
        components = {
            "kernel_host": digest(b"kernel"),
            "vscode_bridge": digest(b"bridge"),
            "xpc_tool_helper": digest(b"tool"),
            "metal_inference_service": digest(b"metal"),
        }
        bundles = {
            "kernel_host": "dev.agentmage.release.host",
            "vscode_bridge": "dev.agentmage.release.bridge",
            "xpc_tool_helper": "dev.agentmage.release.tool",
            "metal_inference_service": "dev.agentmage.release.inference",
        }
        team = "B123456789"
        policy = {
            "schema_version": 1,
            "record_type": "macos-release-runner-policy",
            "status": "release-approved",
            "ceremony_id": "agentmage-macos-release-1.2.3",
            "source_revision": "1" * 40,
            "version": "1.2.3",
            "expected_macos_build": "24A123",
            "expected_xcode_build": "16A123",
            "architecture": "arm64",
            "team_id": team,
            "application_identity_sha1": "2" * 40,
            "installer_identity_sha1": "3" * 40,
            "bundle_identifiers": bundles,
            "app_group_identifier": "group.dev.agentmage.release",
            "keychain_access_group": f"{team}.dev.agentmage.release",
            "entitlements": EXPECTED_ENTITLEMENTS,
            "previous_package_path": str(previous),
            "previous_package_sha256": digest(previous.read_bytes()),
            "notary_keychain_profile": "agentmage-release-notary-v1",
            "credential_values_present": False,
            "private_environment_values_present": False,
            "release_claim": "authorized-candidate",
        }
        requirements = {
            name: (
                f"anchor apple generic and identifier {identifier} and "
                f"certificate leaf[subject.OU] = {team}"
            )
            for name, identifier in bundles.items()
        }
        manifest = {
            "schema_version": 1,
            "record_type": "macos-release-manifest",
            "manifest_id": "agentmage-macos-arm64-1.2.3",
            "status": "signed-release",
            "identity_class": "release-derived",
            "platform": {
                "family": "macos",
                "minimum_macos_version": "15.0",
                "tested_macos_build": {"id": "24A123", "sha256": digest(b"24A123")},
                "architecture": "arm64",
            },
            "toolchain": {
                "apple_sdk": {"id": "macosx15.0", "sha256": digest(b"sdk")},
                "swift_toolchain": {
                    "version": "6.0",
                    "build": "swift-release-build",
                    "sha256": digest(b"swift"),
                },
            },
            "vscode": {"version": "1.132.0", "commit": "4" * 40, "sha256": digest(b"vscode")},
            "code_identity": {
                "team_id": team,
                "bundle_identifiers": bundles,
                "app_group_identifier": "group.dev.agentmage.release",
                "designated_requirements": requirements,
            },
            "entitlements": EXPECTED_ENTITLEMENTS,
            "component_hashes": components,
            "package": {"format": "pkg", "sha256": package_sha256},
            "credential_values_present": False,
            "private_environment_values_present": False,
            "macos_execution_performed": True,
            "release_claim": "signed-package-candidate",
        }
        terminal = {
            "schema_version": 1,
            "record_type": "macos-release-runner-terminal",
            "source_revision": "1" * 40,
            "version": "1.2.3",
            "macos_build": "24A123",
            "xcode_build": "16A123",
            "architecture": "arm64",
            "package_sha256": package_sha256,
            "notary_status": "Accepted",
            "staple_valid": True,
            "gatekeeper_install_accepted": True,
            "candidate_install": True,
            "candidate_launch": True,
            "candidate_uninstall": True,
            "rollback_install": True,
            "rollback_launch": True,
            "credential_values_present": False,
            "release_claim": "signed-package-candidate",
        }
        paths = (root / "policy.json", root / "manifest.json", package, root / "terminal.json")
        for path, value in zip((paths[0], paths[1], paths[3]), (policy, manifest, terminal), strict=True):
            path.write_text(json.dumps(value), encoding="utf-8")
            path.chmod(0o600)
        return paths

    def test_source_contract_is_closed_and_blocked(self) -> None:
        self.assertEqual(validate_sources(), [])
        report = build_source_report()
        self.assertEqual(report["status"], "prepared-source-only-blocked-macos")
        self.assertTrue(all(value is False for value in report["execution"].values()))
        self.assertTrue(all(value is False for value in report["claims"].values()))
        self.assertEqual(len(report["remaining_blockers"]), 6)
        self.assertEqual(len(SOURCE_PATHS), 4)

    def test_valid_synthetic_bundle_produces_content_free_candidate_record(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            paths = self.bundle(Path(temporary))
            failures, result = validate_reference_bundle(*paths)
            self.assertEqual(failures, [])
            self.assertIsNotNone(result)
            assert result is not None
            self.assertEqual(result["status"], "accepted-candidate-evidence")
            self.assertFalse(result["native_signature_reverified_by_ingestor"])
            self.assertFalse(result["macos_support_claim"])

    def test_package_and_cross_record_digest_mutations_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            paths = self.bundle(Path(temporary))
            paths[2].write_bytes(b"substituted package")
            failures, result = validate_reference_bundle(*paths)
            self.assertIsNone(result)
            self.assertTrue(any("digest" in item or "package_sha256" in item for item in failures))

    def test_policy_identity_credential_and_entitlement_mutations_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            paths = self.bundle(Path(temporary))
            policy = json.loads(paths[0].read_text())
            policy["team_id"] = "AAAAAAAAAA"
            self.assertNotEqual(validate_release_policy(policy), [])
            policy = json.loads(paths[0].read_text())
            policy["password"] = "forbidden"
            self.assertNotEqual(validate_release_policy(policy), [])
            policy = json.loads(paths[0].read_text())
            policy["entitlements"]["metal_inference_service"].append(
                "com.apple.security.network.client"
            )
            self.assertNotEqual(validate_release_policy(policy), [])

    def test_manifest_component_identity_and_hash_mutations_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            paths = self.bundle(Path(temporary))
            manifest = json.loads(paths[1].read_text())
            manifest["component_hashes"]["vscode_bridge"] = manifest["component_hashes"]["kernel_host"]
            paths[1].write_text(json.dumps(manifest))
            paths[1].chmod(0o600)
            failures, result = validate_reference_bundle(*paths)
            self.assertIsNone(result)
            self.assertTrue(any("component hashes" in item for item in failures))

    def test_terminal_failure_or_unknown_field_cannot_be_accepted(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            paths = self.bundle(Path(temporary))
            terminal = json.loads(paths[3].read_text())
            terminal["candidate_uninstall"] = False
            terminal["unknown"] = True
            paths[3].write_text(json.dumps(terminal))
            paths[3].chmod(0o600)
            failures, result = validate_reference_bundle(*paths)
            self.assertIsNone(result)
            self.assertTrue(any("closed evidence fields" in item for item in failures))

    def test_symlink_hardlink_and_writable_inputs_fail_before_hashing(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            paths = list(self.bundle(root))
            paths[0].chmod(0o666)
            failures, result = validate_reference_bundle(*paths)
            self.assertIsNone(result)
            self.assertTrue(any("unsafe evidence file" in item for item in failures))
            paths = list(self.bundle(root))
            linked = root / "linked.pkg"
            os.link(paths[2], linked)
            failures, result = validate_reference_bundle(*paths)
            self.assertIsNone(result)
            self.assertTrue(any("unsafe reference package" in item for item in failures))


if __name__ == "__main__":
    unittest.main()
