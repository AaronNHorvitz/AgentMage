#!/usr/bin/env python3
"""Generate and validate the closed Sprint 59 Word review records."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
CORPUS: Final = ROOT / "docs/verification/sprint-59-word-review-corpus.json"
MANIFEST: Final = ROOT / "docs/verification/sprint-59-word-renderer-manifest.json"


def case(identifier: str, category: str, expected: str, requirement: str) -> dict[str, str]:
    return {
        "case_id": identifier,
        "category": category,
        "expected": expected,
        "requirement": requirement,
    }


def expected_cases() -> list[dict[str, str]]:
    groups = {
        "rich_generation": (
            "S-050-I05",
            [
                "style identities are closed unique and deterministic",
                "built-in styles cannot be replaced by caller input",
                "font family size bold and shading serialize semantically",
                "portrait and landscape page dimensions remain explicit",
                "page margins and columns remain bounded",
                "header text is emitted through an internal relationship",
                "footer text is emitted through an internal relationship",
                "core title subject creator and description remain visible metadata",
                "custom metadata is rendered into the declared metadata table",
                "warnings remain visible document text",
                "Markdown tables become semantic Word tables",
                "decision cards retain decision status rationale and owner fields",
                "external hyperlink targets reject",
                "internal hyperlink targets resolve to deterministic bookmarks",
                "unresolved internal anchors reject before package generation",
                "table width remains within declared page bounds",
                "table borders support none single double and dotted styles",
                "single-cell border coordinates remain bounded",
                "table and cell shading use exact RGB values",
                "cell margins remain bounded",
                "numbering definitions remain unique and canonical",
                "numbering can be applied only to an existing paragraph",
                "paragraph cloning preserves semantic properties",
                "paragraph removal cannot silently target a missing block",
                "section replacement uses exact existing start and end identities",
                "identical specifications produce identical package bytes",
            ],
        ),
        "edit": (
            "S-050-I06",
            [
                "source package digest must match immutable bytes",
                "source and output workspace identities must differ",
                "edit operation identities remain unique and canonical",
                "exact fragment identity range and text must all match",
                "plain replacement changes only the selected text range",
                "replacement text is XML escaped and remains inert",
                "redline replacement retains simple-run properties",
                "redline deletion and insertion identities are explicit",
                "revision identities cannot be reused",
                "comment identities cannot be reused",
                "comments attach only to supported main-document simple runs",
                "comment author initials and body are encoded inertly",
                "comment content type and relationship are explicit",
                "comment relationship identity collisions reject",
                "overlapping edit ranges reject",
                "unsupported complex runs reject",
                "untouched package-part bytes retain exact digests",
                "changed and preserved part ledgers remain canonical",
                "edited package reopens through bounded inspection",
                "preview verification recomputes exact package bytes",
            ],
        ),
        "visual": (
            "S-050-I07",
            [
                "render profile binds renderer version binary and font manifest",
                "render profile thresholds use deterministic integers",
                "decoded pages require exact consecutive one-based identities",
                "decoded page byte count must equal width times height times four",
                "decoded page digest must match exact RGBA bytes",
                "page count and dimensions remain bounded",
                "total comparison memory remains bounded",
                "before and after platforms must match",
                "before and after profile identities must match",
                "before and after evidence classes must match",
                "identical pixels produce an empty difference rectangle",
                "changed pixels use exact integer parts-per-million ratios",
                "maximum channel delta is exact",
                "changed-pixel bounds are tight and deterministic",
                "dimension changes fail page comparison",
                "pagination changes fail the zero-delta profile",
                "clipping overlap and font fallback observations are explicit",
                "table and image regression observations are explicit",
                "synthetic fixtures always require human review",
                "comparison performs no rendering filesystem network or execution effect",
            ],
        ),
        "receipt": (
            "S-050-I08",
            [
                "receipt binds exact input paths and digests",
                "receipt binds generator converter and editor identities",
                "receipt binds exact changed part digests",
                "structural semantic visual accessibility malware and canary checks are closed",
                "passed and failed checks require retained evidence digests",
                "unavailable checks cannot claim evidence",
                "required platform inventory is retained in the receipt",
                "render report references retain exact report digests",
                "synthetic render evidence blocks completion",
                "missing required platform evidence blocks completion",
                "failed checks take precedence over blocked state",
                "failed render thresholds take precedence over blocked state",
                "blocking fidelity limits prevent local verification",
                "receipt disposition is deterministically recomputable",
                "receipt verification rejects any field mutation",
                "receipt creation performs no filesystem network or execution effect",
            ],
        ),
        "hostile_fixture": (
            "S-050-ST01",
            [
                "macro and active-content package parts quarantine",
                "encrypted package entries quarantine without decryption",
                "malformed ZIP containers fail closed",
                "deflate expansion bombs fail before extraction",
                "parent-traversal package paths quarantine",
                "duplicate central-directory names quarantine",
                "symbolic-link entries quarantine",
                "unadmitted compression methods quarantine",
                "external template relationships quarantine without resolution",
                "active HTTP HTTPS file and FTP relationships quarantine",
                "hidden and web-hidden WordprocessingML quarantine",
                "malformed relationship XML quarantines",
                "malformed document XML quarantines",
                "parser-crash bytes return a bounded error",
                "all hostile paths retain zero network and execution effects",
            ],
        ),
        "runtime_record": (
            "S-050-UT01/S-050-UT02",
            [
                "rich package proposal schema is closed and versioned",
                "edit preview schema is closed and versioned",
                "visual comparison schema is closed and versioned",
                "artifact receipt schema is closed and versioned",
                "package byte arrays bind exact SHA-256 identities",
                "reopened inspection binds proposed output identity",
                "all ledgers reject noncanonical order",
                "visual unchanged-page contradictions reject",
                "receipt false-completion claims reject",
                "unknown fields and hidden authority claims reject",
            ],
        ),
        "external_blocker": (
            "S-050-IT01",
            [
                "no renderer is admitted without exact package license version and digest",
                "Fedora comparator tests are not Fedora renderer evidence",
                "Fedora evidence cannot substitute for Ubuntu rendering",
                "Linux evidence cannot substitute for Windows 11 rendering",
                "first-GA evidence cannot substitute for retained macOS evidence",
                "synthetic page fixtures cannot satisfy native rendering",
                "visual machine checks cannot substitute for required human review",
                "local unit tests cannot satisfy installed-product accessibility review",
                "manual fuzzing remains explicitly deferred",
                "Sprint 59 remains blocked while required external evidence is absent",
            ],
        ),
    }
    cases: list[dict[str, str]] = []
    for category, (requirement, expectations) in groups.items():
        for index, expected in enumerate(expectations, 1):
            cases.append(case(f"word59-{category}-{index:02d}", category, expected, requirement))
    return cases


def expected_corpus() -> dict[str, Any]:
    cases = expected_cases()
    return {
        "schema_version": 1,
        "record_type": "sprint_59_word_review_corpus",
        "story_id": "59.1",
        "legacy_story_id": "S-050",
        "case_count": len(cases),
        "cases": cases,
        "synthetic_visual_fixture_count": 2,
        "native_render_fixture_count": 0,
        "network_enabled": False,
        "execution_enabled": False,
        "filesystem_mutation_enabled": False,
        "product_integration_claimed": False,
    }


def expected_manifest() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "sprint_59_word_renderer_manifest",
        "story_id": "59.1",
        "admitted_ooxml_dependencies": [
            {
                "name": "quick-xml",
                "version": "0.41.0",
                "cargo_checksum_sha256": "e660451e55124f798a69a5af3f49ccfbefbd41910eefd25caf2393e1f3473ec1",
                "license": "MIT",
            },
            {
                "name": "zip",
                "version": "8.6.0",
                "cargo_checksum_sha256": "2d04a6b5381502aa6087c94c669499eb1602eb9c5e8198e534de571f7154809b",
                "license": "MIT",
            },
        ],
        "visual_comparator": {
            "implementation": "agentmage-capability-knowledge::word_visual",
            "input": "caller-supplied-decoded-rgba8",
            "synthetic_fixture_only": True,
            "renderer_execution_authority": False,
            "filesystem_authority": False,
            "network_authority": False,
        },
        "required_first_ga_platforms": ["fedora", "ubuntu", "windows11-x64"],
        "retained_post_ga_platforms": ["macos-apple-silicon"],
        "executed_local_comparator_platforms": ["fedora-x86_64"],
        "executed_native_renderer_platforms": [],
        "unadmitted_components": [
            {
                "capability": "word-renderer",
                "state": "blocked",
                "reason_code": "word.renderer.package-license-version-digest-not-admitted",
            }
        ],
        "cross_platform_render_acceptance_complete": False,
        "independent_native_review_complete": False,
    }


def validate(corpus: Any, manifest: Any) -> list[str]:
    failures = []
    if corpus != expected_corpus():
        failures.append("Sprint 59 Word review corpus drifted")
    if manifest != expected_manifest():
        failures.append("Sprint 59 Word renderer manifest drifted")
    encoded = json.dumps([corpus, manifest], sort_keys=True).lower()
    for prohibited in (
        "credential_value",
        "secret_value",
        "private_key",
        "access_token",
        "raw_document",
        "absolute_workspace_path",
        "native_renderer_complete",
        "cross_platform_render_passed",
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
        print("error: Sprint 59 Word review records are missing")
        return 1
    failures = validate(
        json.loads(CORPUS.read_text(encoding="utf-8")),
        json.loads(MANIFEST.read_text(encoding="utf-8")),
    )
    if failures:
        for failure in failures:
            print(f"error: {failure}")
        return 1
    print(f"validated {len(expected_cases())} Sprint 59 Word review cases")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
