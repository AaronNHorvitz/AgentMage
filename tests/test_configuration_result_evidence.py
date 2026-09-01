from __future__ import annotations

import copy
import unittest
from pathlib import Path

from scripts.configuration_result_evidence import (
    CLIPPY_COMMAND,
    CONFIGURATION_IDENTITY_FIELDS,
    EXPECTED_TESTS,
    REPORT_PATH,
    RESULT_KINDS,
    SCHEMA_COMMAND,
    SCHEMA_TEST_COMMAND,
    build_report,
    check_artifact,
    execute_gate,
    read_json,
    test_command,
    validate_report,
)


class ConfigurationResultEvidenceTests(unittest.TestCase):
    def test_checked_report_is_current_and_exact(self) -> None:
        self.assertEqual(check_artifact(), [])
        self.assertEqual(read_json(REPORT_PATH), build_report(EXPECTED_TESTS))

    def test_gate_executes_each_exact_test_lint_and_schema_validation(self) -> None:
        calls: list[tuple[str, ...]] = []

        def runner(command: tuple[str, ...], _root: Path) -> str:
            calls.append(command)
            if command in (CLIPPY_COMMAND, SCHEMA_COMMAND, SCHEMA_TEST_COMMAND):
                return "gate passed"
            name = command[-3].rsplit("::", 1)[-1]
            return f"test configuration::tests::{name} ... ok"

        self.assertEqual(execute_gate(runner=runner), EXPECTED_TESTS)
        self.assertEqual(
            calls,
            [
                *[test_command(name) for name in EXPECTED_TESTS],
                CLIPPY_COMMAND,
                SCHEMA_COMMAND,
                SCHEMA_TEST_COMMAND,
            ],
        )

    def test_missing_or_failed_test_stops_evidence_generation(self) -> None:
        def missing(_command: tuple[str, ...], _root: Path) -> str:
            return ""

        def failed(_command: tuple[str, ...], _root: Path) -> str:
            raise RuntimeError("bounded failure")

        with self.assertRaises(RuntimeError):
            execute_gate(runner=missing)
        with self.assertRaises(RuntimeError):
            execute_gate(runner=failed)

    def test_stale_hash_and_product_or_macos_overclaim_fail_closed(self) -> None:
        report = build_report(EXPECTED_TESTS)
        stale = copy.deepcopy(report)
        stale["source_artifacts"][0]["sha256"] = "0" * 64
        product = copy.deepcopy(report)
        product["product_session_execution_claim"] = "pass"
        macos = copy.deepcopy(report)
        macos["macos_execution_status"] = "pass"
        for changed in (stale, product, macos):
            self.assertTrue(validate_report(changed))

    def test_result_kind_and_configuration_identity_closures_are_exact(self) -> None:
        report = build_report(EXPECTED_TESTS)
        self.assertEqual(tuple(report["result_kinds"]), RESULT_KINDS)
        self.assertEqual(
            tuple(report["configuration_identity_fields"]),
            CONFIGURATION_IDENTITY_FIELDS,
        )
        kind = copy.deepcopy(report)
        kind["result_kinds"].append("task")
        field = copy.deepcopy(report)
        field["configuration_identity_fields"].pop()
        self.assertTrue(validate_report(kind))
        self.assertTrue(validate_report(field))


if __name__ == "__main__":
    unittest.main()
