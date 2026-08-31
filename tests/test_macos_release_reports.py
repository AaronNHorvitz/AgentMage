from __future__ import annotations

import hashlib
import json
import os
import plistlib
import tempfile
import unittest
from pathlib import Path

from scripts.macos_release_runner_source_contract import EXPECTED_ENTITLEMENTS
from scripts.macos_release_reports import (
    SOURCE_PATHS,
    assemble_reports,
    build_source_report,
    validate_reports,
    validate_sources,
)


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


class MacOSReleaseReportsTests(unittest.TestCase):
    """Seven closed tests for native report assembly and reconciliation."""

    def bundle(self, root: Path) -> tuple[Path, Path, Path, Path, Path]:
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
        policy_path = root / "policy.json"
        manifest_path = root / "manifest.json"
        terminal_path = root / "terminal.json"
        for path, value in ((policy_path, policy), (manifest_path, manifest), (terminal_path, terminal)):
            path.write_text(json.dumps(value), encoding="utf-8")
            path.chmod(0o600)

        evidence = root / "evidence"
        logs = evidence / "command-logs"
        evidence.mkdir(mode=0o700)
        logs.mkdir(mode=0o700)
        fixed = {
            "005-codesign-deep-verify.log": b"valid on disk\nsatisfies its Designated Requirement\n",
            "014-package-signature.log": b"Status: signed by a developer certificate\n",
            "018-staple.log": b"The staple and validate action worked!\n",
            "019-staple-validate.log": b"The validate action worked!\n",
            "020-gatekeeper-install.log": b"accepted\nsource=Notarized Developer ID\n",
        }
        for name, value in fixed.items():
            (logs / name).write_bytes(value)
        for index, (component, bundle) in enumerate(bundles.items(), start=1):
            identity = logs / f"{index + 5:03d}-component-{index}-identity.log"
            identity.write_text(
                f"Identifier={bundle}\nTeamIdentifier={team}\nflags=0x10000(runtime)\n",
                encoding="utf-8",
            )
            plist = logs / f"{index + 20:03d}-component-{index}-entitlements.plist"
            plist.write_bytes(
                plistlib.dumps({name: True for name in EXPECTED_ENTITLEMENTS[component]})
            )
        submission = evidence / "notary-submit.json"
        submission.write_text(
            json.dumps({"id": "12345678-1234-1234-1234-1234567890ab", "status": "Accepted"}),
            encoding="utf-8",
        )
        notary_log = evidence / "notary-log.json"
        notary_log.write_text(json.dumps({"status": "Accepted", "issues": []}), encoding="utf-8")
        for path in evidence.rglob("*"):
            if path.is_file():
                path.chmod(0o600)
        return policy_path, manifest_path, package, terminal_path, evidence

    def assembled(self, root: Path) -> tuple[tuple[Path, Path, Path, Path, Path], Path, dict]:
        paths = self.bundle(root)
        failures, report = assemble_reports(*paths)
        self.assertEqual(failures, [])
        self.assertIsNotNone(report)
        assert report is not None
        report_path = root / "reports.json"
        report_path.write_text(json.dumps(report), encoding="utf-8")
        report_path.chmod(0o600)
        return paths, report_path, report

    def test_source_contract_is_closed_and_blocked(self) -> None:
        self.assertEqual(validate_sources(), [])
        report = build_source_report()
        self.assertEqual(report["status"], "prepared-source-only-blocked-macos")
        self.assertTrue(all(value is False for value in report["execution"].values()))
        self.assertTrue(all(value is False for value in report["claims"].values()))
        self.assertEqual(len(report["remaining_blockers"]), 7)
        self.assertEqual(len(SOURCE_PATHS), 5)

    def test_valid_raw_outputs_assemble_and_reconcile_content_free(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            paths, report_path, report = self.assembled(root)
            self.assertEqual(report["status"], "ceremony-passed")
            failures, result = validate_reports(*paths[:4], report_path)
            self.assertEqual(failures, [])
            self.assertIsNotNone(result)
            assert result is not None
            self.assertTrue(result["native_outputs_reconciled"])
            self.assertFalse(result["native_tools_executed_by_ingestor"])
            self.assertFalse(result["macos_support_claim"])

    def test_entitlement_substitution_fails_assembly_and_review(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            paths = self.bundle(root)
            plist = next((paths[4] / "command-logs").glob("*-component-1-entitlements.plist"))
            plist.write_bytes(plistlib.dumps({"com.apple.security.network.client": True}))
            failures, result = assemble_reports(*paths)
            self.assertIsNone(result)
            self.assertTrue(any("identity mismatch" in item for item in failures))

    def test_code_identity_component_hash_and_runtime_mutations_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            paths, report_path, report = self.assembled(root)
            report["code_signing"]["components"]["vscode_bridge"]["component_sha256"] = "9" * 64
            report["code_signing"]["components"]["kernel_host"]["hardened_runtime"] = False
            report_path.write_text(json.dumps(report), encoding="utf-8")
            failures, result = validate_reports(*paths[:4], report_path)
            self.assertIsNone(result)
            self.assertTrue(any("code-signing component mismatch" in item for item in failures))

    def test_notary_failure_issue_and_submission_mutations_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            paths, report_path, report = self.assembled(root)
            report["notarization"]["status"] = "Invalid"
            report["notarization"]["issues"] = [{"severity": "error"}]
            report["notarization"]["submission_id"] = "not-a-uuid"
            report_path.write_text(json.dumps(report), encoding="utf-8")
            failures, result = validate_reports(*paths[:4], report_path)
            self.assertIsNone(result)
            self.assertTrue(any("notarization report" in item for item in failures))

    def test_staple_gatekeeper_package_and_source_mutations_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            paths, report_path, report = self.assembled(root)
            report["stapling"]["package_sha256"] = "8" * 64
            report["gatekeeper"]["source"] = "Unnotarized Developer ID"
            report["source_revision"] = "7" * 40
            report_path.write_text(json.dumps(report), encoding="utf-8")
            failures, result = validate_reports(*paths[:4], report_path)
            self.assertIsNone(result)
            self.assertTrue(any("stapling report" in item for item in failures))
            self.assertTrue(any("Gatekeeper report" in item for item in failures))
            self.assertTrue(any("source_revision" in item for item in failures))

    def test_writable_linked_unknown_and_credential_shaped_inputs_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            paths, report_path, report = self.assembled(root)
            report["password"] = "forbidden"
            report_path.write_text(json.dumps(report), encoding="utf-8")
            report_path.chmod(0o666)
            failures, result = validate_reports(*paths[:4], report_path)
            self.assertIsNone(result)
            self.assertTrue(any("unsafe evidence file" in item for item in failures))
            report_path.chmod(0o600)
            linked = root / "linked-reports.json"
            os.link(report_path, linked)
            failures, result = validate_reports(*paths[:4], report_path)
            self.assertIsNone(result)
            self.assertTrue(any("unsafe evidence file" in item for item in failures))


if __name__ == "__main__":
    unittest.main()
