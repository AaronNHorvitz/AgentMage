from __future__ import annotations

import copy
import json
import tempfile
import unittest
from pathlib import Path

from scripts.configuration_startup_evidence import (
    CLIPPY_COMMAND,
    COMMAND,
    EXPECTED_TEST,
    REPORT_PATH,
    RESULT_PREFIX,
    SOURCE_PATHS,
    build_report,
    check_artifact,
    execute_gate,
    read_json,
    validate_report,
    validate_results,
)


class ConfigurationStartupEvidenceTests(unittest.TestCase):
    def test_checked_report_is_current_and_exact(self) -> None:
        self.assertEqual(check_artifact(), [])
        report = read_json(REPORT_PATH)
        self.assertEqual(report, build_report(report["result_bundles"]))

    def test_gate_executes_exact_test_parses_seven_results_then_lints(self) -> None:
        report = read_json(REPORT_PATH)
        calls: list[tuple[str, ...]] = []

        def runner(command: tuple[str, ...], _root: Path) -> str:
            calls.append(command)
            if command == CLIPPY_COMMAND:
                return "lint passed"
            lines = [
                f"{RESULT_PREFIX}{json.dumps(item, separators=(',', ':'))}"
                for item in report["result_bundles"]
            ]
            lines.append(f"test configuration::tests::{EXPECTED_TEST} ... ok")
            return "\n".join(lines)

        self.assertEqual(
            list(execute_gate(runner=runner)), report["result_bundles"]
        )
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

    def test_result_mutations_fail_closed(self) -> None:
        results = read_json(REPORT_PATH)["result_bundles"]
        removed = copy.deepcopy(results[:-1])
        registered = copy.deepcopy(results)
        registered[0]["registered_capabilities"] = ["workspace.read"]
        wrong_dependency = copy.deepcopy(results)
        wrong_dependency[0]["dependency_sha256"] = "0" * 64
        wrong_record = copy.deepcopy(results)
        wrong_record[0]["record_sha256"] = "0" * 64
        unknown = copy.deepcopy(results)
        unknown[0]["unreviewed_extension"] = True
        for changed in (removed, registered, wrong_dependency, wrong_record, unknown):
            self.assertTrue(validate_results(changed))

    def test_summary_and_platform_overclaims_fail_closed(self) -> None:
        report = read_json(REPORT_PATH)
        activated = copy.deepcopy(report)
        activated["product_startup_activation_claim"] = "pass"
        count = copy.deepcopy(report)
        count["summary"]["registered_capability_count"] = 1
        dirty = copy.deepcopy(report)
        dirty["clean_environment"]["startup_directory_writes"] = 1
        macos = copy.deepcopy(report)
        macos["macos_execution_status"] = "pass"
        release = copy.deepcopy(report)
        release["release_readiness_claim"] = "pass"
        for changed in (activated, count, dirty, macos, release):
            self.assertTrue(validate_report(changed))

    def test_missing_source_prevents_report_validation(self) -> None:
        report = read_json(REPORT_PATH)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for relative in SOURCE_PATHS[:-1]:
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b"fixture")
            self.assertTrue(validate_report(report, root))


if __name__ == "__main__":
    unittest.main()
