#!/usr/bin/env python3
"""Validate Sprint 68 bounded local database artifacts and source boundaries."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Final

ROOT: Final = Path(__file__).resolve().parents[1]
CORPUS: Final = ROOT / "docs/verification/sprint-68-database-corpus.json"
DEPENDENCIES: Final = ROOT / "docs/verification/sprint-68-database-dependency-manifest.json"
GROUPS: Final = ("positive_cases", "prohibited_cases", "invalid_cases", "boundary_cases", "hostile_cases")
COUNTS: Final = {"positive_cases": 14, "prohibited_cases": 12, "invalid_cases": 12, "boundary_cases": 10, "hostile_cases": 10}


def validate() -> list[str]:
    """Return deterministic contract failures."""
    failures: list[str] = []
    corpus = json.loads(CORPUS.read_text(encoding="utf-8"))
    dependencies = json.loads(DEPENDENCIES.read_text(encoding="utf-8"))
    seen: set[str] = set()
    for group in GROUPS:
        values = corpus.get(group, [])
        if len(values) != COUNTS[group] or len(values) != len(set(values)):
            failures.append(f"{group} count or uniqueness drifted")
        if seen.intersection(values): failures.append("database case appears in multiple groups")
        seen.update(values)
    if sorted(item.get("name") for item in dependencies.get("direct_dependencies", [])) != ["rusqlite", "serde", "serde_json", "sha2"]:
        failures.append("dependency inventory drifted")
    for field in ("live_connectors", "external_credentials", "path_openers", "postgresql_adapters", "extension_loaders"):
        if dependencies.get(field) != []: failures.append(f"unadmitted dependency appeared: {field}")
    source = (ROOT / "capabilities/knowledge/src/database_adapter.rs").read_text(encoding="utf-8")
    for fragment in (
        "pub fn from_synthetic_fixture(", "pub fn from_agentmage_owned_rows(",
        "pub fn refuse_unavailable_adapter(", "pub fn query(", "PRAGMA query_only=ON",
        "statement.readonly()", "WHERE category=?1", "ORDER BY row_id", "DatabaseSourceKind::LiveExternal",
        "filesystem_effect_performed: false", "network_effect_performed: false",
    ):
        if fragment not in source: failures.append(f"database boundary fragment absent: {fragment}")
    for forbidden in ("Connection::open(", "ATTACH DATABASE", "load_extension_enable", "postgres://", "password="):
        if forbidden in source: failures.append(f"forbidden database surface present: {forbidden}")
    return failures


if __name__ == "__main__":
    problems = validate()
    if problems:
        for problem in problems: print(f"error: {problem}")
        raise SystemExit(1)
    print(f"validated {sum(COUNTS.values())} Sprint 68 bounded database cases")
