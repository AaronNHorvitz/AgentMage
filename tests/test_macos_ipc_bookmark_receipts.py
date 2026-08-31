from __future__ import annotations

import hashlib
import json
import os
import tempfile
import unittest
from pathlib import Path

from scripts.macos_ipc_bookmark_receipts import (
    BOOKMARK_TRUE_FIELDS,
    IPC_TRUE_FIELDS,
    SOURCE_PATHS,
    assemble_receipts,
    build_source_report,
    validate_receipts,
    validate_sources,
)
from scripts.macos_release_runner_source_contract import EXPECTED_ENTITLEMENTS


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


class MacOSIPCBookmarkReceiptTests(unittest.TestCase):
    """Seven closed tests for IPC and bookmark receipt reconciliation."""

    def release_bundle(self, root: Path) -> tuple[Path, Path, Path, Path, dict, dict]:
        previous = root / "previous.pkg"
        previous.write_bytes(b"previous signed package")
        package = root / "AgentMage-1.2.3.pkg"
        package.write_bytes(b"synthetic signed package")
        package_sha256 = digest(package.read_bytes())
        team = "B123456789"
        bundles = {
            "kernel_host": "dev.agentmage.release.host",
            "vscode_bridge": "dev.agentmage.release.bridge",
            "xpc_tool_helper": "dev.agentmage.release.tool",
            "metal_inference_service": "dev.agentmage.release.inference",
        }
        components = {
            "kernel_host": digest(b"kernel"),
            "vscode_bridge": digest(b"bridge"),
            "xpc_tool_helper": digest(b"tool"),
            "metal_inference_service": digest(b"metal"),
        }
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
            "vscode": {
                "version": "1.132.0",
                "commit": "4" * 40,
                "sha256": digest(b"vscode"),
            },
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
        policy_path = root / "policy.json"
        manifest_path = root / "manifest.json"
        terminal_path = root / "terminal.json"
        for path, value in (
            (policy_path, policy),
            (manifest_path, manifest),
            (terminal_path, terminal),
        ):
            path.write_text(json.dumps(value), encoding="utf-8")
            path.chmod(0o600)
        return policy_path, manifest_path, package, terminal_path, policy, manifest

    def raw_receipts(
        self, root: Path, policy: dict, manifest: dict, package: Path
    ) -> tuple[Path, dict, dict]:
        evidence = root / "native-receipts"
        evidence.mkdir(mode=0o700)
        common = {
            "schema_version": 1,
            "status": "passed",
            "source_revision": policy["source_revision"],
            "version": policy["version"],
            "macos_build": policy["expected_macos_build"],
            "xcode_build": policy["expected_xcode_build"],
            "architecture": "arm64",
            "package_sha256": digest(package.read_bytes()),
            "team_id": policy["team_id"],
            "credential_values_present": False,
            "private_environment_values_present": False,
        }
        ipc = {
            **common,
            "record_type": "macos-ipc-authentication-receipt",
            "host_bundle_identifier": policy["bundle_identifiers"]["kernel_host"],
            "bridge_bundle_identifier": policy["bundle_identifiers"]["vscode_bridge"],
            "app_group_identifier": policy["app_group_identifier"],
            "bridge_designated_requirement": manifest["code_identity"]["designated_requirements"]["vscode_bridge"],
            "protocol_version": 1,
            "handshake_bytes": 68,
            "request_maximum_bytes": 64 * 1024,
            "response_maximum_bytes": 4 * 1024 * 1024,
            "socket_parent_mode": "0700",
            "socket_mode": "0600",
            "peer_audit_identity_sha256": digest(b"peer-audit-identity"),
            **{key: True for key in IPC_TRUE_FIELDS},
        }
        bookmark = {
            **common,
            "record_type": "macos-bookmark-lifecycle-receipt",
            "host_bundle_identifier": policy["bundle_identifiers"]["kernel_host"],
            "app_group_identifier": policy["app_group_identifier"],
            "keychain_access_group": policy["keychain_access_group"],
            "bookmark_identifier_sha256": digest(b"bookmark-identity"),
            "resource_identity_sha256": digest(b"resource-identity"),
            "volume_identity_sha256": digest(b"volume-identity"),
            "selection_count": 1,
            "bookmark_scope": "app-scoped",
            "bookmark_authority": "read-only",
            **{key: True for key in BOOKMARK_TRUE_FIELDS},
        }
        for name, value in (
            ("ipc-authentication-receipt.json", ipc),
            ("bookmark-lifecycle-receipt.json", bookmark),
        ):
            path = evidence / name
            path.write_text(json.dumps(value), encoding="utf-8")
            path.chmod(0o600)
        return evidence, ipc, bookmark

    def assembled(
        self, root: Path
    ) -> tuple[tuple[Path, Path, Path, Path, dict, dict], Path, Path, dict]:
        bundle = self.release_bundle(root)
        evidence, _ipc, _bookmark = self.raw_receipts(
            root, bundle[4], bundle[5], bundle[2]
        )
        failures, report = assemble_receipts(*bundle[:4], evidence)
        self.assertEqual(failures, [])
        self.assertIsNotNone(report)
        assert report is not None
        receipts_path = root / "receipts.json"
        receipts_path.write_text(json.dumps(report), encoding="utf-8")
        receipts_path.chmod(0o600)
        return bundle, evidence, receipts_path, report

    def test_source_contract_is_closed_and_blocked(self) -> None:
        self.assertEqual(validate_sources(), [])
        report = build_source_report()
        self.assertEqual(report["status"], "prepared-source-only-blocked-macos")
        self.assertTrue(all(value is False for value in report["execution"].values()))
        self.assertTrue(all(value is False for value in report["claims"].values()))
        self.assertEqual(len(report["remaining_blockers"]), 6)
        self.assertEqual(len(SOURCE_PATHS), 4)

    def test_valid_receipts_assemble_and_reconcile_content_free(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bundle, _evidence, receipts_path, report = self.assembled(root)
            self.assertEqual(report["status"], "native-lifecycle-passed")
            failures, result = validate_receipts(*bundle[:4], receipts_path)
            self.assertEqual(failures, [])
            self.assertIsNotNone(result)
            assert result is not None
            self.assertTrue(result["ipc_authentication_reconciled"])
            self.assertTrue(result["bookmark_lifecycle_reconciled"])
            self.assertFalse(result["native_operations_executed_by_ingestor"])
            self.assertFalse(result["macos_support_claim"])

    def test_ipc_identity_and_peer_mutations_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bundle = self.release_bundle(root)
            evidence, ipc, _bookmark = self.raw_receipts(root, bundle[4], bundle[5], bundle[2])
            ipc["team_id"] = "C123456789"
            ipc["live_audit_token_verified"] = False
            ipc["peer_audit_identity_sha256"] = "0" * 64
            path = evidence / "ipc-authentication-receipt.json"
            path.write_text(json.dumps(ipc), encoding="utf-8")
            path.chmod(0o600)
            failures, result = assemble_receipts(*bundle[:4], evidence)
            self.assertIsNone(result)
            self.assertTrue(any("team_id" in item for item in failures))
            self.assertTrue(any("live_audit_token_verified" in item for item in failures))
            self.assertTrue(any("audit identity" in item for item in failures))

    def test_ipc_protocol_socket_challenge_and_replay_mutations_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bundle, _evidence, receipts_path, report = self.assembled(root)
            report["ipc_authentication"]["protocol_version"] = 2
            report["ipc_authentication"]["socket_mode"] = "0666"
            report["ipc_authentication"]["fresh_launch_challenge_verified"] = False
            report["ipc_authentication"]["replay_refused"] = False
            receipts_path.write_text(json.dumps(report), encoding="utf-8")
            receipts_path.chmod(0o600)
            failures, result = validate_receipts(*bundle[:4], receipts_path)
            self.assertIsNone(result)
            self.assertTrue(any("protocol_version" in item for item in failures))
            self.assertTrue(any("socket_mode" in item for item in failures))
            self.assertTrue(any("fresh_launch_challenge_verified" in item for item in failures))
            self.assertTrue(any("replay_refused" in item for item in failures))

    def test_bookmark_identity_scope_and_lifecycle_mutations_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bundle = self.release_bundle(root)
            evidence, _ipc, bookmark = self.raw_receipts(root, bundle[4], bundle[5], bundle[2])
            bookmark["bookmark_authority"] = "read-write"
            bookmark["resource_identity_sha256"] = bookmark["volume_identity_sha256"]
            bookmark["start_stop_access_balanced"] = False
            bookmark["post_revocation_denied"] = False
            path = evidence / "bookmark-lifecycle-receipt.json"
            path.write_text(json.dumps(bookmark), encoding="utf-8")
            path.chmod(0o600)
            failures, result = assemble_receipts(*bundle[:4], evidence)
            self.assertIsNone(result)
            self.assertTrue(any("bookmark_authority" in item for item in failures))
            self.assertTrue(any("collide" in item for item in failures))
            self.assertTrue(any("start_stop_access_balanced" in item for item in failures))
            self.assertTrue(any("post_revocation_denied" in item for item in failures))

    def test_release_binding_unknown_fields_and_credential_shapes_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bundle = self.release_bundle(root)
            evidence, ipc, _bookmark = self.raw_receipts(root, bundle[4], bundle[5], bundle[2])
            ipc["source_revision"] = "9" * 40
            ipc["password"] = "forbidden"
            path = evidence / "ipc-authentication-receipt.json"
            path.write_text(json.dumps(ipc), encoding="utf-8")
            path.chmod(0o600)
            failures, result = assemble_receipts(*bundle[:4], evidence)
            self.assertIsNone(result)
            self.assertTrue(any("field closure" in item for item in failures))

    def test_writable_linked_and_substituted_receipts_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bundle, evidence, receipts_path, report = self.assembled(root)
            ipc_path = evidence / "ipc-authentication-receipt.json"
            ipc_path.chmod(0o666)
            failures, result = assemble_receipts(*bundle[:4], evidence)
            self.assertIsNone(result)
            self.assertTrue(any("unsafe evidence file" in item for item in failures))
            ipc_path.chmod(0o600)
            linked = evidence / "linked-ipc.json"
            os.link(ipc_path, linked)
            failures, result = assemble_receipts(*bundle[:4], evidence)
            self.assertIsNone(result)
            self.assertTrue(any("unsafe evidence file" in item for item in failures))
            linked.unlink()
            report["raw_receipt_hashes"]["bookmark_lifecycle"] = report["raw_receipt_hashes"]["ipc_authentication"]
            receipts_path.write_text(json.dumps(report), encoding="utf-8")
            receipts_path.chmod(0o600)
            failures, result = validate_receipts(*bundle[:4], receipts_path)
            self.assertIsNone(result)
            self.assertTrue(any("hashes" in item for item in failures))


if __name__ == "__main__":
    unittest.main()
