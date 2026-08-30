from __future__ import annotations

import copy
import io
import unittest
import zipfile

from scripts.fixture_security_scan import (
    APPROVED_INERT_ARCHIVE_ENTRIES,
    APPROVED_INERT_OFFICE_RELATIONSHIPS,
    APPROVED_MALFORMED_ARCHIVES,
    CORPUS_PATH,
    EXPECTED_CATEGORIES,
    FINAL_EVIDENCE_ENVELOPE,
    FINAL_EVIDENCE_VERIFIER,
    REPORT_PATH,
    ScanMetrics,
    build_report,
    check_report,
    read_json,
    scan_archive,
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
        self.assertFalse(self.report["active_content_executed"])
        self.assertFalse(self.report["product_runtime_executed"])

    def test_scope_recurses_through_corpus_and_nested_archives(self) -> None:
        metrics = self.report["metrics"]
        self.assertGreater(metrics["files_scanned"], 20)
        self.assertGreaterEqual(metrics["archive_entries_scanned"], 77)
        self.assertGreater(metrics["archive_containers_scanned"], 4)
        self.assertGreater(metrics["nested_archives_scanned"], 0)
        self.assertGreater(metrics["text_payloads_scanned"], 0)
        self.assertGreater(metrics["python_modules_parsed"], 0)
        self.assertGreater(metrics["synthetic_canary_occurrences"], 0)
        self.assertEqual(
            metrics["synthetic_canary_occurrences"],
            metrics["approved_synthetic_canary_occurrences"],
        )
        self.assertEqual(
            metrics["approved_inert_remote_references"],
            len(APPROVED_INERT_OFFICE_RELATIONSHIPS),
        )
        self.assertEqual(
            metrics["approved_inert_archive_entries"],
            len(APPROVED_INERT_ARCHIVE_ENTRIES),
        )
        self.assertEqual(
            metrics["approved_malformed_archives"],
            len(APPROVED_MALFORMED_ARCHIVES),
        )

    def test_final_evidence_envelope_has_an_explicit_independent_verifier(self) -> None:
        scope = self.report["scope"]
        self.assertEqual(
            scope["excluded_final_evidence_envelope"],
            list(FINAL_EVIDENCE_ENVELOPE),
        )
        self.assertEqual(
            scope["final_evidence_envelope_verifier"], FINAL_EVIDENCE_VERIFIER
        )

    def test_every_seeded_prohibited_category_is_detected(self) -> None:
        cases = seeded_cases()
        self.assertEqual([item["category"] for item in cases], list(EXPECTED_CATEGORIES))
        self.assertTrue(all(item["detected"] for item in cases))

    def test_real_credential_and_private_path_shapes_are_detected(self) -> None:
        credential = b"ghp_" + (b"A" * 24)
        private_paths = (
            b"path=/" + b"home/person/private.txt",
            b"path=C:\\Users\\person\\private.txt",
        )
        observed = {
            item.category
            for item in scan_blob("fixture.txt", credential, ScanMetrics())
        }
        self.assertIn("real-credential", observed)
        for private_path in private_paths:
            observed = {
                item.category
                for item in scan_blob("fixture.txt", private_path, ScanMetrics())
            }
            self.assertIn("private-path", observed)

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

    def test_nested_executable_and_unapproved_uninspectable_packages_are_detected(self) -> None:
        inner_bytes = io.BytesIO()
        with zipfile.ZipFile(inner_bytes, "w") as inner:
            inner.writestr("payload.exe", b"inert executable-shaped payload")
        outer_bytes = io.BytesIO()
        with zipfile.ZipFile(outer_bytes, "w") as outer:
            outer.writestr("nested.zip", inner_bytes.getvalue())
        metrics = ScanMetrics()
        nested = scan_archive("evidence.zip", outer_bytes.getvalue(), metrics)
        self.assertIn("hidden-executable", {item.category for item in nested})
        self.assertEqual(metrics.nested_archives_scanned, 1)

        traversal_bytes = io.BytesIO()
        with zipfile.ZipFile(traversal_bytes, "w") as archive:
            archive.writestr("../outside.txt", b"inert traversal payload")
        traversal = scan_archive("unapproved.zip", traversal_bytes.getvalue(), ScanMetrics())
        malformed = scan_archive("unapproved.zip", b"not a ZIP", ScanMetrics())
        self.assertIn("hidden-executable", {item.category for item in traversal})
        self.assertIn("hidden-executable", {item.category for item in malformed})

    def test_only_approved_synthetic_canaries_and_reserved_invalid_urls_are_allowed(self) -> None:
        entry = "adversarial/secret-canaries/plain.txt"
        with zipfile.ZipFile(CORPUS_PATH) as archive:
            content = archive.read(entry)
        metrics = ScanMetrics()
        approved_path = f"fixtures/corpus/v1/agentmage-synthetic-corpus-v1.zip!{entry}"
        self.assertEqual(scan_blob(approved_path, content, metrics), [])
        self.assertEqual(metrics.synthetic_canary_occurrences, 1)
        self.assertEqual(metrics.approved_synthetic_canary_occurrences, 1)
        reserved = b"https://service.fixture.invalid/v1/value"
        self.assertEqual(scan_blob("fixture.txt", reserved, metrics), [])
        self.assertEqual(metrics.reserved_invalid_urls, 1)
        findings = scan_blob("artifacts/evidence.json", content, ScanMetrics())
        self.assertIn("retained-raw-canary", {item.category for item in findings})

    def test_every_inert_exception_is_bound_to_exact_bytes(self) -> None:
        entry = "adversarial/secret-canaries/plain.txt"
        with zipfile.ZipFile(CORPUS_PATH) as archive:
            canary = archive.read(entry)
        canary_path = f"fixtures/corpus/v1/agentmage-synthetic-corpus-v1.zip!{entry}"
        canary_findings = scan_blob(canary_path, canary + b"changed", ScanMetrics())
        self.assertIn("retained-raw-canary", {item.category for item in canary_findings})

        relationship_path = next(iter(APPROVED_INERT_OFFICE_RELATIONSHIPS))
        relationship = b'<Relationships><Relationship TargetMode="External"/></Relationships>'
        relationship_findings = scan_blob(
            relationship_path,
            relationship,
            ScanMetrics(),
            office_xml=True,
        )
        self.assertIn("active-formula", {item.category for item in relationship_findings})

        malformed_path = next(iter(APPROVED_MALFORMED_ARCHIVES))
        malformed_findings = scan_archive(
            malformed_path,
            b"changed malformed package",
            ScanMetrics(),
        )
        self.assertIn("hidden-executable", {item.category for item in malformed_findings})

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
