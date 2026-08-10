#!/usr/bin/env python3
"""Generate deterministic, inert document-format fixtures."""

from __future__ import annotations

import argparse
import binascii
import hashlib
import io
import json
import os
import shutil
import struct
import sys
import tempfile
import zipfile
import zlib
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
PROFILE_PATH = ROOT / "fixtures" / "document-fixture-profile.json"
REPORT_PATH = (
    ROOT
    / "artifacts"
    / "sprints"
    / "sprint-2"
    / "story-2.1"
    / "document-fixture-report.json"
)
EXPECTED_FAMILIES = (
    (
        "text",
        "text/plain",
        "documents/text/meeting-note.txt",
    ),
    (
        "json",
        "application/json",
        "documents/json/project-record.json",
    ),
    (
        "word",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "documents/word/status-report.docx",
    ),
    (
        "portable-document-format",
        "application/pdf",
        "documents/pdf/status-report.pdf",
    ),
    (
        "spreadsheets",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "documents/spreadsheets/project-ledger.xlsx",
    ),
    (
        "presentations",
        "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "documents/presentations/project-brief.pptx",
    ),
    (
        "images",
        "image/png",
        "documents/images/status-grid.png",
    ),
    (
        "archives",
        "application/zip",
        "documents/archives/reference-bundle.zip",
    ),
    (
        "notebooks",
        "application/x-ipynb+json",
        "documents/notebooks/analysis-notes.ipynb",
    ),
    (
        "logs",
        "text/plain",
        "documents/logs/application.log",
    ),
)
EXPECTED_SIDE_EFFECTS = {
    "executes_external_commands": False,
    "uses_network": False,
    "writes_outside_requested_destination": False,
    "overwrites_existing_destination": False,
    "creates_executable_files": False,
}
EXPECTED_CONTENT_SAFETY = {
    "macros": False,
    "scripts": False,
    "active_formulas": False,
    "external_relationships": False,
    "embedded_executables": False,
    "archive_path_traversal": False,
    "notebook_execution_history": False,
}
EXPECTED_PRIVACY = {
    "private_user_data": False,
    "real_credentials": False,
    "real_person_identity": False,
    "private_absolute_paths": False,
}
ZIP_TIMESTAMP = (2024, 1, 1, 0, 0, 0)


@dataclass(frozen=True)
class DocumentFile:
    family: str
    media_type: str
    content: bytes


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def safe_relative(value: str) -> bool:
    path = PurePosixPath(value)
    return bool(value) and not path.is_absolute() and ".." not in path.parts


def profile_families(profile: Any) -> tuple[tuple[Any, Any, Any], ...]:
    if not isinstance(profile, dict) or not isinstance(profile.get("families"), list):
        return ()
    return tuple(
        (item.get("id"), item.get("media_type"), item.get("path"))
        for item in profile["families"]
        if isinstance(item, dict)
    )


def validate_profile(profile: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(profile, dict):
        return ["document fixture profile must be an object"]
    if (
        profile.get("schema_version") != 1
        or profile.get("profile_id") != "agentmage-document-fixtures-v1"
        or profile.get("status") != "inert-synthetic-document-contract"
    ):
        failures.append("document fixture profile identity is invalid")
    if profile.get("seed") != "agentmage-document-synthetic-v1":
        failures.append("document fixture seed is not pinned")
    if profile.get("fixed_timestamp_epoch") != 1704067200:
        failures.append("document fixture timestamp is not pinned")
    if profile_families(profile) != EXPECTED_FAMILIES:
        failures.append("document fixture family closure drifted")
    if profile.get("side_effect_contract") != EXPECTED_SIDE_EFFECTS:
        failures.append("document fixture side-effect contract was weakened")
    if profile.get("content_safety_contract") != EXPECTED_CONTENT_SAFETY:
        failures.append("document fixture content-safety contract was weakened")
    if profile.get("privacy_contract") != EXPECTED_PRIVACY:
        failures.append("document fixture privacy contract was weakened")
    if profile.get("product_parser_support_claim") != "none":
        failures.append("document fixtures cannot claim product parser support")
    if any(not safe_relative(item[2]) for item in EXPECTED_FAMILIES):
        failures.append("document fixture profile contains an unsafe path")
    return failures


def fixture_marker(seed: str, family: str) -> str:
    return sha256_bytes(f"{seed}:{family}".encode("utf-8"))[:12]


def deterministic_zip(entries: dict[str, bytes]) -> bytes:
    output = io.BytesIO()
    with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_STORED) as archive:
        for name, content in sorted(entries.items()):
            if not safe_relative(name) or name.endswith("/"):
                raise ValueError(f"unsafe ZIP entry: {name}")
            info = zipfile.ZipInfo(name, ZIP_TIMESTAMP)
            info.compress_type = zipfile.ZIP_STORED
            info.create_system = 3
            info.external_attr = 0o100644 << 16
            archive.writestr(info, content)
    return output.getvalue()


def word_document(marker: str) -> bytes:
    entries = {
        "[Content_Types].xml": b"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>
""",
        "_rels/.rels": b"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>
""",
        "word/document.xml": (
            """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Synthetic status report</w:t></w:r></w:p>
    <w:p><w:r><w:t>Fixture identity %s</w:t></w:r></w:p>
    <w:sectPr/>
  </w:body>
</w:document>
"""
            % marker
        ).encode("utf-8"),
    }
    return deterministic_zip(entries)


def spreadsheet_document(marker: str) -> bytes:
    entries = {
        "[Content_Types].xml": b"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
  <Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
</Types>
""",
        "_rels/.rels": b"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>
""",
        "xl/workbook.xml": b"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets><sheet name="Ledger" sheetId="1" r:id="rId1"/></sheets>
</workbook>
""",
        "xl/_rels/workbook.xml.rels": b"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
</Relationships>
""",
        "xl/worksheets/sheet1.xml": (
            """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData>
    <row r="1"><c r="A1" t="inlineStr"><is><t>Item</t></is></c><c r="B1" t="inlineStr"><is><t>Status</t></is></c></row>
    <row r="2"><c r="A2" t="inlineStr"><is><t>%s</t></is></c><c r="B2" t="inlineStr"><is><t>ready</t></is></c></row>
  </sheetData>
</worksheet>
"""
            % marker
        ).encode("utf-8"),
    }
    return deterministic_zip(entries)


def presentation_document(marker: str) -> bytes:
    entries = {
        "[Content_Types].xml": b"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
</Types>
""",
        "_rels/.rels": b"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>
""",
        "ppt/presentation.xml": b"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldIdLst><p:sldId id="256" r:id="rId1"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
</p:presentation>
""",
        "ppt/_rels/presentation.xml.rels": b"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
</Relationships>
""",
        "ppt/slides/slide1.xml": (
            """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
    <p:sp><p:nvSpPr><p:cNvPr id="2" name="Title"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/>
      <p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Synthetic brief %s</a:t></a:r></a:p></p:txBody>
    </p:sp>
  </p:spTree></p:cSld>
</p:sld>
"""
            % marker
        ).encode("utf-8"),
    }
    return deterministic_zip(entries)


def pdf_document(marker: str) -> bytes:
    stream = (
        "BT /F1 12 Tf 72 720 Td (Synthetic status report %s) Tj ET\n" % marker
    ).encode("ascii")
    objects = (
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
        b"/Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        b"<< /Length "
        + str(len(stream)).encode("ascii")
        + b" >>\nstream\n"
        + stream
        + b"endstream",
    )
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
    document.extend(
        (
            f"trailer\n<< /Size {len(objects) + 1} /Root 1 0 R >>\n"
            f"startxref\n{xref_offset}\n%%EOF\n"
        ).encode("ascii")
    )
    return bytes(document)


def png_chunk(chunk_type: bytes, data: bytes) -> bytes:
    checksum = binascii.crc32(chunk_type + data) & 0xFFFFFFFF
    return struct.pack(">I", len(data)) + chunk_type + data + struct.pack(">I", checksum)


def stored_zlib(data: bytes) -> bytes:
    if len(data) > 0xFFFF:
        raise ValueError("PNG fixture scanline exceeds one stored DEFLATE block")
    block = (
        b"\x01"
        + struct.pack("<H", len(data))
        + struct.pack("<H", 0xFFFF - len(data))
        + data
    )
    return b"\x78\x01" + block + struct.pack(">I", zlib.adler32(data) & 0xFFFFFFFF)


def png_document(marker: str) -> bytes:
    digest = bytes.fromhex(marker * 2)
    row_one = b"\x00" + digest[:6]
    row_two = b"\x00" + digest[6:12]
    header = struct.pack(">IIBBBBB", 2, 2, 8, 2, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + png_chunk(b"IHDR", header)
        + png_chunk(b"IDAT", stored_zlib(row_one + row_two))
        + png_chunk(b"IEND", b"")
    )


def archive_document(marker: str) -> bytes:
    return deterministic_zip(
        {
            "README.txt": (
                "Synthetic reference bundle. No executable content.\n"
                f"Fixture identity {marker}.\n"
            ).encode("utf-8"),
            "records/items.json": canonical_json(
                {
                    "fixture": marker,
                    "items": ["alpha", "beta"],
                    "synthetic": True,
                }
            ),
        }
    )


def notebook_document(marker: str) -> bytes:
    return canonical_json(
        {
            "cells": [
                {
                    "cell_type": "markdown",
                    "metadata": {},
                    "source": ["# Synthetic analysis notes\n", f"Fixture `{marker}`.\n"],
                },
                {
                    "cell_type": "raw",
                    "metadata": {},
                    "source": ["No code is executed by this fixture.\n"],
                },
            ],
            "metadata": {
                "agentmage_fixture": True,
                "language_info": {"name": "text"},
            },
            "nbformat": 4,
            "nbformat_minor": 5,
        }
    )


def materialize_specs(profile: dict[str, Any]) -> dict[str, DocumentFile]:
    marker = {
        family: fixture_marker(profile["seed"], family)
        for family, _media_type, _path in EXPECTED_FAMILIES
    }
    content_by_family = {
        "text": (
            "Synthetic planning meeting\n"
            f"Fixture identity: {marker['text']}\n"
            "Decision: retain bounded, local test data.\n"
        ).encode("utf-8"),
        "json": canonical_json(
            {
                "fixture": marker["json"],
                "name": "Project Cedar",
                "status": "synthetic",
                "tasks": ["index", "review"],
            }
        ),
        "word": word_document(marker["word"]),
        "portable-document-format": pdf_document(marker["portable-document-format"]),
        "spreadsheets": spreadsheet_document(marker["spreadsheets"]),
        "presentations": presentation_document(marker["presentations"]),
        "images": png_document(marker["images"]),
        "archives": archive_document(marker["archives"]),
        "notebooks": notebook_document(marker["notebooks"]),
        "logs": (
            "2024-01-01T00:00:00Z INFO synthetic-service started\n"
            f"2024-01-01T00:00:01Z INFO fixture={marker['logs']} status=ready\n"
            "2024-01-01T00:00:02Z WARN synthetic retry count=1\n"
        ).encode("utf-8"),
    }
    files: dict[str, DocumentFile] = {}
    for family, media_type, path in EXPECTED_FAMILIES:
        if path in files or not safe_relative(path):
            raise ValueError(f"invalid document fixture path: {path}")
        files[path] = DocumentFile(family, media_type, content_by_family[family])
    return dict(sorted(files.items()))


def corpus_identity(files: dict[str, DocumentFile]) -> str:
    digest = hashlib.sha256()
    for path, fixture in sorted(files.items()):
        for field in (path.encode(), fixture.family.encode(), fixture.media_type.encode()):
            digest.update(len(field).to_bytes(8, "big"))
            digest.update(field)
        digest.update(len(fixture.content).to_bytes(8, "big"))
        digest.update(fixture.content)
    return digest.hexdigest()


def generate(profile: dict[str, Any], destination: Path) -> dict[str, Any]:
    failures = validate_profile(profile)
    if failures:
        raise ValueError("; ".join(failures))
    if destination.exists():
        raise FileExistsError("document fixture destination already exists")
    destination.parent.mkdir(parents=True, exist_ok=True)
    staging = Path(tempfile.mkdtemp(prefix=".agentmage-documents-", dir=destination.parent))
    files = materialize_specs(profile)
    try:
        for relative, fixture in files.items():
            target = staging / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(fixture.content)
            target.chmod(0o644)
            os.utime(target, (profile["fixed_timestamp_epoch"],) * 2)
        for directory in sorted(
            (path for path in staging.rglob("*") if path.is_dir()), reverse=True
        ):
            os.utime(directory, (profile["fixed_timestamp_epoch"],) * 2)
        os.replace(staging, destination)
    except BaseException:
        shutil.rmtree(staging, ignore_errors=True)
        raise
    return {
        "corpus_sha256": corpus_identity(files),
        "family_count": len(files),
        "file_count": len(files),
    }


def build_report(root: Path = ROOT) -> dict[str, Any]:
    profile_path = root / PROFILE_PATH.relative_to(ROOT)
    profile = read_json(profile_path)
    failures = validate_profile(profile)
    if failures:
        raise ValueError("; ".join(failures))
    files = materialize_specs(profile)
    return {
        "schema_version": 1,
        "task_id": "2.1.1.5",
        "status": "pass",
        "profile": {
            "id": profile["profile_id"],
            "sha256": sha256_bytes(profile_path.read_bytes()),
            "seed_sha256": sha256_bytes(profile["seed"].encode("utf-8")),
        },
        "corpus_preview": {
            "persisted": False,
            "sha256": corpus_identity(files),
            "family_count": len(files),
            "files": [
                {
                    "bytes": len(fixture.content),
                    "family": fixture.family,
                    "media_type": fixture.media_type,
                    "path": path,
                    "sha256": sha256_bytes(fixture.content),
                }
                for path, fixture in sorted(files.items())
            ],
        },
        "side_effect_contract": profile["side_effect_contract"],
        "content_safety_contract": profile["content_safety_contract"],
        "privacy_contract": profile["privacy_contract"],
        "product_parser_support_claim": "none",
        "versioned_corpus_status": "fulfilled-by-agentmage-versioned-synthetic-corpus-v1",
        "macos_support_claim": "none",
    }


def validate_report(report: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(report, dict):
        return ["document fixture report must be an object"]
    failures: list[str] = []
    if report.get("schema_version") != 1 or report.get("task_id") != "2.1.1.5":
        failures.append("document fixture report identity is invalid")
    if report.get("status") != "pass":
        failures.append("document fixture generation did not pass")
    if report.get("product_parser_support_claim") != "none":
        failures.append("document fixture report made a parser support claim")
    if (
        report.get("versioned_corpus_status")
        != "fulfilled-by-agentmage-versioned-synthetic-corpus-v1"
    ):
        failures.append("document fixture report lost its versioned corpus disposition")
    if report.get("macos_support_claim") != "none":
        failures.append("document fixture report made a macOS support claim")
    if report != build_report(root):
        failures.append("document fixture report is stale or non-deterministic")
    return failures


def write_report(root: Path = ROOT) -> None:
    output = root / REPORT_PATH.relative_to(ROOT)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(canonical_json(build_report(root)))


def check_report(root: Path = ROOT) -> list[str]:
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read document fixture report: {error}"]
    return validate_report(report, root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path)
    parser.add_argument("--write-report", action="store_true")
    args = parser.parse_args()
    try:
        profile = read_json(PROFILE_PATH)
        if args.output is not None:
            result = generate(profile, args.output)
            print(json.dumps(result, indent=2, sort_keys=True))
        if args.write_report:
            write_report()
        failures = check_report()
    except (OSError, ValueError) as error:
        print(f"document fixture generator failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"document fixture generator failed: {failure}", file=sys.stderr)
        return 1
    if args.output is None:
        print("deterministic inert document fixtures validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
