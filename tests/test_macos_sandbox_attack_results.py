from __future__ import annotations

import hashlib
import json
import os
import tempfile
import unittest
from pathlib import Path

from scripts.macos_release_runner_source_contract import COMPONENTS, EXPECTED_ENTITLEMENTS
from scripts.macos_sandbox_attack_results import (
    ATTACK_CLASSES,
    SOURCE_PATHS,
    assemble_results,
    attack_log_name,
    build_source_report,
    expected_attack_pairs,
    validate_results,
    validate_sources,
)
from tests import test_macos_ipc_bookmark_receipts as release_fixture


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


class MacOSSandboxAttackResultTests(unittest.TestCase):
    """Seven closed tests for sandbox profile and attack-result reconciliation."""

    def release_bundle(self, root: Path) -> tuple[Path, Path, Path, Path, dict, dict]:
        return release_fixture.MacOSIPCBookmarkReceiptTests().release_bundle(root)

    def native_evidence(
        self, root: Path, policy: dict, manifest: dict, package: Path
    ) -> tuple[Path, dict, dict]:
        evidence = root / "sandbox-evidence"
        evidence.mkdir(mode=0o700)
        profile_logs = evidence / "profile-logs"
        attack_logs = evidence / "attack-logs"
        profile_logs.mkdir(mode=0o700)
        attack_logs.mkdir(mode=0o700)
        package_sha256 = digest(package.read_bytes())
        common = {
            "schema_version": 1,
            "status": "passed",
            "source_revision": policy["source_revision"],
            "version": policy["version"],
            "macos_build": policy["expected_macos_build"],
            "xcode_build": policy["expected_xcode_build"],
            "architecture": "arm64",
            "team_id": policy["team_id"],
            "package_sha256": package_sha256,
            "credential_values_present": False,
            "private_environment_values_present": False,
        }
        profiles = []
        for component in COMPONENTS:
            log = profile_logs / f"{component}.log"
            log.write_bytes(f"synthetic profile evidence {component}\n".encode())
            log.chmod(0o600)
            profiles.append(
                {
                    "component": component,
                    "bundle_identifier": policy["bundle_identifiers"][component],
                    "designated_requirement": manifest["code_identity"]["designated_requirements"][component],
                    "component_sha256": manifest["component_hashes"][component],
                    "entitlement_keys": list(EXPECTED_ENTITLEMENTS[component]),
                    "app_sandbox_observed": True,
                    "hardened_runtime_observed": True,
                    "sandbox_profile_active": True,
                    "network_entitlement_present": False,
                    "workspace_write_entitlement_present": False,
                    "temporary_exception_entitlement_present": False,
                    "raw_report_sha256": digest(log.read_bytes()),
                }
            )
        profile_record = {
            **common,
            "record_type": "macos-sandbox-profile-results",
            "profiles": profiles,
        }
        cases = []
        for component, attack_class in expected_attack_pairs():
            log = attack_logs / attack_log_name(component, attack_class)
            log.write_bytes(
                f"synthetic denied attack {component} {attack_class}\n".encode()
            )
            log.chmod(0o600)
            cases.append(
                {
                    "case_id": f"{component}:{attack_class}",
                    "component": component,
                    "attack_class": attack_class,
                    "status": "passed-zero-unauthorized-access",
                    "attempted": True,
                    "unauthorized_access_count": 0,
                    "unauthorized_byte_count": 0,
                    "network_connection_count": 0,
                    "network_byte_count": 0,
                    "descendant_process_count": 0,
                    "residue_count": 0,
                    "canary_observed": False,
                    "workspace_modified": False,
                    "authority_broadened": False,
                    "raw_report_sha256": digest(log.read_bytes()),
                }
            )
        attack_record = {
            **common,
            "record_type": "macos-sandbox-attack-results",
            "attack_classes": list(ATTACK_CLASSES),
            "cases": cases,
        }
        for name, value in (
            ("sandbox-profile-results.json", profile_record),
            ("sandbox-attack-results.json", attack_record),
        ):
            path = evidence / name
            path.write_text(json.dumps(value), encoding="utf-8")
            path.chmod(0o600)
        return evidence, profile_record, attack_record

    def assembled(
        self, root: Path
    ) -> tuple[tuple[Path, Path, Path, Path, dict, dict], Path, Path, dict]:
        bundle = self.release_bundle(root)
        evidence, _profiles, _attacks = self.native_evidence(
            root, bundle[4], bundle[5], bundle[2]
        )
        failures, report = assemble_results(*bundle[:4], evidence)
        self.assertEqual(failures, [])
        self.assertIsNotNone(report)
        assert report is not None
        results_path = root / "results.json"
        results_path.write_text(json.dumps(report), encoding="utf-8")
        results_path.chmod(0o600)
        return bundle, evidence, results_path, report

    def rewrite(self, path: Path, value: dict) -> None:
        path.write_text(json.dumps(value), encoding="utf-8")
        path.chmod(0o600)

    def test_source_contract_is_closed_and_blocked(self) -> None:
        self.assertEqual(validate_sources(), [])
        report = build_source_report()
        self.assertEqual(report["status"], "prepared-source-only-blocked-macos")
        self.assertTrue(all(value is False for value in report["execution"].values()))
        self.assertTrue(all(value is False for value in report["claims"].values()))
        self.assertEqual(report["contract"]["attack_case_count"], 36)
        self.assertEqual(len(report["remaining_blockers"]), 6)
        self.assertEqual(len(SOURCE_PATHS), 5)

    def test_valid_profiles_and_36_attacks_reconcile_content_free(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bundle, _evidence, results_path, report = self.assembled(root)
            self.assertEqual(len(report["profiles"]), 4)
            self.assertEqual(len(report["attacks"]["cases"]), 36)
            failures, result = validate_results(*bundle[:4], results_path)
            self.assertEqual(failures, [])
            self.assertIsNotNone(result)
            assert result is not None
            self.assertEqual(result["unauthorized_access_count"], 0)
            self.assertEqual(result["unauthorized_byte_count"], 0)
            self.assertFalse(result["native_operations_executed_by_ingestor"])
            self.assertFalse(result["macos_support_claim"])

    def test_profile_identity_entitlement_and_sandbox_mutations_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bundle = self.release_bundle(root)
            evidence, profiles, _attacks = self.native_evidence(
                root, bundle[4], bundle[5], bundle[2]
            )
            profiles["profiles"][0]["component_sha256"] = "9" * 64
            profiles["profiles"][1]["entitlement_keys"].append(
                "com.apple.security.network.client"
            )
            profiles["profiles"][2]["sandbox_profile_active"] = False
            profiles["profiles"][3]["temporary_exception_entitlement_present"] = True
            self.rewrite(evidence / "sandbox-profile-results.json", profiles)
            failures, result = assemble_results(*bundle[:4], evidence)
            self.assertIsNone(result)
            self.assertTrue(any("component_sha256" in item for item in failures))
            self.assertTrue(any("entitlement_keys" in item for item in failures))
            self.assertTrue(any("sandbox_profile_active" in item for item in failures))
            self.assertTrue(any("temporary_exception" in item for item in failures))

    def test_missing_duplicate_and_reordered_attack_matrix_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bundle = self.release_bundle(root)
            evidence, _profiles, attacks = self.native_evidence(
                root, bundle[4], bundle[5], bundle[2]
            )
            attacks["cases"][0], attacks["cases"][1] = (
                attacks["cases"][1], attacks["cases"][0]
            )
            self.rewrite(evidence / "sandbox-attack-results.json", attacks)
            failures, result = assemble_results(*bundle[:4], evidence)
            self.assertIsNone(result)
            self.assertTrue(any("matrix order" in item for item in failures))
            attacks["cases"] = attacks["cases"][:-1]
            self.rewrite(evidence / "sandbox-attack-results.json", attacks)
            failures, result = assemble_results(*bundle[:4], evidence)
            self.assertIsNone(result)
            self.assertTrue(any("case closure" in item for item in failures))

    def test_access_bytes_network_workspace_authority_and_residue_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bundle, _evidence, results_path, report = self.assembled(root)
            case = report["attacks"]["cases"][0]
            case["unauthorized_access_count"] = 1
            case["unauthorized_byte_count"] = 8
            case["network_connection_count"] = 1
            case["workspace_modified"] = True
            case["authority_broadened"] = True
            case["residue_count"] = 1
            self.rewrite(results_path, report)
            failures, result = validate_results(*bundle[:4], results_path)
            self.assertIsNone(result)
            for field in (
                "unauthorized_access_count",
                "unauthorized_byte_count",
                "network_connection_count",
                "workspace_modified",
                "authority_broadened",
                "residue_count",
            ):
                self.assertTrue(any(field in item for item in failures))

    def test_missing_extra_substituted_and_colliding_raw_logs_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bundle = self.release_bundle(root)
            evidence, _profiles, attacks = self.native_evidence(
                root, bundle[4], bundle[5], bundle[2]
            )
            first = attacks["cases"][0]
            first_log = evidence / "attack-logs" / attack_log_name(
                first["component"], first["attack_class"]
            )
            first_log.write_bytes(b"substituted")
            first_log.chmod(0o600)
            extra = evidence / "attack-logs" / "extra.log"
            extra.write_bytes(b"extra")
            extra.chmod(0o600)
            failures, result = assemble_results(*bundle[:4], evidence)
            self.assertIsNone(result)
            self.assertTrue(any("log closure" in item for item in failures))
            extra.unlink()
            failures, result = assemble_results(*bundle[:4], evidence)
            self.assertIsNone(result)
            self.assertTrue(any("raw digest mismatch" in item for item in failures))
            first_log.unlink()
            failures, result = assemble_results(*bundle[:4], evidence)
            self.assertIsNone(result)
            self.assertTrue(any("log closure" in item for item in failures))

    def test_unsafe_linked_unknown_credential_and_release_binding_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bundle = self.release_bundle(root)
            evidence, profiles, _attacks = self.native_evidence(
                root, bundle[4], bundle[5], bundle[2]
            )
            profiles["source_revision"] = "8" * 40
            profiles["password"] = "forbidden"
            profile_path = evidence / "sandbox-profile-results.json"
            self.rewrite(profile_path, profiles)
            profile_path.chmod(0o666)
            failures, result = assemble_results(*bundle[:4], evidence)
            self.assertIsNone(result)
            self.assertTrue(any("unsafe evidence file" in item for item in failures))
            profile_path.chmod(0o600)
            linked = root / "linked-profile-results.json"
            os.link(profile_path, linked)
            failures, result = assemble_results(*bundle[:4], evidence)
            self.assertIsNone(result)
            self.assertTrue(any("unsafe evidence file" in item for item in failures))
            linked.unlink()
            failures, result = assemble_results(*bundle[:4], evidence)
            self.assertIsNone(result)
            self.assertTrue(any("closed evidence fields" in item for item in failures))
            del profiles["password"]
            self.rewrite(profile_path, profiles)
            failures, result = assemble_results(*bundle[:4], evidence)
            self.assertIsNone(result)
            self.assertTrue(any("source_revision" in item for item in failures))


if __name__ == "__main__":
    unittest.main()
