from __future__ import annotations

import hashlib
import json
import os
import tempfile
import unittest
from pathlib import Path

from scripts.macos_ipc_handshake_refusal_results import (
    REFUSAL_CASES,
    REQUIRED_CONTROLS,
    SOURCE_PATHS,
    assemble_results,
    build_source_report,
    case_ids,
    expected_case,
    log_name,
    validate_results,
    validate_sources,
)
from tests import test_macos_ipc_bookmark_receipts as release_fixture


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


class MacOSIPCHandshakeRefusalResultTests(unittest.TestCase):
    """Seven closed tests for S-008-UT01 native-result reconciliation."""

    def release_bundle(self, root: Path) -> tuple[Path, Path, Path, Path, dict, dict]:
        return release_fixture.MacOSIPCBookmarkReceiptTests().release_bundle(root)

    def native_evidence(
        self, root: Path, policy: dict, package: Path
    ) -> tuple[Path, dict]:
        evidence = root / "handshake-refusal-evidence"
        evidence.mkdir(mode=0o700)
        log_root = evidence / "refusal-logs"
        log_root.mkdir(mode=0o700)
        cases = []
        for definition in REFUSAL_CASES:
            case_id = definition[0]
            log = log_root / log_name(case_id)
            log.write_bytes(f"synthetic refusal evidence {case_id}\n".encode())
            log.chmod(0o600)
            cases.append(
                {
                    **expected_case(definition),
                    "raw_report_sha256": digest(log.read_bytes()),
                }
            )
        record = {
            "schema_version": 1,
            "record_type": "macos-ipc-handshake-refusal-results",
            "status": "passed",
            "source_revision": policy["source_revision"],
            "version": policy["version"],
            "macos_build": policy["expected_macos_build"],
            "xcode_build": policy["expected_xcode_build"],
            "architecture": "arm64",
            "team_id": policy["team_id"],
            "package_sha256": digest(package.read_bytes()),
            "required_controls": list(REQUIRED_CONTROLS),
            "case_order": list(case_ids()),
            "cases": cases,
            "credential_values_present": False,
            "private_environment_values_present": False,
        }
        path = evidence / "ipc-handshake-refusal-results.json"
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
        result_path = root / "assembled-refusals.json"
        self.rewrite(result_path, result)
        return bundle, evidence, result_path, result

    def rewrite(self, path: Path, value: dict) -> None:
        path.write_text(json.dumps(value), encoding="utf-8")
        path.chmod(0o600)

    def test_source_contract_is_closed_and_blocked(self) -> None:
        self.assertEqual(validate_sources(), [])
        report = build_source_report()
        self.assertEqual(report["status"], "prepared-source-only-blocked-macos")
        self.assertEqual(report["contract"]["refusal_case_count"], 17)
        self.assertEqual(report["contract"]["required_control_count"], 7)
        self.assertEqual(report["contract"]["mutation_dimension_count"], 8)
        self.assertTrue(all(value is False for value in report["execution"].values()))
        self.assertTrue(all(value is False for value in report["claims"].values()))
        self.assertEqual(len(report["remaining_blockers"]), 5)
        self.assertEqual(len(SOURCE_PATHS), 9)

    def test_valid_17_case_matrix_reconciles_content_free(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bundle, _evidence, results_path, assembled = self.assembled(root)
            self.assertEqual(len(assembled["cases"]), 17)
            failures, accepted = validate_results(*bundle[:4], results_path)
            self.assertEqual(failures, [])
            self.assertIsNotNone(accepted)
            assert accepted is not None
            self.assertEqual(accepted["refusal_case_count"], 17)
            self.assertEqual(accepted["accepted_mutated_handshake_count"], 0)
            self.assertFalse(accepted["native_operations_executed_by_ingestor"])
            self.assertFalse(accepted["macos_support_claim"])

    def test_missing_duplicate_and_reordered_cases_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bundle = self.release_bundle(root)
            evidence, record = self.native_evidence(root, bundle[4], bundle[2])
            record["cases"][0], record["cases"][1] = (
                record["cases"][1],
                record["cases"][0],
            )
            self.rewrite(evidence / "ipc-handshake-refusal-results.json", record)
            failures, result = assemble_results(*bundle[:4], evidence)
            self.assertIsNone(result)
            self.assertTrue(any("matrix order" in item for item in failures))
            record["cases"] = record["cases"][:-1]
            self.rewrite(evidence / "ipc-handshake-refusal-results.json", record)
            failures, result = assemble_results(*bundle[:4], evidence)
            self.assertIsNone(result)
            self.assertTrue(any("case closure" in item for item in failures))

    def test_acceptance_failure_and_challenge_consumption_mutations_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bundle, _evidence, results_path, assembled = self.assembled(root)
            assembled["cases"][0]["mutated_handshake_refused"] = False
            assembled["cases"][1]["expected_failure"] = "macos.ipc.replay"
            assembled["cases"][2]["challenge_consumed_only_by_valid_handshake"] = False
            assembled["cases"][3]["valid_handshake_accepted_after_refusal"] = False
            self.rewrite(results_path, assembled)
            failures, result = validate_results(*bundle[:4], results_path)
            self.assertIsNone(result)
            for field in (
                "mutated_handshake_refused",
                "expected_failure",
                "challenge_consumed_only_by_valid_handshake",
                "valid_handshake_accepted_after_refusal",
            ):
                self.assertTrue(any(field in item for item in failures))

    def test_missing_extra_and_substituted_raw_logs_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bundle = self.release_bundle(root)
            evidence, record = self.native_evidence(root, bundle[4], bundle[2])
            first_id = REFUSAL_CASES[0][0]
            first_log = evidence / "refusal-logs" / log_name(first_id)
            first_log.write_bytes(b"substituted")
            first_log.chmod(0o600)
            extra = evidence / "refusal-logs" / "extra.log"
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
            self.assertEqual(record["case_order"], list(case_ids()))

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
            self.assertTrue(any("closed evidence fields" in item for item in failures))
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
            record_path = evidence / "ipc-handshake-refusal-results.json"
            self.rewrite(record_path, record)
            record_path.chmod(0o666)
            failures, result = assemble_results(*bundle[:4], evidence)
            self.assertIsNone(result)
            self.assertTrue(any("unsafe evidence file" in item for item in failures))
            record_path.chmod(0o600)
            linked = root / "linked-refusal-results.json"
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
