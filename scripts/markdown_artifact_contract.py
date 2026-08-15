#!/usr/bin/env python3
"""Generate and validate the closed Sprint 57 Markdown artifact corpus."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
OUTPUT: Final = ROOT / "docs/verification/sprint-57-markdown-artifact-corpus.json"


def case(identifier: str, category: str, expected: str, requirement: str) -> dict[str, str]:
    return {
        "case_id": identifier,
        "category": category,
        "expected": expected,
        "requirement": requirement,
    }


def expected_cases() -> list[dict[str, str]]:
    cases: list[dict[str, str]] = []
    syntax = [
        "heading", "paragraph", "ordered_list", "unordered_list", "task_list", "table",
        "code_fence", "fence_content", "link", "frontmatter", "comment", "lf", "crlf",
    ]
    for index, item in enumerate(syntax, 1):
        cases.append(case(
            f"markdown-syntax-{index:02d}", "syntax",
            f"unchanged {item} retains exact bytes and source ranges", "S-049-UT01",
        ))
    quality = [
        "CommonMark spacing is visible",
        "broken local link is visible without resolution",
        "duplicate heading is visible",
        "malformed table is visible",
        "unsupported structure is preserved with a limitation",
        "long sentence or paragraph is marked without rewriting",
        "unknown acronym is marked without guessed expansion",
    ]
    for index, expected in enumerate(quality, 1):
        cases.append(case(
            f"markdown-quality-{index:02d}", "quality", expected, "S-049-I02/S-049-I03",
        ))
    scoped = [
        "heading replacement changes only the exact previewed range",
        "paragraph replacement preserves surrounding blank lines",
        "list replacement preserves unrelated items",
        "table replacement preserves unrelated rows and blocks",
        "code-fence content remains protected",
        "raw-note region remains protected",
        "mixed-format source preserves unsupported bytes",
        "long-word and narrow-width review records a visible limitation",
    ]
    for index, expected in enumerate(scoped, 1):
        cases.append(case(
            f"markdown-scoped-{index:02d}", "scoped_edit", expected, "S-049-UT02",
        ))
    artifacts = [
        "meeting cleanup", "status report", "standup script", "task document", "handoff",
        "decision record", "evidence report",
    ]
    for index, artifact in enumerate(artifacts, 1):
        cases.append(case(
            f"markdown-artifact-{index:02d}", "artifact",
            f"{artifact} renders headings, evidence states, and exact citation identities",
            "S-049-I04",
        ))
    adversarial = [
        "executable HTML remains inert and blocks generation",
        "script-like attributes remain inert and block generation",
        "remote assets are not fetched",
        "dangerous URI schemes remain inert and block generation",
        "hidden text and comments remain inert source",
        "secret canary remains visible as a blocking finding without disclosure output",
        "instruction-like source remains inert and creates no authority",
    ]
    for index, expected in enumerate(adversarial, 1):
        cases.append(case(
            f"markdown-adversarial-{index:02d}", "adversarial", expected, "S-049-ST01",
        ))
    round_trip = [
        "identical reopen passes exact byte identity",
        "identical reopen passes supported semantic identity",
        "identical local render signatures pass rendered identity",
        "changed bytes produce a named limitation",
        "changed supported structure produces a named limitation",
        "changed rendered blocks produce a named limitation",
        "any failed dimension prevents local completion",
    ]
    for index, expected in enumerate(round_trip, 1):
        cases.append(case(
            f"markdown-roundtrip-{index:02d}", "round_trip", expected, "S-049-I07/S-049-IT01",
        ))
    citations = [
        "confirmed statement requires an exact citation",
        "inferred statement requires an exact citation",
        "historical statement requires an exact citation",
        "disputed statement requires an exact citation",
        "unknown statement may remain explicitly uncited",
        "unknown citation identity rejects the artifact",
    ]
    for index, expected in enumerate(citations, 1):
        cases.append(case(
            f"markdown-citation-{index:02d}", "citation", expected, "S-049-IT01",
        ))
    display_links = [
        "validated workspace path produces a display-only file URI",
        "one-based line anchor is retained",
        "absolute path is rejected",
        "parent traversal is rejected",
        "display link grants no read write or command authority",
    ]
    for index, expected in enumerate(display_links, 1):
        cases.append(case(
            f"markdown-display-{index:02d}", "display_link", expected, "S-049-I06",
        ))
    denials = [
        "network access", "remote asset fetch", "HTML execution", "script execution",
        "source-instruction execution", "unpreviewed source mutation", "citation invention",
        "acronym expansion invention",
    ]
    for index, effect in enumerate(denials, 1):
        cases.append(case(
            f"markdown-denial-{index:02d}", "prohibited_effect", f"deny {effect}", "S-049-ST01",
        ))
    return cases


def expected_document() -> dict[str, Any]:
    cases = expected_cases()
    return {
        "schema_version": 1,
        "record_type": "sprint_57_markdown_artifact_acceptance_corpus",
        "story_id": "57.1",
        "legacy_story_id": "S-049",
        "case_count": len(cases),
        "cases": cases,
        "network_enabled": False,
        "execution_enabled": False,
        "filesystem_mutation_enabled": False,
        "citation_invention_enabled": False,
        "product_integration_claimed": False,
    }


def validate(value: Any) -> list[str]:
    failures = []
    if value != expected_document():
        failures.append("Markdown artifact corpus drifted from the closed inventory")
    encoded = json.dumps(value, sort_keys=True).lower()
    for prohibited in (
        "credential_value", "secret_value", "private_key", "access_token", "raw_document",
        "raw_message", "absolute_workspace_path",
    ):
        if prohibited in encoded:
            failures.append(f"prohibited corpus field: {prohibited}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    arguments = parser.parse_args()
    if arguments.write:
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_text(json.dumps(expected_document(), indent=2) + "\n", encoding="utf-8")
    if not OUTPUT.is_file():
        print(f"error: missing {OUTPUT.relative_to(ROOT)}")
        return 1
    failures = validate(json.loads(OUTPUT.read_text(encoding="utf-8")))
    if failures:
        for failure in failures:
            print(f"error: {failure}")
        return 1
    print(f"validated {len(expected_cases())} Sprint 57 Markdown artifact cases")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
