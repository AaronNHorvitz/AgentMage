#!/usr/bin/env python3
"""Build and verify Story 2.3.1.2 PDF, DOCX, and XLSX fixtures."""

from __future__ import annotations

import argparse
import copy
import hashlib
import io
import json
import sys
import zipfile
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.document_fixture_generator import (  # noqa: E402
    deterministic_zip,
    pdf_document,
    spreadsheet_document,
    word_document,
)
from scripts.engineering_artifact_admission_fixtures import (  # noqa: E402
    ZERO_SHA256,
    build_archive,
    canonical_json,
    sealed,
    sha256_bytes,
    validate_archive,
    write_atomic,
)


FIXTURE_DIR: Final = ROOT / "fixtures" / "artifact-evaluation" / "v1"
ARCHIVE_PATH: Final = FIXTURE_DIR / "document-variant-corpus-v1.zip"
MANIFEST_PATH: Final = FIXTURE_DIR / "document-variant-manifest.json"
OVERSIZED_BYTES: Final = 32 * 1024 * 1024
OVERSIZED_SEED: Final = "agentmage-story-2.3-document-oversize-v1"
CATEGORIES: Final = (
    "pdf_digital",
    "pdf_scanned",
    "pdf_mixed",
    "pdf_encrypted",
    "pdf_malformed",
    "pdf_oversized",
    "docx_structured",
    "docx_malformed",
    "docx_oversized",
    "docx_relationship_hostile",
    "xlsx_structured",
    "xlsx_malformed",
    "xlsx_oversized",
    "xlsx_relationship_hostile",
)


def pdf_from_objects(objects: list[bytes], *, encrypt_object: int | None = None) -> bytes:
    document = bytearray(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n")
    offsets = [0]
    for number, body in enumerate(objects, start=1):
        offsets.append(len(document))
        document.extend(f"{number} 0 obj\n".encode("ascii"))
        document.extend(body)
        document.extend(b"\nendobj\n")
    xref_offset = len(document)
    document.extend(f"xref\n0 {len(objects) + 1}\n".encode("ascii"))
    document.extend(b"0000000000 65535 f \n")
    for offset in offsets[1:]:
        document.extend(f"{offset:010d} 00000 n \n".encode("ascii"))
    encrypt = "" if encrypt_object is None else f" /Encrypt {encrypt_object} 0 R"
    document.extend(
        (
            f"trailer\n<< /Size {len(objects) + 1} /Root 1 0 R{encrypt} >>\n"
            f"startxref\n{xref_offset}\n%%EOF\n"
        ).encode("ascii")
    )
    return bytes(document)


def scanned_pdf() -> bytes:
    image = b"\x00\x7f\xff\x40"
    stream = b"q 100 0 0 100 72 620 cm /Im1 Do Q\n"
    return pdf_from_objects(
        [
            b"<< /Type /Catalog /Pages 2 0 R >>",
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /XObject << /Im1 4 0 R >> >> /Contents 5 0 R >>",
            b"<< /Type /XObject /Subtype /Image /Width 2 /Height 2 /ColorSpace /DeviceGray /BitsPerComponent 8 /Length 4 >>\nstream\n" + image + b"\nendstream",
            b"<< /Length " + str(len(stream)).encode() + b" >>\nstream\n" + stream + b"endstream",
        ]
    )


def mixed_pdf() -> bytes:
    text = b"BT /F1 12 Tf 72 720 Td (Public synthetic digital page) Tj ET\n"
    image = b"\x20\xe0\x60\xa0"
    draw = b"q 100 0 0 100 72 620 cm /Im1 Do Q\n"
    return pdf_from_objects(
        [
            b"<< /Type /Catalog /Pages 2 0 R >>",
            b"<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 5 0 R >> >> /Contents 7 0 R >>",
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /XObject << /Im1 6 0 R >> >> /Contents 8 0 R >>",
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
            b"<< /Type /XObject /Subtype /Image /Width 2 /Height 2 /ColorSpace /DeviceGray /BitsPerComponent 8 /Length 4 >>\nstream\n" + image + b"\nendstream",
            b"<< /Length " + str(len(text)).encode() + b" >>\nstream\n" + text + b"endstream",
            b"<< /Length " + str(len(draw)).encode() + b" >>\nstream\n" + draw + b"endstream",
        ]
    )


def encrypted_pdf() -> bytes:
    stream = b"synthetic encrypted payload marker"
    return pdf_from_objects(
        [
            b"<< /Type /Catalog /Pages 2 0 R >>",
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R >>",
            b"<< /Length " + str(len(stream)).encode() + b" >>\nstream\n" + stream + b"\nendstream",
            b"<< /Filter /Standard /V 1 /R 2 /O <00112233> /U <44556677> /P -4 >>",
        ],
        encrypt_object=5,
    )


def external_relationship(base: bytes, relationship_path: str) -> bytes:
    with zipfile.ZipFile(io.BytesIO(base)) as archive:
        entries = {name: archive.read(name) for name in archive.namelist()}
    relationship = b"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="hostileExternal" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://fixture.invalid/never-fetch" TargetMode="External"/>
</Relationships>
"""
    if relationship_path in entries:
        original = entries[relationship_path]
        entries[relationship_path] = original.replace(b"</Relationships>", relationship.split(b"\n", 1)[1].replace(b"<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\n", b"").replace(b"</Relationships>\n", b"") + b"</Relationships>")
    else:
        entries[relationship_path] = relationship
    return deterministic_zip(entries)


def payloads() -> dict[str, bytes]:
    digital = pdf_document("23a1d1f17a2b")
    docx = word_document("8a2fbc2e1d09")
    xlsx = spreadsheet_document("4be9c1aa1e70")
    entries = {
        "documents/pdf/digital.pdf": digital,
        "documents/pdf/scanned.pdf": scanned_pdf(),
        "documents/pdf/mixed.pdf": mixed_pdf(),
        "documents/pdf/encrypted.pdf": encrypted_pdf(),
        "documents/pdf/malformed.pdf": digital[:-31],
        "documents/docx/structured.docx": docx,
        "documents/docx/malformed.docx": docx[:-23],
        "documents/docx/relationship-hostile.docx": external_relationship(docx, "word/_rels/document.xml.rels"),
        "documents/xlsx/structured.xlsx": xlsx,
        "documents/xlsx/malformed.xlsx": xlsx[:-23],
        "documents/xlsx/relationship-hostile.xlsx": external_relationship(xlsx, "xl/externalLinks/_rels/externalLink1.xml.rels"),
    }
    return dict(sorted(entries.items()))


def oversized_identity(format_id: str) -> dict[str, Any]:
    block = f"{OVERSIZED_SEED}:{format_id}:public-synthetic-block\n".encode("ascii")
    digest = hashlib.sha256()
    remaining = OVERSIZED_BYTES
    while remaining:
        chunk = block[: min(remaining, len(block))]
        digest.update(chunk)
        remaining -= len(chunk)
    return {"byte_length": OVERSIZED_BYTES, "sha256": digest.hexdigest()}


def expected_sections(category: str, content: bytes | None) -> list[dict[str, Any]]:
    if category == "pdf_digital":
        return [{"section_id": "page-1-text", "coordinate": "pdf-page-1", "state": "exact_text", "content_sha256": sha256_bytes(b"Synthetic status report 23a1d1f17a2b")}]
    if category == "pdf_scanned":
        return [{"section_id": "page-1-image", "coordinate": "pdf-page-1", "state": "ocr_required", "content_sha256": sha256_bytes(b"\x00\x7f\xff\x40")}]
    if category == "pdf_mixed":
        return [
            {"section_id": "page-1-text", "coordinate": "pdf-page-1", "state": "exact_text", "content_sha256": sha256_bytes(b"Public synthetic digital page")},
            {"section_id": "page-2-image", "coordinate": "pdf-page-2", "state": "ocr_required", "content_sha256": sha256_bytes(b"\x20\xe0\x60\xa0")},
        ]
    if category == "docx_structured":
        with zipfile.ZipFile(io.BytesIO(content)) as archive:
            part = archive.read("word/document.xml")
        return [{"section_id": "word-document-root", "coordinate": "package:word/document.xml", "state": "exact_structure", "content_sha256": sha256_bytes(part)}]
    if category == "xlsx_structured":
        with zipfile.ZipFile(io.BytesIO(content)) as archive:
            part = archive.read("xl/worksheets/sheet1.xml")
        return [{"section_id": "sheet-ledger", "coordinate": "package:xl/worksheets/sheet1.xml", "state": "exact_structure", "content_sha256": sha256_bytes(part)}]
    return []


def cases() -> list[dict[str, Any]]:
    entries = payloads()
    mapping = (
        ("pdf_digital", "documents/pdf/digital.pdf", "captured", []),
        ("pdf_scanned", "documents/pdf/scanned.pdf", "partial", ["ocr_required"]),
        ("pdf_mixed", "documents/pdf/mixed.pdf", "partial", ["ocr_required_for_page_2"]),
        ("pdf_encrypted", "documents/pdf/encrypted.pdf", "unsupported", ["encrypted_content_no_bypass"]),
        ("pdf_malformed", "documents/pdf/malformed.pdf", "failed", ["malformed_truncated_pdf"]),
        ("pdf_oversized", None, "denied", ["source_byte_ceiling_exceeded"]),
        ("docx_structured", "documents/docx/structured.docx", "captured", []),
        ("docx_malformed", "documents/docx/malformed.docx", "failed", ["malformed_truncated_package"]),
        ("docx_oversized", None, "denied", ["source_byte_ceiling_exceeded"]),
        ("docx_relationship_hostile", "documents/docx/relationship-hostile.docx", "denied", ["external_relationship_blocked"]),
        ("xlsx_structured", "documents/xlsx/structured.xlsx", "captured", []),
        ("xlsx_malformed", "documents/xlsx/malformed.xlsx", "failed", ["malformed_truncated_package"]),
        ("xlsx_oversized", None, "denied", ["source_byte_ceiling_exceeded"]),
        ("xlsx_relationship_hostile", "documents/xlsx/relationship-hostile.xlsx", "denied", ["external_relationship_blocked"]),
    )
    result = []
    for category, entry, disposition, warnings in mapping:
        format_id = category.split("_", 1)[0]
        generated = entry is None
        content = None if generated else entries[entry]
        byte_identity = oversized_identity(format_id) if generated else {"byte_length": len(content), "sha256": sha256_bytes(content)}
        result.append(
            {
                "fixture_id": f"eval-{category.replace('_', '-')}-v1",
                "category": category,
                "format": format_id,
                "archive_entry": entry,
                "generated_from_seed": generated,
                "byte_identity": byte_identity,
                "expected_disposition": disposition,
                "expected_sections": expected_sections(category, content),
                "provenance": {
                    "generator_id": "artifact-evaluation-document-fixtures",
                    "generator_version": "1.0.0",
                    "source_sha256": byte_identity["sha256"],
                    "warning_codes": warnings,
                    "active_content_executed": False,
                    "external_relationship_fetched": False,
                },
            }
        )
    return result


def build_suite() -> tuple[bytes, dict[str, Any]]:
    entries = payloads()
    archive = build_archive(entries)
    value = sealed(
        {
            "schema_version": 1,
            "corpus_id": "artifact-evaluation-document-variants-v1",
            "task_id": "2.3.1.2",
            "generated_on": "2026-08-30",
            "status": "synthetic-document-fixture-contract",
            "synthetic_only": True,
            "network_access": False,
            "product_parser_support_claim": "none",
            "required_categories": list(CATEGORIES),
            "case_count": len(CATEGORIES),
            "archive": {"path": str(ARCHIVE_PATH.relative_to(ROOT)), "format": "zip-stored", "entry_count": len(entries), "byte_length": len(archive), "sha256": sha256_bytes(archive)},
            "oversized_recipe": {"seed": OVERSIZED_SEED, "algorithm": "stream-repeated-format-block-v1", "target_byte_length": OVERSIZED_BYTES, "persisted_in_archive": False},
            "generator": {"path": "scripts/artifact_evaluation_document_fixtures.py", "sha256": sha256_bytes(Path(__file__).read_bytes())},
            "cases": cases(),
            "manifest_sha256": ZERO_SHA256,
        },
        "manifest_sha256",
    )
    return archive, value


def validate_manifest(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["document variant manifest must be an object"]
    failures: list[str] = []
    unhashed = copy.deepcopy(value)
    recorded = unhashed.get("manifest_sha256")
    unhashed["manifest_sha256"] = ZERO_SHA256
    if recorded != sha256_bytes(canonical_json(unhashed)):
        failures.append("document variant manifest self-hash is invalid")
    observed = value.get("cases", [])
    if [item.get("category") for item in observed if isinstance(item, dict)] != list(CATEGORIES):
        failures.append("document variant categories are incomplete or reordered")
    if value.get("case_count") != len(CATEGORIES):
        failures.append("document variant case count is incomplete")
    for item in observed if isinstance(observed, list) else []:
        provenance = item.get("provenance", {})
        if provenance.get("source_sha256") != item.get("byte_identity", {}).get("sha256"):
            failures.append(f"document provenance identity drifted: {item.get('category')}")
        if provenance.get("active_content_executed") is not False or provenance.get("external_relationship_fetched") is not False:
            failures.append(f"document fixture gained an active effect: {item.get('category')}")
        if item.get("expected_disposition") in {"denied", "failed", "unsupported"} and item.get("expected_sections"):
            failures.append(f"denied document invented sections: {item.get('category')}")
    if value.get("network_access") is not False or value.get("product_parser_support_claim") != "none":
        failures.append("document corpus widened network or parser-support scope")
    return failures


def check() -> list[str]:
    try:
        expected_archive, expected_manifest = build_suite()
        actual_archive = ARCHIVE_PATH.read_bytes()
        actual_manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    except (OSError, ValueError, json.JSONDecodeError, zipfile.BadZipFile) as error:
        return [f"cannot validate document variant corpus: {error}"]
    failures = validate_archive(actual_archive, payloads()) + validate_manifest(actual_manifest)
    if actual_archive != expected_archive:
        failures.append("checked document variant archive is stale or corrupt")
    if actual_manifest != expected_manifest:
        failures.append("checked document variant manifest is stale, incomplete, or widened")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    if args.write:
        archive, manifest = build_suite()
        write_atomic(ARCHIVE_PATH, archive)
        write_atomic(MANIFEST_PATH, canonical_json(manifest))
    failures = check()
    if failures:
        for failure in failures:
            print(f"Document variant fixture validation failed: {failure}", file=sys.stderr)
        return 1
    print(f"Validated {len(CATEGORIES)} PDF, DOCX, and XLSX evaluation fixtures")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
