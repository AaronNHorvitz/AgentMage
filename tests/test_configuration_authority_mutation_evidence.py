from __future__ import annotations

import copy
import tempfile
import unittest
from pathlib import Path

from scripts.configuration_authority_mutation_evidence import (
    CLIPPY_COMMAND,
    EXPECTED_TESTS,
    REGISTRY_PATH,
    REPORT_PATH,
    SOURCES,
    SOURCE_PATHS,
    build_report,
    check_artifact,
    execute_gate,
    read_json,
    test_command,
    validate_registry,
    validate_report,
)


class ConfigurationAuthorityMutationEvidenceTests(unittest.TestCase):
    def test_checked_report_is_current_and_exact(self) -> None:
        self.assertEqual(check_artifact(), [])
        self.assertEqual(read_json(REPORT_PATH), build_report(EXPECTED_TESTS))

    def test_gate_executes_exact_tests_then_strict_lint(self) -> None:
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

    def test_registry_rejects_removed_duplicate_and_unknown_entries(self) -> None:
        registry = read_json(REGISTRY_PATH)
        removed = copy.deepcopy(registry)
        removed["entries"].pop()
        duplicate = copy.deepcopy(registry)
        duplicate["entries"].append(copy.deepcopy(duplicate["entries"][0]))
        unknown = copy.deepcopy(registry)
        unknown["entries"][0]["unreviewed_extension"] = True
        weakened = copy.deepcopy(registry)
        weakened["entries"][0]["expected_rejection"] = "pass"
        for changed in (removed, duplicate, unknown, weakened):
            self.assertTrue(validate_registry(changed))

    def test_result_weakening_and_platform_overclaims_fail_closed(self) -> None:
        report = build_report(EXPECTED_TESTS)
        missing_source = copy.deepcopy(report)
        missing_source["untrusted_sources"] = list(SOURCES[:-1])
        accepted = copy.deepcopy(report)
        accepted["summary"]["accepted_broadening_count"] = 1
        unsigned = copy.deepcopy(report)
        unsigned["parent_signature"]["verified_before_authority_comparison"] = False
        product = copy.deepcopy(report)
        product["product_profile_activation_claim"] = "pass"
        macos = copy.deepcopy(report)
        macos["macos_execution_status"] = "pass"
        for changed in (missing_source, accepted, unsigned, product, macos):
            self.assertTrue(validate_report(changed))

    def test_missing_source_prevents_report_reconstruction(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for relative in SOURCE_PATHS[:-1]:
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b"fixture")
            with self.assertRaises((OSError, ValueError)):
                build_report(EXPECTED_TESTS, root)


if __name__ == "__main__":
    unittest.main()
