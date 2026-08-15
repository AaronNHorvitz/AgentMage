#!/usr/bin/env python3
"""Generate and validate the closed Sprint 62 tabular review records."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
CORPUS: Final = ROOT / "docs/verification/sprint-62-tabular-corpus.json"
MANIFEST: Final = ROOT / "docs/verification/sprint-62-tabular-dependency-manifest.json"


def groups() -> dict[str, tuple[str, list[str]]]:
    return {
        "delimited_input": (
            "62.1.1.3/62.1.1.4",
            [
                "CSV TSV and semicolon dialects are explicit",
                "quoted fields retain delimiters",
                "quoted fields retain embedded CRLF data",
                "doubled quotes decode exactly",
                "invalid UTF-8 rejects",
                "unterminated quoted fields reject",
                "characters after a closing quote reject",
                "empty input rejects",
                "duplicate normalized headers reject",
                "empty normalized headers reject",
                "ragged records reject",
                "source bytes rows columns and fields are bounded",
                "source path and source digest remain exact",
                "original source remains authoritative",
                "parsing performs no file network or execution effect",
                "missing values use cleaned whitespace semantics",
                "duplicate rows use exact canonical row digests",
                "filters bind admitted normalized columns",
                "sorts use stable normalized lexical order",
                "source row projections retain exact row digests",
                "matching retains exact left and right source digests",
                "duplicate and missing keys have closed reason codes",
                "differing fields are canonically ordered",
                "normalization uses NFKC lowercase and collapsed whitespace",
                "filename helpers cannot introduce path separators",
                "URL normalization never resolves or fetches a target",
                "CSV output quotes delimiter quote and line-break values",
                "formula DDE and leading-control cells receive an inert prefix",
                "safe CSV bytes reopen as inert text",
                "safe CSV proposals perform no file network or execution effect",
            ],
        ),
        "open_xml": (
            "62.1.1.1/62.1.1.2",
            [
                "only caller-supplied XLSX bytes and a canonical path are accepted",
                "ZIP entry paths are enclosed and traversal-free",
                "duplicate package entries reject",
                "encrypted entries reject without password handling",
                "only stored and deflated entries are admitted",
                "compressed source entry count entry bytes and total bytes are bounded",
                "workbook relationships resolve only inside the package",
                "external worksheet relationships reject",
                "worksheet count cells and shared strings are bounded",
                "workbook 1900 and 1904 date systems are explicit",
                "Excel serial 60 retains the documented 1900 leap-day convention",
                "worksheet identity name state part and digest are retained",
                "visible hidden and very-hidden worksheets remain distinct",
                "cell address row column type raw value and displayed value are retained",
                "shared and inline strings are decoded without execution",
                "formulas remain inert with cached values separated",
                "formula cells never recalculate",
                "styles and number formats are retained",
                "admitted date styles produce bounded ISO dates",
                "hidden row and column ledgers are canonical",
                "merged ranges are canonical and deduplicated",
                "hyperlinks retain exact target digests and followed false",
                "external hyperlinks remain inert and nonblocking",
                "external workbook formulas block analytical reuse",
                "DDE-like formulas block analytical reuse",
                "VBA and macro-enabled content blocks analytical reuse",
                "malformed XML or missing required parts rejects",
                "cells findings and links are canonically ordered",
                "safe analysis is recomputed from blocking findings",
                "inspection performs no file network or execution effect",
            ],
        ),
        "runtime_records": (
            "62.1.2/62.1.3",
            [
                "tabular document schema is closed and versioned",
                "tabular comparison schema is closed and versioned",
                "safe CSV proposal schema is closed and versioned",
                "spreadsheet inspection schema is closed and versioned",
                "normalized header drift rejects",
                "row-shape drift rejects",
                "comparison reason and overlap drift reject",
                "comparison row and field order drift rejects",
                "safe CSV byte digest drift rejects",
                "cell address coordinate drift rejects",
                "hyperlink digest drift rejects",
                "safe-analysis and effect drift reject",
            ],
        ),
        "external_blocker": (
            "62.1",
            [
                "Sprint 61 remains an upstream blocker",
                "legacy binary XLS parsing is not admitted",
                "encrypted workbook decryption is not admitted",
                "no native office-suite renderer is admitted",
                "no Fedora native office reopen evidence exists",
                "no Ubuntu native office reopen evidence exists",
                "no Windows native office reopen evidence exists",
                "no retained Apple Silicon macOS reopen evidence exists",
                "no installed-product accessibility evidence exists",
                "no independent spreadsheet-boundary review exists",
                "no independent visual review exists",
                "manual fuzzing remains explicitly deferred",
                "Sprint 62 remains blocked while required evidence is absent",
            ],
        ),
    }


def expected_cases() -> list[dict[str, str]]:
    return [
        {
            "case_id": f"data62-{category}-{index:02d}",
            "category": category,
            "expected": expected,
            "requirement": requirement,
        }
        for category, (requirement, expectations) in groups().items()
        for index, expected in enumerate(expectations, 1)
    ]


def expected_corpus() -> dict[str, Any]:
    cases = expected_cases()
    return {
        "schema_version": 1,
        "record_type": "sprint_62_tabular_corpus",
        "story_id": "62.1",
        "legacy_story_id": "S-052",
        "case_count": len(cases),
        "cases": cases,
        "executable_tabular_rust_fixture_count": 5,
        "executable_spreadsheet_rust_fixture_count": 4,
        "runtime_schema_count": 4,
        "native_office_fixture_count": 0,
        "network_enabled": False,
        "filesystem_mutation_enabled": False,
        "formula_execution_enabled": False,
        "release_completion_claimed": False,
    }


def expected_manifest() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "sprint_62_tabular_dependency_manifest",
        "story_id": "62.1",
        "admitted_production_dependencies": [
            {"name": "quick-xml", "version": "0.41.0", "license": "MIT", "cargo_checksum_sha256": "e660451e55124f798a69a5af3f49ccfbefbd41910eefd25caf2393e1f3473ec1", "capability": "bounded-open-xml-parsing"},
            {"name": "unicode-normalization", "version": "0.1.25", "license": "MIT OR Apache-2.0", "cargo_checksum_sha256": "5fd4f6878c9cb28d874b009da9e8d183b5abc80117c40bbd187a1fde336be6e8", "capability": "stable-tabular-normalization"},
            {"name": "zip", "version": "8.6.0", "license": "MIT", "cargo_checksum_sha256": "2d04a6b5381502aa6087c94c669499eb1602eb9c5e8198e534de571f7154809b", "capability": "bounded-xlsx-container-reading"},
        ],
        "unadmitted_product_components": [
            {"capability": "legacy-xls-parsing", "reason_code": "spreadsheet.xls.not-admitted", "state": "blocked"},
            {"capability": "encrypted-workbook-decryption", "reason_code": "spreadsheet.encryption.not-admitted", "state": "blocked"},
            {"capability": "native-office-rendering", "reason_code": "spreadsheet.renderer.not-admitted", "state": "blocked"},
        ],
        "required_first_ga_platforms": ["fedora", "ubuntu", "windows11-x64"],
        "retained_post_ga_platforms": ["macos-apple-silicon"],
        "executed_core_platforms": ["fedora-x86_64"],
        "executed_native_office_platforms": [],
        "cross_platform_acceptance_complete": False,
        "independent_review_complete": False,
    }


def validate(corpus: Any, manifest: Any) -> list[str]:
    failures = []
    if corpus != expected_corpus():
        failures.append("Sprint 62 tabular corpus drifted")
    if manifest != expected_manifest():
        failures.append("Sprint 62 dependency manifest drifted")
    encoded = json.dumps([corpus, manifest], sort_keys=True).lower()
    for prohibited in ("credential_value", "secret_value", "private_key", "access_token", "raw_workbook", "absolute_workspace_path", "native_office_complete", "release_approved"):
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
        print("error: Sprint 62 tabular review records are missing")
        return 1
    failures = validate(json.loads(CORPUS.read_text()), json.loads(MANIFEST.read_text()))
    if failures:
        for failure in failures:
            print(f"error: {failure}")
        return 1
    print(f"Sprint 62 tabular records validated: {expected_corpus()['case_count']} cases")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
