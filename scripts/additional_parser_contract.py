#!/usr/bin/env python3
"""Validate the retained Sprint 66 additional-parser contract."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Final


ROOT: Final = Path(__file__).resolve().parents[1]
CORPUS: Final = ROOT / "docs/verification/sprint-66-parser-corpus.json"
DEPENDENCIES: Final = ROOT / "docs/verification/sprint-66-parser-dependency-manifest.json"
GROUPS: Final = ("positive_cases", "prohibited_cases", "invalid_cases", "boundary_cases", "hostile_cases", "evidence_limits")
EXPECTED_COUNTS: Final = {
    "positive_cases": 28, "prohibited_cases": 20, "invalid_cases": 31,
    "boundary_cases": 20, "hostile_cases": 24, "evidence_limits": 17,
}


def validate() -> list[str]:
    """Return deterministic contract failures."""

    failures: list[str] = []
    corpus = json.loads(CORPUS.read_text(encoding="utf-8"))
    dependencies = json.loads(DEPENDENCIES.read_text(encoding="utf-8"))
    if set(corpus) != {"schema_version", "corpus_id", *GROUPS}:
        failures.append("corpus fields drifted")
    if corpus.get("schema_version") != 1 or corpus.get("corpus_id") != "sprint-66-additional-parser-contract-v1":
        failures.append("corpus identity drifted")
    seen: set[str] = set()
    for group in GROUPS:
        values = corpus.get(group, [])
        if len(values) != EXPECTED_COUNTS[group] or len(values) != len(set(values)):
            failures.append(f"{group} count or uniqueness drifted")
        if seen.intersection(values):
            failures.append(f"case appears in multiple groups: {sorted(seen.intersection(values))[0]}")
        seen.update(values)
    if sorted(item.get("name", "") for item in dependencies.get("direct_dependencies", [])) != ["serde", "serde_json", "sha2", "zip"]:
        failures.append("dependency inventory drifted")
    if any(item.get("executes-content") is not False for item in dependencies.get("direct_dependencies", [])):
        failures.append("dependency execution authority broadened")
    for field in ("active_browser", "xml_entity_resolver", "yaml_constructor_runtime", "notebook_kernel", "archive_member_extraction"):
        if dependencies.get(field) is not False:
            failures.append(f"unsupported runtime admitted: {field}")
    for field in ("native_parsers", "network_providers", "deferred_formats_enabled"):
        if dependencies.get(field) != []:
            failures.append(f"unadmitted dependency appeared: {field}")
    source = (ROOT / "capabilities/knowledge/src/additional_parsers.rs").read_text(encoding="utf-8")
    for fragment in (
        "pub fn parse_saved_html(", "pub fn parse_bounded_xml(", "pub fn parse_notebook(",
        "pub fn parse_yaml_configuration(", "pub fn parse_structured_log(",
        "pub fn inventory_zip_archive(", "pub fn deferred_format_dispositions(",
        "member_content_opened: false", "filesystem_effect_performed: false",
        "network_access_performed: false", "execution_performed: false",
        "constructor_execution_performed: false", "archive.nested.quarantined",
    ):
        if fragment not in source:
            failures.append(f"parser boundary fragment absent: {fragment}")
    return failures


if __name__ == "__main__":
    problems = validate()
    if problems:
        for problem in problems:
            print(f"error: {problem}")
        raise SystemExit(1)
    print(f"validated {sum(EXPECTED_COUNTS.values())} Sprint 66 parser cases")
