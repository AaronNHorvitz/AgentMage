#!/usr/bin/env python3
"""Generate and validate the closed Sprint 60 PDF extraction review records."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
CORPUS: Final = ROOT / "docs/verification/sprint-60-pdf-extraction-corpus.json"
MANIFEST: Final = ROOT / "docs/verification/sprint-60-pdf-dependency-manifest.json"


def expected_cases() -> list[dict[str, str]]:
    groups = {
        "dependency": (
            "S-051-I01",
            [
                "lopdf is pinned to exact version 0.44.0",
                "lopdf default features are disabled",
                "the parser registry checksum is retained",
                "the parser MIT license is retained",
                "all parser transitives remain in the locked software bill of materials",
                "metadata uses the same admitted parser boundary",
                "no PDF renderer is implicitly admitted",
                "no PDF generator is implicitly admitted",
                "no OCR package or model is implicitly admitted",
                "unadmitted components cannot satisfy story completion",
            ],
        ),
        "extraction": (
            "S-051-I02",
            [
                "source bytes are accepted only from caller memory",
                "source bytes bind an exact SHA-256 identity",
                "the parser operates in strict mode",
                "source object page stream and text ceilings are explicit",
                "logical pages are consecutive and one based",
                "each page binds source object number and generation",
                "page identity is deterministic from source page and object",
                "embedded text retains an exact UTF-8 digest",
                "total retained text bytes are recomputable",
                "citations bind exact source page identity",
                "citations bind exact extracted text digest",
                "embedded text confidence is exact",
                "region coordinates remain explicitly unavailable",
                "the original PDF remains authoritative",
                "extraction performs no filesystem network or process effect",
            ],
        ),
        "detection": (
            "S-051-I03",
            [
                "empty input rejects",
                "non-PDF input rejects",
                "truncated PDF syntax fails closed",
                "malformed trailer or cross-reference structure fails closed",
                "oversized source rejects before parsing",
                "object-count overflow rejects",
                "page-count overflow rejects",
                "decompressed stream overflow remains visible",
                "per-page text overflow retains no partial best-effort text",
                "total-text overflow remains visible",
                "image-count overflow remains visible",
                "image inventory failure remains visible",
                "image-only pages become scanned candidates",
                "pages without text or images remain empty",
                "text decoding failure becomes extraction limited",
                "encrypted documents reject without password handling",
                "empty-password encrypted documents still reject",
                "unsupported parser paths never broaden authority",
            ],
        ),
        "ocr_boundary": (
            "S-051-I04",
            [
                "the extractor never launches OCR",
                "OCR requires an exact package name and version",
                "OCR requires exact package and model digests",
                "OCR requires a retained license expression",
                "OCR requires an exact admission receipt digest",
                "OCR admission verification is asserted by a trusted caller",
                "OCR source digest must match extraction source",
                "OCR page number and page identity must both match",
                "OCR is accepted only for a scanned candidate page",
                "OCR confidence remains explicit in basis points",
                "OCR text retains an exact digest",
                "OCR citations retain exact source page identity",
                "OCR output remains labeled probabilistic",
                "OCR validation itself launches no process",
                "no OCR package or model is presently admitted",
            ],
        ),
        "runtime_record": (
            "60.1.2/60.1.3",
            [
                "PDF extraction result schema is closed and versioned",
                "PDF OCR projection schema is closed and versioned",
                "unknown fields reject",
                "page identity drift rejects",
                "text digest drift rejects",
                "citation drift rejects",
                "aggregate byte-count drift rejects",
                "false completion claims reject",
                "false OCR admission claims reject",
                "effect-field broadening rejects",
            ],
        ),
        "external_blocker": (
            "60.1",
            [
                "Sprint 59 remains an upstream blocker",
                "no approved local OCR package or model evidence exists",
                "no OCR dependency failure or cancellation campaign exists",
                "no PDF renderer is admitted",
                "no PDF generation dependency is admitted",
                "no redaction residue report exists",
                "no native Fedora PDF rendering evidence exists",
                "no native Ubuntu PDF rendering evidence exists",
                "no native Windows 11 PDF rendering evidence exists",
                "no retained macOS PDF rendering evidence exists",
                "no installed-product accessibility review exists",
                "no independent native-boundary review exists",
                "manual fuzzing remains explicitly deferred",
                "Sprint 60 remains blocked while required evidence is absent",
            ],
        ),
    }
    cases = []
    for category, (requirement, expectations) in groups.items():
        for index, expected in enumerate(expectations, 1):
            cases.append(
                {
                    "case_id": f"pdf60-{category}-{index:02d}",
                    "category": category,
                    "expected": expected,
                    "requirement": requirement,
                }
            )
    return cases


def expected_corpus() -> dict[str, Any]:
    cases = expected_cases()
    return {
        "schema_version": 1,
        "record_type": "sprint_60_pdf_extraction_corpus",
        "story_id": "60.1",
        "legacy_story_id": "S-051",
        "case_count": len(cases),
        "cases": cases,
        "executable_rust_fixture_count": 6,
        "native_ocr_fixture_count": 0,
        "native_render_fixture_count": 0,
        "network_enabled": False,
        "execution_enabled": False,
        "filesystem_mutation_enabled": False,
        "product_integration_claimed": False,
    }


def expected_manifest() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "sprint_60_pdf_dependency_manifest",
        "story_id": "60.1",
        "admitted_dependencies": [
            {
                "capabilities": ["bounded-extraction", "basic-metadata", "page-identity"],
                "cargo_checksum_sha256": "5e2ec995d822e05cabc3f06d196ee43650af3fe4fe38012cacb35e0c3d113b68",
                "default_features": False,
                "license": "MIT",
                "name": "lopdf",
                "version": "0.44.0",
            }
        ],
        "unadmitted_components": [
            {
                "capability": "pdf-generation",
                "reason_code": "pdf.generator.package-license-version-digest-not-admitted",
                "state": "blocked",
            },
            {
                "capability": "pdf-ocr",
                "reason_code": "pdf.ocr.package-model-license-version-digest-not-admitted",
                "state": "blocked",
            },
            {
                "capability": "pdf-rendering",
                "reason_code": "pdf.renderer.package-license-version-digest-not-admitted",
                "state": "blocked",
            },
        ],
        "required_first_ga_platforms": ["fedora", "ubuntu", "windows11-x64"],
        "retained_post_ga_platforms": ["macos-apple-silicon"],
        "executed_parser_platforms": ["fedora-x86_64"],
        "executed_native_ocr_platforms": [],
        "executed_native_renderer_platforms": [],
        "cross_platform_extraction_acceptance_complete": False,
        "independent_native_review_complete": False,
    }


def validate(corpus: Any, manifest: Any) -> list[str]:
    failures = []
    if corpus != expected_corpus():
        failures.append("Sprint 60 PDF extraction corpus drifted")
    if manifest != expected_manifest():
        failures.append("Sprint 60 PDF dependency manifest drifted")
    encoded = json.dumps([corpus, manifest], sort_keys=True).lower()
    for prohibited in (
        "credential_value",
        "secret_value",
        "private_key",
        "access_token",
        "raw_document",
        "absolute_workspace_path",
        "native_ocr_complete",
        "cross_platform_extraction_passed",
    ):
        if prohibited in encoded:
            failures.append(f"prohibited review-record field: {prohibited}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    arguments = parser.parse_args()
    if arguments.write:
        CORPUS.parent.mkdir(parents=True, exist_ok=True)
        CORPUS.write_text(json.dumps(expected_corpus(), indent=2) + "\n", encoding="utf-8")
        MANIFEST.write_text(json.dumps(expected_manifest(), indent=2) + "\n", encoding="utf-8")
    if not CORPUS.is_file() or not MANIFEST.is_file():
        print("error: Sprint 60 PDF review records are missing")
        return 1
    failures = validate(
        json.loads(CORPUS.read_text(encoding="utf-8")),
        json.loads(MANIFEST.read_text(encoding="utf-8")),
    )
    if failures:
        for failure in failures:
            print(f"error: {failure}")
        return 1
    print(
        "Sprint 60 PDF extraction records validated: "
        f"{expected_corpus()['case_count']} cases, 1 admitted parser, 3 blocked capability classes"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
