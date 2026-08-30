from __future__ import annotations

import copy
import io
import json
import unittest
import zipfile

from scripts import artifact_evaluation_document_fixtures as fixtures


class ArtifactEvaluationDocumentFixtureTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.manifest = json.loads(fixtures.MANIFEST_PATH.read_text(encoding="utf-8"))
        cls.by_category = {item["category"]: item for item in cls.manifest["cases"]}

    def test_required_format_variant_matrix_is_exact(self) -> None:
        self.assertEqual([item["category"] for item in self.manifest["cases"]], list(fixtures.CATEGORIES))
        self.assertEqual(sum(item["format"] == "pdf" for item in self.manifest["cases"]), 6)
        self.assertEqual(sum(item["format"] == "docx" for item in self.manifest["cases"]), 4)
        self.assertEqual(sum(item["format"] == "xlsx" for item in self.manifest["cases"]), 4)

    def test_pdf_modes_have_real_distinct_structural_markers(self) -> None:
        entries = fixtures.payloads()
        self.assertIn(b"Synthetic status report", entries["documents/pdf/digital.pdf"])
        self.assertIn(b"/Subtype /Image", entries["documents/pdf/scanned.pdf"])
        self.assertIn(b"/Count 2", entries["documents/pdf/mixed.pdf"])
        self.assertIn(b"/Encrypt 5 0 R", entries["documents/pdf/encrypted.pdf"])
        self.assertFalse(entries["documents/pdf/malformed.pdf"].endswith(b"%%EOF\n"))

    def test_office_structured_and_relationship_hostile_packages_are_exact(self) -> None:
        entries = fixtures.payloads()
        with zipfile.ZipFile(io.BytesIO(entries["documents/docx/structured.docx"])) as archive:
            self.assertIn("word/document.xml", archive.namelist())
        with zipfile.ZipFile(io.BytesIO(entries["documents/xlsx/structured.xlsx"])) as archive:
            self.assertIn("xl/worksheets/sheet1.xml", archive.namelist())
        for name in ("documents/docx/relationship-hostile.docx", "documents/xlsx/relationship-hostile.xlsx"):
            with zipfile.ZipFile(io.BytesIO(entries[name])) as archive:
                combined = b"\n".join(archive.read(item) for item in archive.namelist())
                self.assertIn(b"TargetMode=\"External\"", combined)
                self.assertIn(b"https://fixture.invalid/never-fetch", combined)

    def test_malformed_office_packages_fail_crc_or_container_open(self) -> None:
        entries = fixtures.payloads()
        for name in ("documents/docx/malformed.docx", "documents/xlsx/malformed.xlsx"):
            with self.assertRaises(zipfile.BadZipFile):
                with zipfile.ZipFile(io.BytesIO(entries[name])) as archive:
                    archive.testzip()

    def test_oversized_recipes_are_exact_streamed_identities(self) -> None:
        for format_id in ("pdf", "docx", "xlsx"):
            item = self.by_category[f"{format_id}_oversized"]
            self.assertEqual(item["byte_identity"], fixtures.oversized_identity(format_id))
            self.assertEqual(item["byte_identity"]["byte_length"], 32 * 1024 * 1024)
            self.assertIsNone(item["archive_entry"])

    def test_expected_sections_and_provenance_are_source_bound(self) -> None:
        self.assertEqual(len(self.by_category["pdf_digital"]["expected_sections"]), 1)
        self.assertEqual(len(self.by_category["pdf_scanned"]["expected_sections"]), 1)
        self.assertEqual(len(self.by_category["pdf_mixed"]["expected_sections"]), 2)
        self.assertEqual(len(self.by_category["docx_structured"]["expected_sections"]), 1)
        self.assertEqual(len(self.by_category["xlsx_structured"]["expected_sections"]), 1)
        for item in self.manifest["cases"]:
            self.assertEqual(item["provenance"]["source_sha256"], item["byte_identity"]["sha256"])
            self.assertFalse(item["provenance"]["active_content_executed"])
            self.assertFalse(item["provenance"]["external_relationship_fetched"])

    def test_mutation_effect_and_parser_overclaims_fail(self) -> None:
        changed = copy.deepcopy(self.manifest)
        changed["cases"].pop()
        self.assertTrue(fixtures.validate_manifest(changed))
        changed = copy.deepcopy(self.manifest)
        changed["cases"][-1]["provenance"]["external_relationship_fetched"] = True
        self.assertTrue(fixtures.validate_manifest(changed))
        changed = copy.deepcopy(self.manifest)
        changed["product_parser_support_claim"] = "supported"
        self.assertTrue(fixtures.validate_manifest(changed))

    def test_checked_corpus_matches_the_reproducible_builder(self) -> None:
        self.assertEqual(fixtures.check(), [])


if __name__ == "__main__":
    unittest.main()
