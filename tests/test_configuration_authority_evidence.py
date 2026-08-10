from __future__ import annotations

import copy
import unittest
from pathlib import Path

from scripts.configuration_authority_evidence import (
    AUTHORITY_DIMENSIONS,
    CLIPPY_COMMAND,
    EXPECTED_TESTS,
    REPORT_PATH,
    SOURCES,
    build_report,
    check_artifact,
    execute_gate,
    read_json,
    test_command,
    validate_report,
)


class ConfigurationAuthorityEvidenceTests(unittest.TestCase):
    def test_checked_report_is_current_and_exact(self) -> None:
        self.assertEqual(check_artifact(), [])
        self.assertEqual(read_json(REPORT_PATH), build_report(EXPECTED_TESTS))

    def test_gate_executes_each_exact_test_then_strict_lint(self) -> None:
        calls: list[tuple[str, ...]] = []

        def runner(command: tuple[str, ...], _root: Path) -> str:
            calls.append(command)
            if command == CLIPPY_COMMAND:
                return "lint passed"
            name = command[-3].rsplit("::", 1)[-1]
            return f"test configuration::tests::{name} ... ok"

        self.assertEqual(execute_gate(runner=runner), EXPECTED_TESTS)
        self.assertEqual(
            calls,
            [*[test_command(name) for name in EXPECTED_TESTS], CLIPPY_COMMAND],
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
        product["product_profile_activation_claim"] = "pass"
        macos = copy.deepcopy(report)
        macos["macos_execution_status"] = "pass"
        for changed in (stale, product, macos):
            self.assertTrue(validate_report(changed))

    def test_source_and_authority_dimension_closures_are_exact(self) -> None:
        report = build_report(EXPECTED_TESTS)
        self.assertEqual(tuple(report["untrusted_sources"]), SOURCES)
        self.assertEqual(tuple(report["authority_dimensions"]), AUTHORITY_DIMENSIONS)
        source = copy.deepcopy(report)
        source["untrusted_sources"].pop()
        dimension = copy.deepcopy(report)
        dimension["authority_dimensions"].pop()
        self.assertTrue(validate_report(source))
        self.assertTrue(validate_report(dimension))


if __name__ == "__main__":
    unittest.main()
