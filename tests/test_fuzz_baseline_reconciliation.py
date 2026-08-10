from __future__ import annotations

import copy
import tempfile
import unittest
from pathlib import Path

from scripts.fuzz_baseline_reconciliation import (
    REPORT_PATH,
    build_report,
    check_report,
    prepare_clean_root,
    read_json,
    validate_report,
)


class FuzzBaselineReconciliationTests(unittest.TestCase):
    def test_checked_report_is_current_and_two_clean_runs_reconcile(self) -> None:
        self.assertEqual(check_report(), [])
        report = build_report()
        self.assertEqual(read_json(REPORT_PATH), report)
        self.assertEqual(report["runs"][0]["reconciliation_sha256"], report["runs"][1]["reconciliation_sha256"])
        self.assertTrue(all(report["comparison"].values()))

    def test_clean_root_contains_only_allowlisted_inputs(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            paths = prepare_clean_root(root)
            actual = tuple(
                sorted(
                    path.relative_to(root).as_posix()
                    for path in root.rglob("*")
                    if path.is_file()
                )
            )
        self.assertEqual(actual, paths)
        self.assertEqual(len(paths), 13)

    def test_nonempty_clean_root_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "unexpected.txt").write_text("synthetic", encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "existing empty directory"):
                prepare_clean_root(root)

    def test_omitted_and_duplicate_results_fail_closed(self) -> None:
        report = build_report()
        omitted = copy.deepcopy(report)
        omitted["runs"][0]["results"].pop()
        duplicated = copy.deepcopy(report)
        duplicated["runs"][0]["results"][1] = copy.deepcopy(
            duplicated["runs"][0]["results"][0]
        )
        self.assertTrue(validate_report(omitted))
        self.assertTrue(validate_report(duplicated))

    def test_pass_conversion_and_timeout_conversion_fail_closed(self) -> None:
        report = build_report()
        converted = copy.deepcopy(report)
        converted["runs"][0]["results"][0]["result_status"] = "pass"
        timeout = copy.deepcopy(report)
        timeout_result = next(
            item
            for item in timeout["runs"][0]["results"]
            if item["timeout_resource_event"]["event"] == "run-timeout"
        )
        timeout_result["result_status"] = "pass"
        self.assertTrue(validate_report(converted))
        self.assertTrue(validate_report(timeout))

    def test_coverage_and_evidence_hash_mutations_fail_closed(self) -> None:
        report = build_report()
        coverage = copy.deepcopy(report)
        coverage["runs"][1]["results"][0]["coverage_sha256"] = "0" * 64
        evidence = copy.deepcopy(report)
        evidence["runs"][1]["results"][0]["evidence_sha256"] = "0" * 64
        self.assertTrue(validate_report(coverage))
        self.assertTrue(validate_report(evidence))

    def test_private_data_side_effect_and_macos_overclaims_fail_closed(self) -> None:
        report = build_report()
        private = copy.deepcopy(report)
        private["private_user_data_used"] = True
        network = copy.deepcopy(report)
        network["network_used"] = True
        macos = copy.deepcopy(report)
        macos["macos_execution_status"] = "pass"
        for changed in (private, network, macos):
            self.assertTrue(validate_report(changed))


if __name__ == "__main__":
    unittest.main()
