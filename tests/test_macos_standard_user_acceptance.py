from __future__ import annotations

import json
import os
import tempfile
import unittest
from pathlib import Path

from scripts.macos_standard_user_acceptance import (
    PASS_FIELDS,
    PHASE_ORDER,
    SOURCE_PATHS,
    build_source_report,
    validate_acceptance,
    validate_sources,
)
from tests import test_macos_ipc_bookmark_receipts as release_fixture


class MacOSStandardUserAcceptanceTests(unittest.TestCase):
    """Seven closed tests for S-008-AT01 evidence verification."""

    def bundle(self, root: Path) -> tuple[Path, Path, Path, Path, dict, dict]:
        return release_fixture.MacOSIPCBookmarkReceiptTests().release_bundle(root)

    def acceptance(self, root: Path, bundle: tuple) -> tuple[Path, dict]:
        policy = bundle[4]
        package = bundle[2]
        value = {
            "schema_version": 1,
            "record_type": "macos-standard-user-acceptance",
            "source_revision": policy["source_revision"],
            "version": policy["version"],
            "macos_build": policy["expected_macos_build"],
            "xcode_build": policy["expected_xcode_build"],
            "architecture": "arm64",
            "package_sha256": release_fixture.digest(package.read_bytes()),
            "identity_class": "non-admin-standard-user",
            "real_effective_uid_match": True,
            "home_owner_match": True,
            "primary_gid_observed": True,
            "phase_order": list(PHASE_ORDER),
            **{field: True for field in PASS_FIELDS},
            "zero_residue": True,
            "network_closed_before_installed_execution": True,
            "credential_values_present": False,
            "private_environment_values_present": False,
            "release_claim": "signed-package-candidate",
        }
        path = root / "standard-user-acceptance.json"
        self.rewrite(path, value)
        return path, value

    def rewrite(self, path: Path, value: dict) -> None:
        path.write_text(json.dumps(value), encoding="utf-8")
        path.chmod(0o600)

    def verify(self, root: Path) -> tuple[tuple, Path, dict]:
        bundle = self.bundle(root)
        path, value = self.acceptance(root, bundle)
        return bundle, path, value

    def test_source_contract_is_closed_and_blocked(self) -> None:
        self.assertEqual(validate_sources(), [])
        report = build_source_report()
        self.assertEqual(report["contract"]["acceptance_phase_count"], 9)
        self.assertTrue(all(value is False for value in report["execution"].values()))
        self.assertTrue(all(value is False for value in report["claims"].values()))
        self.assertEqual(len(report["remaining_blockers"]), 4)
        self.assertEqual(len(SOURCE_PATHS), 6)

    def test_valid_nine_phase_record_verifies_content_free(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            bundle, path, _value = self.verify(Path(temporary))
            failures, result = validate_acceptance(*bundle[:4], path)
            self.assertEqual(failures, [])
            self.assertIsNotNone(result)
            assert result is not None
            self.assertEqual(result["phase_count"], 9)
            self.assertEqual(result["failed_phase_count"], 0)
            self.assertTrue(result["standard_user_identity_verified"])
            self.assertFalse(result["native_operations_executed_by_verifier"])
            self.assertFalse(result["macos_support_claim"])

    def test_reordered_missing_and_duplicate_phases_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            bundle, path, value = self.verify(Path(temporary))
            value["phase_order"][0], value["phase_order"][1] = (
                value["phase_order"][1],
                value["phase_order"][0],
            )
            self.rewrite(path, value)
            failures, result = validate_acceptance(*bundle[:4], path)
            self.assertIsNone(result)
            self.assertTrue(any("phase_order" in item for item in failures))
            value["phase_order"] = list(PHASE_ORDER[:-1])
            self.rewrite(path, value)
            failures, _result = validate_acceptance(*bundle[:4], path)
            self.assertTrue(any("phase_order" in item for item in failures))

    def test_identity_mutations_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            bundle, path, value = self.verify(Path(temporary))
            value["identity_class"] = "administrator"
            value["real_effective_uid_match"] = False
            value["home_owner_match"] = False
            self.rewrite(path, value)
            failures, result = validate_acceptance(*bundle[:4], path)
            self.assertIsNone(result)
            for field in ("identity_class", "real_effective_uid_match", "home_owner_match"):
                self.assertTrue(any(field in item for item in failures))

    def test_each_failed_phase_residue_and_network_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for field in (*PASS_FIELDS, "zero_residue", "network_closed_before_installed_execution"):
                bundle, path, value = self.verify(root)
                value[field] = False
                self.rewrite(path, value)
                failures, result = validate_acceptance(*bundle[:4], path)
                self.assertIsNone(result)
                self.assertTrue(any(field in item for item in failures))

    def test_release_mismatch_unknown_and_credential_fields_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            bundle, path, value = self.verify(Path(temporary))
            value["package_sha256"] = "8" * 64
            value["password"] = "forbidden"
            self.rewrite(path, value)
            failures, result = validate_acceptance(*bundle[:4], path)
            self.assertIsNone(result)
            self.assertTrue(any("field" in item for item in failures))

    def test_writable_and_linked_records_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bundle, path, _value = self.verify(root)
            path.chmod(0o666)
            failures, result = validate_acceptance(*bundle[:4], path)
            self.assertIsNone(result)
            self.assertTrue(any("unsafe evidence file" in item for item in failures))
            path.chmod(0o600)
            linked = root / "linked.json"
            os.link(path, linked)
            failures, result = validate_acceptance(*bundle[:4], path)
            self.assertIsNone(result)
            self.assertTrue(any("unsafe evidence file" in item for item in failures))


if __name__ == "__main__":
    unittest.main()
