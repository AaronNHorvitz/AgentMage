from __future__ import annotations

import copy
import tempfile
import unittest
from pathlib import Path

from scripts.configuration_migration_recovery_evidence import (
    CLIPPY_COMMAND,
    COMMAND,
    DURABLE_TRANSITIONS,
    EXPECTED_TEST,
    REPORT_PATH,
    SOURCE_PATHS,
    build_report,
    check_artifact,
    execute_gate,
    read_json,
    validate_report,
)


class ConfigurationMigrationRecoveryEvidenceTests(unittest.TestCase):
    def test_checked_report_is_current_and_exact(self) -> None:
        self.assertEqual(check_artifact(), [])
        self.assertEqual(read_json(REPORT_PATH), build_report(EXPECTED_TEST))

    def test_gate_executes_exact_test_then_strict_lint(self) -> None:
        calls: list[tuple[str, ...]] = []

        def runner(command: tuple[str, ...], _root: Path) -> str:
            calls.append(command)
            if command == CLIPPY_COMMAND:
                return "lint passed"
            return f"test configuration::tests::{EXPECTED_TEST} ... ok"

        self.assertEqual(execute_gate(runner=runner), EXPECTED_TEST)
        self.assertEqual(calls, [COMMAND, CLIPPY_COMMAND])

    def test_missing_or_failed_test_stops_evidence_generation(self) -> None:
        def missing(_command: tuple[str, ...], _root: Path) -> str:
            return ""

        def failed(_command: tuple[str, ...], _root: Path) -> str:
            raise RuntimeError("bounded failure")

        with self.assertRaises(RuntimeError):
            execute_gate(runner=missing)
        with self.assertRaises(RuntimeError):
            execute_gate(runner=failed)

    def test_transition_result_and_platform_overclaims_fail_closed(self) -> None:
        report = build_report(EXPECTED_TEST)
        missing_transition = copy.deepcopy(report)
        missing_transition["durable_transitions"] = list(DURABLE_TRANSITIONS[:-1])
        accepted_partial = copy.deepcopy(report)
        accepted_partial["state_selection"]["partial_configuration_allowed"] = True
        incomplete_matrix = copy.deepcopy(report)
        incomplete_matrix["summary"]["injected_interruption_count"] = 5
        missing_preimage_race = copy.deepcopy(report)
        missing_preimage_race["summary"]["concurrent_preimage_mutation_count"] = 0
        nonrepeatable = copy.deepcopy(report)
        nonrepeatable["summary"]["repeatable_rollback"] = "not-tested"
        product = copy.deepcopy(report)
        product["product_startup_migration_claim"] = "pass"
        macos = copy.deepcopy(report)
        macos["macos_execution_status"] = "pass"
        for changed in (
            missing_transition,
            accepted_partial,
            incomplete_matrix,
            missing_preimage_race,
            nonrepeatable,
            product,
            macos,
        ):
            self.assertTrue(validate_report(changed))

    def test_missing_source_prevents_report_reconstruction(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for relative in SOURCE_PATHS[:-1]:
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b"fixture")
            with self.assertRaises((OSError, ValueError)):
                build_report(EXPECTED_TEST, root)


if __name__ == "__main__":
    unittest.main()
