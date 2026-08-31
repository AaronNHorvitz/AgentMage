from __future__ import annotations

import hashlib
import json
import os
import tempfile
import unittest
from pathlib import Path

from scripts.macos_lifecycle_recovery_results import (
    RECOVERY_SCENARIOS,
    SOURCE_PATHS,
    assemble_results,
    build_source_report,
    expected_case,
    log_name,
    scenario_names,
    validate_results,
    validate_sources,
)
from tests import test_macos_ipc_bookmark_receipts as release_fixture


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


class MacOSLifecycleRecoveryResultTests(unittest.TestCase):
    """Seven closed tests for S-008-RT01 native-result reconciliation."""

    def release_bundle(self, root: Path) -> tuple[Path, Path, Path, Path, dict, dict]:
        return release_fixture.MacOSIPCBookmarkReceiptTests().release_bundle(root)

    def native_evidence(
        self, root: Path, policy: dict, package: Path
    ) -> tuple[Path, dict]:
        evidence = root / "lifecycle-recovery-evidence"
        evidence.mkdir(mode=0o700)
        log_root = evidence / "recovery-logs"
        log_root.mkdir(mode=0o700)
        cases = []
        for scenario, terminal in RECOVERY_SCENARIOS:
            log = log_root / log_name(scenario)
            log.write_bytes(f"synthetic recovery evidence {scenario}\n".encode())
            log.chmod(0o600)
            cases.append(
                {
                    **expected_case(scenario, terminal),
                    "raw_report_sha256": digest(log.read_bytes()),
                }
            )
        record = {
            "schema_version": 1,
            "record_type": "macos-lifecycle-recovery-results",
            "status": "passed",
            "source_revision": policy["source_revision"],
            "version": policy["version"],
            "macos_build": policy["expected_macos_build"],
            "xcode_build": policy["expected_xcode_build"],
            "architecture": "arm64",
            "team_id": policy["team_id"],
            "package_sha256": digest(package.read_bytes()),
            "scenario_order": list(scenario_names()),
            "cases": cases,
            "credential_values_present": False,
            "private_environment_values_present": False,
        }
        path = evidence / "lifecycle-recovery-results.json"
        self.rewrite(path, record)
        return evidence, record

    def assembled(
        self, root: Path
    ) -> tuple[tuple[Path, Path, Path, Path, dict, dict], Path, Path, dict]:
        bundle = self.release_bundle(root)
        evidence, _record = self.native_evidence(root, bundle[4], bundle[2])
        failures, result = assemble_results(*bundle[:4], evidence)
        self.assertEqual(failures, [])
        self.assertIsNotNone(result)
        assert result is not None
        result_path = root / "assembled-recovery.json"
        self.rewrite(result_path, result)
        return bundle, evidence, result_path, result

    def rewrite(self, path: Path, value: dict) -> None:
        path.write_text(json.dumps(value), encoding="utf-8")
        path.chmod(0o600)

    def test_source_contract_is_closed_and_blocked(self) -> None:
        self.assertEqual(validate_sources(), [])
        report = build_source_report()
        self.assertEqual(report["status"], "prepared-source-only-blocked-macos")
        self.assertEqual(report["contract"]["scenario_count"], 8)
        self.assertEqual(report["contract"]["unsafe_result_mutation_count"], 9)
        self.assertTrue(all(value is False for value in report["execution"].values()))
        self.assertTrue(all(value is False for value in report["claims"].values()))
        self.assertEqual(len(report["remaining_blockers"]), 5)
        self.assertEqual(len(SOURCE_PATHS), 6)

    def test_valid_eight_case_matrix_reconciles_content_free(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bundle, _evidence, results_path, assembled = self.assembled(root)
            self.assertEqual(len(assembled["cases"]), 8)
            failures, accepted = validate_results(*bundle[:4], results_path)
            self.assertEqual(failures, [])
            self.assertIsNotNone(accepted)
            assert accepted is not None
            self.assertEqual(accepted["scenario_count"], 8)
            self.assertEqual(accepted["failed_cleanup_count"], 0)
            self.assertEqual(accepted["failed_prior_state_recovery_count"], 0)
            self.assertEqual(accepted["remaining_descendant_count"], 0)
            self.assertEqual(accepted["remaining_residue_count"], 0)
            self.assertFalse(accepted["native_operations_executed_by_ingestor"])
            self.assertFalse(accepted["macos_support_claim"])

    def test_missing_and_reordered_cases_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bundle = self.release_bundle(root)
            evidence, record = self.native_evidence(root, bundle[4], bundle[2])
            record["cases"][0], record["cases"][1] = (
                record["cases"][1],
                record["cases"][0],
            )
            self.rewrite(evidence / "lifecycle-recovery-results.json", record)
            failures, result = assemble_results(*bundle[:4], evidence)
            self.assertIsNone(result)
            self.assertTrue(any("matrix order" in item for item in failures))
            record["cases"] = record["cases"][:-1]
            self.rewrite(evidence / "lifecycle-recovery-results.json", record)
            failures, result = assemble_results(*bundle[:4], evidence)
            self.assertIsNone(result)
            self.assertTrue(any("case closure" in item for item in failures))

    def test_cleanup_recovery_process_and_authority_mutations_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bundle, _evidence, results_path, assembled = self.assembled(root)
            mutations = (
                (0, "cleanup_verified", False),
                (1, "prior_valid_state_recovered", False),
                (2, "descendant_process_count", 1),
                (3, "residue_count", 1),
                (4, "workspace_modified", True),
                (5, "authority_broadened", True),
            )
            for index, field, value in mutations:
                assembled["cases"][index][field] = value
            self.rewrite(results_path, assembled)
            failures, result = validate_results(*bundle[:4], results_path)
            self.assertIsNone(result)
            for _index, field, _value in mutations:
                self.assertTrue(any(field in item for item in failures))

    def test_missing_extra_and_substituted_raw_logs_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bundle = self.release_bundle(root)
            evidence, record = self.native_evidence(root, bundle[4], bundle[2])
            first = evidence / "recovery-logs" / log_name(RECOVERY_SCENARIOS[0][0])
            first.write_bytes(b"substituted")
            first.chmod(0o600)
            extra = evidence / "recovery-logs" / "extra.log"
            extra.write_bytes(b"extra")
            extra.chmod(0o600)
            failures, result = assemble_results(*bundle[:4], evidence)
            self.assertIsNone(result)
            self.assertTrue(any("log closure" in item for item in failures))
            extra.unlink()
            failures, result = assemble_results(*bundle[:4], evidence)
            self.assertIsNone(result)
            self.assertTrue(any("raw digest mismatch" in item for item in failures))
            first.unlink()
            failures, result = assemble_results(*bundle[:4], evidence)
            self.assertIsNone(result)
            self.assertTrue(any("log closure" in item for item in failures))
            self.assertEqual(record["scenario_order"], list(scenario_names()))

    def test_colliding_digests_and_unknown_fields_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bundle, _evidence, results_path, assembled = self.assembled(root)
            assembled["cases"][1]["raw_report_sha256"] = assembled["cases"][0][
                "raw_report_sha256"
            ]
            assembled["password"] = "forbidden"
            self.rewrite(results_path, assembled)
            failures, result = validate_results(*bundle[:4], results_path)
            self.assertIsNone(result)
            self.assertTrue(any("field" in item for item in failures))
            del assembled["password"]
            self.rewrite(results_path, assembled)
            failures, result = validate_results(*bundle[:4], results_path)
            self.assertIsNone(result)
            self.assertTrue(any("digests collide" in item for item in failures))

    def test_unsafe_linked_and_release_mismatched_records_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bundle = self.release_bundle(root)
            evidence, record = self.native_evidence(root, bundle[4], bundle[2])
            record["source_revision"] = "8" * 40
            record_path = evidence / "lifecycle-recovery-results.json"
            self.rewrite(record_path, record)
            record_path.chmod(0o666)
            failures, result = assemble_results(*bundle[:4], evidence)
            self.assertIsNone(result)
            self.assertTrue(any("unsafe evidence file" in item for item in failures))
            record_path.chmod(0o600)
            linked = root / "linked-lifecycle-recovery-results.json"
            os.link(record_path, linked)
            failures, result = assemble_results(*bundle[:4], evidence)
            self.assertIsNone(result)
            self.assertTrue(any("unsafe evidence file" in item for item in failures))
            linked.unlink()
            failures, result = assemble_results(*bundle[:4], evidence)
            self.assertIsNone(result)
            self.assertTrue(any("source_revision" in item for item in failures))


if __name__ == "__main__":
    unittest.main()
