from __future__ import annotations

import binascii
import copy
import io
import json
import struct
import tempfile
import unittest
import xml.etree.ElementTree as element_tree
import zipfile
import zlib
from pathlib import Path, PurePosixPath

from scripts.document_fixture_generator import (
    EXPECTED_FAMILIES,
    PROFILE_PATH,
    build_report,
    check_report,
    corpus_identity,
    generate,
    materialize_specs,
    read_json,
    validate_profile,
    validate_report,
)


OFFICE_FAMILIES = ("word", "spreadsheets", "presentations")


class DocumentFixtureGeneratorTests(unittest.TestCase):
    def setUp(self) -> None:
        self.profile = read_json(PROFILE_PATH)
        self.specs = materialize_specs(self.profile)
        self.by_family = {item.family: item for item in self.specs.values()}

    def test_checked_in_profile_and_report_are_current(self) -> None:
        self.assertEqual(validate_profile(self.profile), [])
        self.assertEqual(check_report(), [])

    def test_all_ten_required_families_have_one_canonical_fixture(self) -> None:
        expected = {family for family, _media_type, _path in EXPECTED_FAMILIES}
        self.assertEqual(set(self.by_family), expected)
        self.assertEqual(len(self.specs), 10)
        for family, media_type, path in EXPECTED_FAMILIES:
            fixture = self.specs[path]
            self.assertEqual(fixture.family, family)
            self.assertEqual(fixture.media_type, media_type)
            self.assertTrue(fixture.content)

    def test_generation_is_repeatable_non_executable_and_refuses_overwrite(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            first = Path(temporary) / "first"
            second = Path(temporary) / "second"
            first_result = generate(self.profile, first)
            second_result = generate(self.profile, second)
            self.assertEqual(first_result, second_result)
            self.assertEqual(
                {
                    path.relative_to(first): path.read_bytes()
                    for path in first.rglob("*")
                    if path.is_file()
                },
                {
                    path.relative_to(second): path.read_bytes()
                    for path in second.rglob("*")
                    if path.is_file()
                },
            )
            for path in first.rglob("*"):
                if path.is_file():
                    self.assertFalse(path.is_symlink())
                    self.assertEqual(path.stat().st_mode & 0o111, 0)
            with self.assertRaises(FileExistsError):
                generate(self.profile, first)

    def test_text_json_notebook_and_log_are_valid_and_inert(self) -> None:
        text = self.by_family["text"].content.decode("utf-8")
        record = json.loads(self.by_family["json"].content)
        notebook = json.loads(self.by_family["notebooks"].content)
        log = self.by_family["logs"].content.decode("utf-8")
        self.assertIn("Synthetic planning meeting", text)
        self.assertEqual(record["status"], "synthetic")
        self.assertEqual((notebook["nbformat"], notebook["nbformat_minor"]), (4, 5))
        self.assertFalse(any(cell["cell_type"] == "code" for cell in notebook["cells"]))
        self.assertTrue(all("outputs" not in cell for cell in notebook["cells"]))
        self.assertTrue(all("execution_count" not in cell for cell in notebook["cells"]))
        self.assertEqual(len(log.splitlines()), 3)

    def test_office_packages_have_valid_safe_internal_structure(self) -> None:
        required = {
            "word": {"[Content_Types].xml", "_rels/.rels", "word/document.xml"},
            "spreadsheets": {
                "[Content_Types].xml",
                "_rels/.rels",
                "xl/workbook.xml",
                "xl/_rels/workbook.xml.rels",
                "xl/worksheets/sheet1.xml",
            },
            "presentations": {
                "[Content_Types].xml",
                "_rels/.rels",
                "ppt/presentation.xml",
                "ppt/_rels/presentation.xml.rels",
                "ppt/slides/slide1.xml",
            },
        }
        for family in OFFICE_FAMILIES:
            with self.subTest(family=family):
                with zipfile.ZipFile(io.BytesIO(self.by_family[family].content)) as archive:
                    self.assertEqual(set(archive.namelist()), required[family])
                    self.assertIsNone(archive.testzip())
                    for info in archive.infolist():
                        path = PurePosixPath(info.filename)
                        self.assertFalse(path.is_absolute())
                        self.assertNotIn("..", path.parts)
                        self.assertEqual((info.external_attr >> 16) & 0o111, 0)
                        if info.filename.endswith((".xml", ".rels")):
                            element_tree.fromstring(archive.read(info.filename))

    def test_office_packages_have_no_active_or_external_content(self) -> None:
        prohibited_names = (
            ".exe",
            ".dll",
            ".dylib",
            ".so",
            ".js",
            ".vbs",
            ".cmd",
            ".bat",
            "vbaproject.bin",
        )
        for family in OFFICE_FAMILIES:
            with self.subTest(family=family):
                with zipfile.ZipFile(io.BytesIO(self.by_family[family].content)) as archive:
                    self.assertFalse(
                        any(info.filename.lower().endswith(prohibited_names) for info in archive.infolist())
                    )
                    for info in archive.infolist():
                        content = archive.read(info.filename).lower()
                        self.assertNotIn(b"targetmode=\"external\"", content)
                        self.assertNotIn(b"<f>", content)
                        self.assertNotIn(b"<f ", content)
                        self.assertNotIn(b"javascript", content)

    def test_pdf_cross_reference_points_to_each_object(self) -> None:
        content = self.by_family["portable-document-format"].content
        self.assertTrue(content.startswith(b"%PDF-1.4"))
        self.assertTrue(content.endswith(b"%%EOF\n"))
        xref_offset = int(content.rsplit(b"startxref\n", 1)[1].splitlines()[0])
        self.assertTrue(content[xref_offset:].startswith(b"xref\n"))
        xref_lines = content[xref_offset:].splitlines()
        self.assertEqual(xref_lines[1], b"0 6")
        for object_number, line in enumerate(xref_lines[3:8], start=1):
            offset = int(line[:10])
            self.assertTrue(content[offset:].startswith(f"{object_number} 0 obj\n".encode()))
        for prohibited in (b"/JavaScript", b"/JS", b"/URI", b"/Launch", b"/EmbeddedFile"):
            self.assertNotIn(prohibited, content)

    def test_png_chunks_crc_dimensions_and_scanlines_are_valid(self) -> None:
        content = self.by_family["images"].content
        self.assertTrue(content.startswith(b"\x89PNG\r\n\x1a\n"))
        position = 8
        chunks: list[tuple[bytes, bytes]] = []
        while position < len(content):
            size = struct.unpack(">I", content[position : position + 4])[0]
            chunk_type = content[position + 4 : position + 8]
            data = content[position + 8 : position + 8 + size]
            expected_crc = struct.unpack(">I", content[position + 8 + size : position + 12 + size])[0]
            self.assertEqual(binascii.crc32(chunk_type + data) & 0xFFFFFFFF, expected_crc)
            chunks.append((chunk_type, data))
            position += 12 + size
        self.assertEqual([kind for kind, _data in chunks], [b"IHDR", b"IDAT", b"IEND"])
        self.assertEqual(struct.unpack(">II", chunks[0][1][:8]), (2, 2))
        scanlines = zlib.decompress(chunks[1][1])
        self.assertEqual(len(scanlines), 14)
        self.assertEqual((scanlines[0], scanlines[7]), (0, 0))

    def test_archive_contains_only_safe_non_executable_records(self) -> None:
        with zipfile.ZipFile(io.BytesIO(self.by_family["archives"].content)) as archive:
            self.assertEqual(set(archive.namelist()), {"README.txt", "records/items.json"})
            self.assertIsNone(archive.testzip())
            for info in archive.infolist():
                path = PurePosixPath(info.filename)
                self.assertFalse(path.is_absolute())
                self.assertNotIn("..", path.parts)
                self.assertEqual((info.external_attr >> 16) & 0o111, 0)
            self.assertTrue(json.loads(archive.read("records/items.json"))["synthetic"])

    def test_fixture_bytes_exclude_private_paths_and_credential_shapes(self) -> None:
        combined = b"\n".join(item.content for item in self.specs.values())
        for prohibited in (
            b"/home/",
            b"/Users/",
            b"AKIA",
            b"BEGIN PRIVATE KEY",
            b"ghp_",
            b"Bearer ",
        ):
            self.assertNotIn(prohibited, combined)

    def test_seed_change_changes_document_corpus_identity(self) -> None:
        mutated = copy.deepcopy(self.profile)
        mutated["seed"] = "different-synthetic-document-seed"
        self.assertNotEqual(
            corpus_identity(self.specs),
            corpus_identity(materialize_specs(mutated)),
        )

    def test_weakened_content_safety_or_parser_claim_is_rejected(self) -> None:
        active = copy.deepcopy(self.profile)
        active["content_safety_contract"]["macros"] = True
        claimed = copy.deepcopy(self.profile)
        claimed["product_parser_support_claim"] = "supported"
        self.assertTrue(validate_profile(active))
        self.assertTrue(validate_profile(claimed))

    def test_report_is_hash_only_current_and_cannot_claim_versioned_corpus(self) -> None:
        report = build_report()
        self.assertFalse(report["corpus_preview"]["persisted"])
        self.assertEqual(report["corpus_preview"]["family_count"], 10)
        self.assertTrue(all("content" not in item for item in report["corpus_preview"]["files"]))
        mutated = copy.deepcopy(report)
        mutated["versioned_corpus_status"] = "complete"
        self.assertTrue(validate_report(mutated))


if __name__ == "__main__":
    unittest.main()
