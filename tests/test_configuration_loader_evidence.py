from __future__ import annotations

import copy
import tempfile
import unittest
from pathlib import Path

from scripts.configuration_loader_evidence import (
    CLIPPY_COMMAND,
    EXPECTED_TESTS,
    REPORT_PATH,
    SOURCE_PATHS,
    TEST_COMMAND,
    build_report,
    check_artifact,
    execute_gate,
    read_json,
    validate_report,
)


class ConfigurationLoaderEvidenceTests(unittest.TestCase):
    def test_checked_report_is_current_and_exact(self) -> None:
        self.assertEqual(check_artifact(), [])
        self.assertEqual(read_json(REPORT_PATH), build_report(EXPECTED_TESTS))

    def test_gate_requires_exact_test_identity_and_lint_success(self) -> None:
        calls: list[tuple[str, ...]] = []

        def runner(command: tuple[str, ...], _root: Path) -> str:
            calls.append(command)
            if command == TEST_COMMAND:
                return "\n".join(
                    f"test configuration::tests::{name} ... ok"
                    for name in EXPECTED_TESTS
                )
            if command == CLIPPY_COMMAND:
                return "lint passed"
            raise AssertionError("unexpected command")

        self.assertEqual(execute_gate(runner=runner), EXPECTED_TESTS)
        self.assertEqual(calls, [TEST_COMMAND, CLIPPY_COMMAND])

    def test_missing_test_or_command_failure_stops_evidence_generation(self) -> None:
        def missing_test(command: tuple[str, ...], _root: Path) -> str:
            if command == TEST_COMMAND:
                return "\n".join(
                    f"test configuration::tests::{name} ... ok"
                    for name in EXPECTED_TESTS[:-1]
                )
            return ""

        def failed_command(_command: tuple[str, ...], _root: Path) -> str:
            raise RuntimeError("bounded failure")

        with self.assertRaises(RuntimeError):
            execute_gate(runner=missing_test)
        with self.assertRaises(RuntimeError):
            execute_gate(runner=failed_command)

    def test_stale_source_hash_and_product_or_macos_overclaim_fail_closed(self) -> None:
        report = build_report(EXPECTED_TESTS)
        stale = copy.deepcopy(report)
        stale["source_artifacts"][0]["sha256"] = "0" * 64
        product = copy.deepcopy(report)
        product["product_profile_activation_claim"] = "pass"
        macos = copy.deepcopy(report)
        macos["macos_execution_status"] = "pass"
        for changed in (stale, product, macos):
            self.assertTrue(validate_report(changed))

    def test_missing_source_prevents_report_reconstruction(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for relative in SOURCE_PATHS[:-1]:
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b"fixture")
            with self.assertRaises(OSError):
                build_report(EXPECTED_TESTS, root)


if __name__ == "__main__":
    unittest.main()
