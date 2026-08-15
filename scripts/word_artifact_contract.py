#!/usr/bin/env python3
"""Generate and validate the closed Sprint 58 Word artifact review records."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
CORPUS: Final = ROOT / "docs/verification/sprint-58-word-artifact-corpus.json"
DEPENDENCIES: Final = ROOT / "docs/verification/sprint-58-word-dependency-manifest.json"


def case(identifier: str, category: str, expected: str, requirement: str) -> dict[str, str]:
    return {
        "case_id": identifier,
        "category": category,
        "expected": expected,
        "requirement": requirement,
    }


def expected_cases() -> list[dict[str, str]]:
    groups = {
        "dependency": (
            "S-050-I01",
            [
                "quick-xml is exactly pinned and hash bound",
                "zip is exactly pinned and hash bound",
                "both admitted crates have MIT license evidence",
                "default dependency features remain disabled",
                "only the bounded ZIP deflate feature is enabled",
                "Cargo.lock seals the complete transitive graph",
                "the production dependency class is explicit",
                "the supply-chain catalog and software bill of materials agree",
                "Fedora local evidence does not imply macOS or Ubuntu execution",
                "the unadmitted renderer remains a named blocker",
            ],
        ),
        "extraction": (
            "S-050-I02",
            [
                "identical source profile and converter produce identical sidecar bytes",
                "source digest binds every extraction",
                "converter identity binds every extraction",
                "profile identity contributes to the cache key",
                "changed source bytes miss the cache",
                "bounded cache reuses only exact sealed results",
                "UTF-8 text carries exact package-part byte provenance",
                "current text retains current revision state",
                "inserted text retains inserted revision state",
                "deleted text retains deleted revision state",
                "sidecar digest matches exact proposed bytes",
                "extraction performs no persistent cache write",
            ],
        ),
        "fidelity": (
            "S-050-I03",
            [
                "table presence produces an explicit fidelity warning",
                "comment presence produces an explicit fidelity warning",
                "tracked insertion produces an explicit fidelity warning",
                "tracked deletion produces an explicit fidelity warning",
                "header presence produces an explicit fidelity warning",
                "footer presence produces an explicit fidelity warning",
                "numbering presence produces an explicit fidelity warning",
                "section layout produces an explicit fidelity warning",
                "hyperlink presence produces an explicit fidelity warning",
                "image presence produces an explicit fidelity warning",
                "field presence produces an explicit fidelity warning",
                "style presence produces an explicit fidelity warning",
                "the immutable original remains authoritative for every warning",
            ],
        ),
        "inspection": (
            "S-050-I04",
            [
                "required OOXML parts are inventoried in canonical order",
                "every safely read part receives an exact content digest",
                "missing required parts quarantine extraction",
                "duplicate central-directory names quarantine extraction",
                "unsafe entry paths quarantine extraction",
                "encrypted entries quarantine extraction",
                "symbolic-link entries quarantine extraction",
                "unsupported compression quarantines extraction",
                "macro and active-content parts quarantine extraction",
                "external relationships quarantine without resolution",
                "malformed relationship XML quarantines extraction",
                "source byte limit fails closed",
                "entry count limit counts raw central-directory records",
                "entry and aggregate expansion limits fail closed",
                "inspection performs no filesystem network or execution effect",
            ],
        ),
        "generation": (
            "S-050-I04",
            [
                "identical Markdown and output identity produce identical DOCX bytes",
                "headings become semantic Word heading styles",
                "ordered and unordered lists use numbering definitions",
                "Markdown tables become semantic Word tables",
                "code lines remain visible text",
                "raw HTML remains escaped visible text",
                "external Markdown links create no external relationship",
                "generated packages reopen through the same bounded inspector",
                "source-path overwrite identity is rejected",
                "generation returns an unpersisted proposal only",
            ],
        ),
        "record_contract": (
            "S-050-I02/S-050-I04",
            [
                "inspection report schema is closed and versioned",
                "extraction result schema binds sidecar digest and inspection identities",
                "generated package schema binds package digest and reopened inspection",
                "noncanonical part feature finding fragment and warning ledgers reject",
                "any hidden filesystem network or execution effect rejects",
            ],
        ),
        "prohibited_effect": (
            "S-050-ST01",
            [
                "deny source overwrite",
                "deny sidecar persistence",
                "deny generated package persistence",
                "deny external relationship resolution",
                "deny active-content execution",
                "deny field evaluation",
                "deny renderer completion claims",
                "deny cross-platform completion claims from Fedora-only evidence",
            ],
        ),
    }
    cases: list[dict[str, str]] = []
    for category, (requirement, expectations) in groups.items():
        for index, expected in enumerate(expectations, 1):
            cases.append(case(f"word-{category}-{index:02d}", category, expected, requirement))
    return cases


def expected_corpus() -> dict[str, Any]:
    cases = expected_cases()
    return {
        "schema_version": 1,
        "record_type": "sprint_58_word_artifact_acceptance_corpus",
        "story_id": "58.1",
        "legacy_story_id": "S-050",
        "case_count": len(cases),
        "cases": cases,
        "network_enabled": False,
        "execution_enabled": False,
        "filesystem_mutation_enabled": False,
        "renderer_admitted": False,
        "product_integration_claimed": False,
    }


def expected_dependencies() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "sprint_58_word_dependency_manifest",
        "story_id": "58.1",
        "isolation": {
            "resolver": "cargo-lock-v4",
            "locked_build_required": True,
            "default_features_disabled": True,
            "ambient_process_execution": False,
            "ambient_filesystem_access": False,
            "ambient_network_access": False,
        },
        "admitted_components": [
            {
                "name": "quick-xml",
                "version": "0.41.0",
                "cargo_checksum_sha256": "e660451e55124f798a69a5af3f49ccfbefbd41910eefd25caf2393e1f3473ec1",
                "license": "MIT",
                "enabled_features": [],
                "purposes": ["bounded-ooxml-xml-inspection", "bounded-ooxml-text-extraction"],
            },
            {
                "name": "zip",
                "version": "8.6.0",
                "cargo_checksum_sha256": "2d04a6b5381502aa6087c94c669499eb1602eb9c5e8198e534de571f7154809b",
                "license": "MIT",
                "enabled_features": ["deflate-flate2-zlib-rs"],
                "purposes": ["bounded-ooxml-package-generation", "bounded-ooxml-package-inspection"],
            },
        ],
        "source_compatibility_targets": ["fedora", "macos", "ubuntu"],
        "executed_platform_evidence": ["fedora-x86_64"],
        "unadmitted_components": [
            {
                "capability": "word-renderer",
                "state": "deferred-to-sprint-59",
                "reason_code": "word.renderer.cross-platform-package-not-admitted",
            }
        ],
        "cross_platform_acceptance_complete": False,
    }


def validate(corpus: Any, dependencies: Any) -> list[str]:
    failures = []
    if corpus != expected_corpus():
        failures.append("Word artifact corpus drifted from the closed inventory")
    if dependencies != expected_dependencies():
        failures.append("Word dependency manifest drifted from the admitted inventory")
    encoded = json.dumps([corpus, dependencies], sort_keys=True).lower()
    for prohibited in (
        "credential_value", "secret_value", "private_key", "access_token", "raw_document",
        "absolute_workspace_path", "renderer_execution_complete",
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
        DEPENDENCIES.write_text(
            json.dumps(expected_dependencies(), indent=2) + "\n", encoding="utf-8"
        )
    if not CORPUS.is_file() or not DEPENDENCIES.is_file():
        print("error: Sprint 58 review records are missing")
        return 1
    failures = validate(
        json.loads(CORPUS.read_text(encoding="utf-8")),
        json.loads(DEPENDENCIES.read_text(encoding="utf-8")),
    )
    if failures:
        for failure in failures:
            print(f"error: {failure}")
        return 1
    print(f"validated {len(expected_cases())} Sprint 58 Word artifact cases")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
