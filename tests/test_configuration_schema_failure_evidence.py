from __future__ import annotations

import copy
import tempfile
import unittest
from pathlib import Path

from scripts.configuration_schema_failure_evidence import (
    CLIPPY_COMMAND,
    EXPECTED_TEST,
    FAILURE_CLASSES,
    REPORT_PATH,
    SCHEMA_IDS,
    SOURCE_PATHS,
    TEST_COMMAND,
    build_report,
    check_artifact,
    execute_gate,
    read_json,
    validate_report,
)


class ConfigurationSchemaFailureEvidenceTests(unittest.TestCase):
    def test_checked_report_is_current_and_exact(self) -> None:
        self.assertEqual(check_artifact(), [])
        self.assertEqual(read_json(REPORT_PATH), build_report(EXPECTED_TEST))

    def test_gate_requires_exact_test_identity_and_lint_success(self) -> None:
        calls: list[tuple[str, ...]] = []

        def runner(command: tuple[str, ...], _root: Path) -> str:
            calls.append(command)
            if command == TEST_COMMAND:
                return f"test configuration::tests::{EXPECTED_TEST} ... ok"
            if command == CLIPPY_COMMAND:
                return "lint passed"
            raise AssertionError("unexpected command")

        self.assertEqual(execute_gate(runner=runner), EXPECTED_TEST)
        self.assertEqual(calls, [TEST_COMMAND, CLIPPY_COMMAND])

    def test_missing_test_or_command_failure_stops_evidence_generation(self) -> None:
        def missing_test(_command: tuple[str, ...], _root: Path) -> str:
            return "no matching test"

        def failed_command(_command: tuple[str, ...], _root: Path) -> str:
            raise RuntimeError("bounded failure")

        with self.assertRaises(RuntimeError):
            execute_gate(runner=missing_test)
        with self.assertRaises(RuntimeError):
            execute_gate(runner=failed_command)

    def test_matrix_weakening_and_platform_overclaims_fail_closed(self) -> None:
        report = build_report(EXPECTED_TEST)
        missing_schema = copy.deepcopy(report)
        missing_schema["schema_ids"] = list(SCHEMA_IDS[:-1])
        missing_class = copy.deepcopy(report)
        missing_class["failure_classes"] = list(FAILURE_CLASSES[:-1])
        partial = copy.deepcopy(report)
        partial["summary"]["partial_startup_case_count"] = 1
        macos = copy.deepcopy(report)
        macos["macos_execution_status"] = "pass"
        product = copy.deepcopy(report)
        product["product_startup_registration_claim"] = "pass"
        for changed in (missing_schema, missing_class, partial, macos, product):
            self.assertTrue(validate_report(changed))

    def test_missing_source_prevents_report_reconstruction(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for relative in SOURCE_PATHS[:-1]:
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b"fixture")
            with self.assertRaises(OSError):
                build_report(EXPECTED_TEST, root)


if __name__ == "__main__":
    unittest.main()
