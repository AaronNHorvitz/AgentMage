from __future__ import annotations

import copy
import unittest

from scripts.fixture_security_scan import (
    EXPECTED_CATEGORIES,
    REPORT_PATH,
    ScanMetrics,
    build_report,
    check_report,
    read_json,
    scan_blob,
    scan_surfaces,
    seeded_cases,
    validate_report,
)


class FixtureSecurityScanTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.report = build_report()

    def test_checked_report_is_current_and_has_zero_findings(self) -> None:
        self.assertEqual(check_report(), [])
        self.assertEqual(read_json(REPORT_PATH), self.report)
        self.assertEqual(validate_report(self.report), [])
        self.assertEqual(self.report["findings"], [])
        self.assertEqual(self.report["summary"]["blocking_finding_count"], 0)

    def test_scope_recurses_through_corpus_and_nested_archives(self) -> None:
        metrics = self.report["metrics"]
        self.assertGreater(metrics["files_scanned"], 20)
        self.assertGreaterEqual(metrics["archive_entries_scanned"], 77)
        self.assertGreater(metrics["nested_archives_scanned"], 0)
        self.assertGreater(metrics["text_payloads_scanned"], 0)
        self.assertGreater(metrics["python_modules_parsed"], 0)
        self.assertGreater(metrics["synthetic_canary_occurrences"], 0)

    def test_every_seeded_prohibited_category_is_detected(self) -> None:
        cases = seeded_cases()
        self.assertEqual([item["category"] for item in cases], list(EXPECTED_CATEGORIES))
        self.assertTrue(all(item["detected"] for item in cases))

    def test_real_credential_and_private_path_shapes_are_detected(self) -> None:
        credential = b"ghp_" + (b"A" * 24)
        private_path = b"path=/" + b"home/person/private.txt"
        for category, content in (
            ("real-credential", credential),
            ("private-path", private_path),
        ):
            observed = {
                item.category
                for item in scan_blob("fixture.txt", content, ScanMetrics())
            }
            self.assertIn(category, observed)

    def test_executables_formulas_remote_urls_and_network_code_are_detected(self) -> None:
        cases = (
            (
                "hidden-executable",
                "fixture.bin",
                bytes((0x7F, 0x45, 0x4C, 0x46)),
                {},
            ),
            (
                "active-formula",
                "fixture.xlsx!xl/worksheets/sheet.xml",
                b"<f>1+1</f>",
                {"office_xml": True},
            ),
            (
                "remote-reference",
                "fixture.txt",
                b"https://example.com/value",
                {},
            ),
            ("network-behavior", "fixture.py", b"import socket\n", {}),
        )
        for category, path, content, options in cases:
            observed = {
                item.category
                for item in scan_blob(path, content, ScanMetrics(), **options)
            }
            self.assertIn(category, observed)

    def test_synthetic_canaries_and_reserved_invalid_urls_are_allowed(self) -> None:
        content = (
            b"AM_SYNTHETIC_CANARY_ALLOWED https://service.fixture.invalid/v1/value"
        )
        metrics = ScanMetrics()
        self.assertEqual(scan_blob("fixture.txt", content, metrics), [])
        self.assertEqual(metrics.synthetic_canary_occurrences, 1)
        self.assertEqual(metrics.reserved_invalid_urls, 1)

    def test_surface_scan_is_deterministic(self) -> None:
        first, first_metrics = scan_surfaces()
        second, second_metrics = scan_surfaces()
        self.assertEqual(first, second)
        self.assertEqual(first_metrics.as_record(), second_metrics.as_record())

    def test_report_rejects_findings_network_activity_and_macos_claims(self) -> None:
        finding = copy.deepcopy(self.report)
        finding["findings"] = [
            {"category": "private-path", "path": "fixture", "reason": "seed"}
        ]
        networked = copy.deepcopy(self.report)
        networked["network_calls_performed"] = 1
        unblocked = copy.deepcopy(self.report)
        unblocked["macos_execution_status"] = "pass"
        self.assertTrue(validate_report(finding))
        self.assertTrue(validate_report(networked))
        self.assertTrue(validate_report(unblocked))


if __name__ == "__main__":
    unittest.main()
