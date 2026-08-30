#!/usr/bin/env python3
"""Build and verify inert hostile-ingestion fixtures for Story 2.4.1.2."""

from __future__ import annotations

import argparse
import copy
import io
import json
import os
import sys
import tempfile
import zipfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.engineering_artifact_admission_fixtures import (  # noqa: E402
    FixtureSpec,
    build_archive,
    canonical_json,
    case_record,
    safe_archive_entry,
    sha256_bytes,
    validate_archive,
    write_atomic,
)


CORPUS_DIR: Final = ROOT / "fixtures" / "artifact-admission" / "v1"
ARCHIVE_PATH: Final = CORPUS_DIR / "artifact-adversarial-corpus-v1.zip"
MANIFEST_PATH: Final = CORPUS_DIR / "adversarial-manifest.json"
REQUIRED_CATEGORIES: Final = (
    "malformed_package",
    "active_content",
    "external_relationship",
    "archive_traversal",
    "decompression_bomb",
    "mixed_encoding",
    "concurrent_replacement",
    "truncation",
    "cancellation",
    "parser_failure",
)
EXPECTED_TERMINALS: Final = {
    "malformed_package": ("malformed_container", "deny_before_extraction"),
    "active_content": ("active_content_present", "deny_without_execution"),
    "external_relationship": ("external_relationship_present", "deny_without_fetch"),
    "archive_traversal": ("unsafe_archive_path", "deny_before_materialization"),
    "decompression_bomb": ("decompression_limit_exceeded", "deny_and_cleanup"),
    "mixed_encoding": ("mixed_encoding_unsupported", "fail_with_diagnostic"),
    "concurrent_replacement": ("source_replaced", "fail_stale_without_capture"),
    "truncation": ("source_truncated", "partial_with_visible_warning"),
    "cancellation": ("ingestion_cancelled", "cancel_without_partial_publication"),
    "parser_failure": ("parser_failed", "fail_without_derivative"),
}
ZIP_TIME: Final = (2024, 1, 1, 0, 0, 0)


@dataclass(frozen=True)
class AdversarialSpec:
    source: FixtureSpec
    resource_limit: dict[str, int] | None = None


def nested_zip(entries: list[tuple[str, bytes]], *, compressed: bool = False) -> bytes:
    output = io.BytesIO()
    method = zipfile.ZIP_DEFLATED if compressed else zipfile.ZIP_STORED
    with zipfile.ZipFile(output, "w", compression=method, compresslevel=9 if compressed else None) as archive:
        for path, content in entries:
            info = zipfile.ZipInfo(path, ZIP_TIME)
            info.compress_type = method
            info.create_system = 3
            info.external_attr = 0o100644 << 16
            archive.writestr(info, content)
    return output.getvalue()


def active_package() -> bytes:
    return nested_zip(
        [
            (
                "[Content_Types].xml",
                b'<Types><Override PartName="/word/vbaProject.bin" ContentType="application/vnd.ms-office.vbaProject"/></Types>',
            ),
            ("word/document.xml", b"<document><text>Synthetic active-content fixture</text></document>"),
            ("word/vbaProject.bin", b"AM_SYNTHETIC_INERT_VBA_CANARY_DO_NOT_EXECUTE"),
        ]
    )


def external_relationship_package() -> bytes:
    return nested_zip(
        [
            ("word/document.xml", b"<document><text>Synthetic external relationship</text></document>"),
            (
                "word/_rels/document.xml.rels",
                b'<Relationships><Relationship Id="rId1" Target="https://invalid.example/fixture" TargetMode="External"/></Relationships>',
            ),
        ]
    )


def materialize_specs() -> list[AdversarialSpec]:
    malformed = b"PK\x03\x04\x14\x00synthetic-truncated-container"
    traversal = nested_zip(
        [
            ("../outside.txt", b"Synthetic traversal payload. Must never materialize.\n"),
            ("safe/readme.txt", b"Synthetic safe sibling.\n"),
        ]
    )
    bomb = nested_zip(
        [("expanded/repeated.txt", b"A" * (2 * 1024 * 1024))],
        compressed=True,
    )
    mixed = b"\xef\xbb\xbfUTF-8 prefix\n\xff\xfeU\x00T\x00F\x00-\x001\x006\x00\n\x80invalid-tail"
    original = b"Synthetic concurrently observed revision.\n"
    current = b"Synthetic concurrently replaced revision.\n"
    truncation = (b"0123456789abcdef" * 128) + b"\n"
    cancellation = (b"synthetic-cancellable-stream\n" * 64)
    parser_failure = b"%PDF-1.4\n1 0 obj << /Type /Catalog /Pages 999 0 R >>\nendobj\n%%EOF\n"

    def source(
        category: str,
        entry: str,
        content: bytes,
        media_type: str,
        *,
        reference_class: str = "file_path",
        origin_class: str = "file",
        artifact_kind: str = "hostile_input",
        freshness_state: str = "fresh",
        capture_state: str = "captured",
        current_entry: str | None = None,
        current_content: bytes | None = None,
    ) -> FixtureSpec:
        return FixtureSpec(
            fixture_id=f"adversarial-{category}-1",
            category=category,
            reference_class=reference_class,
            origin_class=origin_class,
            artifact_kind=artifact_kind,
            media_type=media_type,
            archive_entry=entry,
            content=content,
            freshness_state=freshness_state,
            capture_state=capture_state,
            current_entry=current_entry,
            current_content=current_content,
            relation=(
                {
                    "kind": "replaced_during_capture",
                    "target_fixture_id": "adversarial-concurrent-replacement-1-current",
                }
                if category == "concurrent_replacement"
                else None
            ),
        )

    return [
        AdversarialSpec(source("malformed_package", "hostile/malformed/truncated.zip", malformed, "application/zip", reference_class="archive", origin_class="archive", artifact_kind="archive")),
        AdversarialSpec(source("active_content", "hostile/active/macro-container.docm", active_package(), "application/vnd.ms-word.document.macroEnabled.12", artifact_kind="docx_active")),
        AdversarialSpec(source("external_relationship", "hostile/relationships/external.docx", external_relationship_package(), "application/vnd.openxmlformats-officedocument.wordprocessingml.document", artifact_kind="docx_external_relationship")),
        AdversarialSpec(source("archive_traversal", "hostile/archive/traversal.zip", traversal, "application/zip", reference_class="archive", origin_class="archive", artifact_kind="archive")),
        AdversarialSpec(source("decompression_bomb", "hostile/archive/bomb.zip", bomb, "application/zip", reference_class="archive", origin_class="archive", artifact_kind="archive"), {"maximum_decompressed_bytes": 65536, "maximum_ratio": 20}),
        AdversarialSpec(source("mixed_encoding", "hostile/text/mixed-encoding.txt", mixed, "text/plain", artifact_kind="text")),
        AdversarialSpec(source("concurrent_replacement", "hostile/race/observed.txt", original, "text/plain", artifact_kind="text", freshness_state="stale", capture_state="failed", current_entry="hostile/race/current.txt", current_content=current)),
        AdversarialSpec(source("truncation", "hostile/limits/truncation.txt", truncation, "text/plain", artifact_kind="text"), {"maximum_included_bytes": 128}),
        AdversarialSpec(source("cancellation", "hostile/lifecycle/cancellable.txt", cancellation, "text/plain", artifact_kind="stream", capture_state="failed"), {"cancel_after_bytes": 128}),
        AdversarialSpec(source("parser_failure", "hostile/parser/broken.pdf", parser_failure, "application/pdf", artifact_kind="pdf")),
    ]


def archive_entries(specs: list[AdversarialSpec]) -> dict[str, bytes]:
    entries: dict[str, bytes] = {}
    for case in specs:
        spec = case.source
        if not safe_archive_entry(spec.archive_entry or "") or spec.content is None:
            raise ValueError(f"adversarial fixture lacks inert payload: {spec.fixture_id}")
        if spec.archive_entry in entries:
            raise ValueError(f"duplicate adversarial archive entry: {spec.archive_entry}")
        entries[spec.archive_entry] = spec.content
        if spec.current_entry is not None and spec.current_content is not None:
            if not safe_archive_entry(spec.current_entry) or spec.current_entry in entries:
                raise ValueError(f"invalid adversarial current entry: {spec.current_entry}")
            entries[spec.current_entry] = spec.current_content
    return dict(sorted(entries.items()))


def case_manifest(case: AdversarialSpec) -> dict[str, Any]:
    source = case_record(case.source)
    error_code, terminal_state = EXPECTED_TERMINALS[case.source.category]
    source.update(
        {
            "expected_result": {
                "terminal": True,
                "terminal_state": terminal_state,
                "error_code": error_code,
                "may_claim_complete": False,
                "may_publish_partial_derivative": case.source.category == "truncation",
                "active_content_executed": False,
                "external_relationship_fetched": False,
                "path_materialized": False,
                "residue_retained": False,
            },
            "resource_limit": case.resource_limit,
        }
    )
    return source


def build_manifest(specs: list[AdversarialSpec], archive: bytes, entries: dict[str, bytes]) -> dict[str, Any]:
    value = {
        "schema_version": 1,
        "corpus_id": "engineering-artifact-adversarial-v1",
        "task_id": "2.4.1.2",
        "generated_on": "2026-08-29",
        "status": "inert-hostile-ingestion-fixture-corpus",
        "synthetic_only": True,
        "network_access": False,
        "active_content_execution": False,
        "product_security_claim": "none",
        "archive": {
            "path": str(ARCHIVE_PATH.relative_to(ROOT)),
            "format": "zip-stored-outer-container",
            "entry_count": len(entries),
            "byte_length": len(archive),
            "sha256": sha256_bytes(archive),
        },
        "generator": {
            "path": "scripts/engineering_artifact_adversarial_fixtures.py",
            "sha256": sha256_bytes(Path(__file__).read_bytes()),
        },
        "admission_generator": {
            "path": "scripts/engineering_artifact_admission_fixtures.py",
            "sha256": sha256_bytes((ROOT / "scripts/engineering_artifact_admission_fixtures.py").read_bytes()),
        },
        "required_categories": list(REQUIRED_CATEGORIES),
        "cases": [case_manifest(spec) for spec in specs],
    }
    value["manifest_sha256"] = sha256_bytes(canonical_json(value))
    return value


def expected() -> tuple[bytes, dict[str, Any], dict[str, bytes]]:
    specs = materialize_specs()
    entries = archive_entries(specs)
    archive = build_archive(entries)
    return archive, build_manifest(specs, archive, entries), entries


def validate_manifest(value: Any, archive: bytes, entries: dict[str, bytes]) -> list[str]:
    if not isinstance(value, dict):
        return ["artifact adversarial manifest must be an object"]
    failures: list[str] = []
    unhashed = copy.deepcopy(value)
    recorded = unhashed.pop("manifest_sha256", None)
    if recorded != sha256_bytes(canonical_json(unhashed)):
        failures.append("artifact adversarial manifest self-hash is invalid")
    if value != build_manifest(materialize_specs(), archive, entries):
        failures.append("artifact adversarial manifest is stale, incomplete, reordered, or widened")
    if value.get("required_categories") != list(REQUIRED_CATEGORIES):
        failures.append("artifact adversarial category closure drifted")
    cases = value.get("cases", [])
    if not isinstance(cases, list) or [item.get("category") for item in cases] != list(REQUIRED_CATEGORIES):
        failures.append("artifact adversarial cases do not cover each required category exactly once")
    for case in cases if isinstance(cases, list) else []:
        result = case.get("expected_result", {})
        if result.get("terminal") is not True or result.get("may_claim_complete") is not False:
            failures.append(f"adversarial case permits false completion: {case.get('fixture_id')}")
        if any(result.get(field) is not False for field in ("active_content_executed", "external_relationship_fetched", "path_materialized", "residue_retained")):
            failures.append(f"adversarial case permits prohibited effect: {case.get('fixture_id')}")
    if value.get("synthetic_only") is not True or value.get("network_access") is not False:
        failures.append("artifact adversarial corpus may use private or network data")
    if value.get("product_security_claim") != "none" or value.get("active_content_execution") is not False:
        failures.append("artifact adversarial corpus makes an unsupported security claim")
    if value.get("archive", {}).get("sha256") != sha256_bytes(archive):
        failures.append("artifact adversarial archive identity is invalid")
    return failures


def check() -> list[str]:
    try:
        expected_archive, expected_manifest, entries = expected()
        actual_archive = ARCHIVE_PATH.read_bytes()
        actual_manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    except (OSError, ValueError, json.JSONDecodeError) as error:
        return [f"cannot validate artifact adversarial corpus: {error}"]
    failures: list[str] = []
    if actual_archive != expected_archive:
        failures.append("checked artifact adversarial archive is stale or corrupt")
    failures.extend(validate_archive(actual_archive, entries))
    failures.extend(validate_manifest(actual_manifest, actual_archive, entries))
    if actual_manifest != expected_manifest:
        failures.append("checked artifact adversarial manifest is stale")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    if args.write:
        archive, manifest, _entries = expected()
        write_atomic(ARCHIVE_PATH, archive)
        write_atomic(MANIFEST_PATH, canonical_json(manifest))
    failures = check()
    if failures:
        for failure in failures:
            print(f"Artifact adversarial fixture validation failed: {failure}", file=sys.stderr)
        return 1
    print(f"Validated {len(REQUIRED_CATEGORIES)} inert hostile artifact fixtures")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
