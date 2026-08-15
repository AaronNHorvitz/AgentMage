#!/usr/bin/env python3
"""Validate the retained Sprint 64 presentation review contract."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Final


ROOT: Final = Path(__file__).resolve().parents[1]
CORPUS: Final = ROOT / "docs/verification/sprint-64-presentation-corpus.json"
DEPENDENCIES: Final = ROOT / "docs/verification/sprint-64-presentation-dependency-manifest.json"
GROUPS: Final = (
    "positive_cases", "prohibited_cases", "invalid_cases", "boundary_cases", "evidence_limits",
)
EXPECTED_COUNTS: Final = {
    "positive_cases": 14,
    "prohibited_cases": 8,
    "invalid_cases": 17,
    "boundary_cases": 13,
    "evidence_limits": 5,
}


def validate() -> list[str]:
    """Return deterministic contract failures."""

    failures: list[str] = []
    corpus = json.loads(CORPUS.read_text(encoding="utf-8"))
    dependencies = json.loads(DEPENDENCIES.read_text(encoding="utf-8"))
    if set(corpus) != {"schema_version", "corpus_id", *GROUPS}:
        failures.append("corpus fields drifted")
    if corpus.get("schema_version") != 1 or corpus.get("corpus_id") != "sprint-64-presentation-contract-v1":
        failures.append("corpus identity drifted")
    seen: set[str] = set()
    for group in GROUPS:
        values = corpus.get(group, [])
        if len(values) != EXPECTED_COUNTS[group] or len(values) != len(set(values)):
            failures.append(f"{group} count or uniqueness drifted")
        overlap = seen.intersection(values)
        if overlap:
            failures.append(f"case appears in multiple groups: {sorted(overlap)[0]}")
        seen.update(values)
    expected_dependencies = ["quick-xml", "serde", "serde_json", "zip"]
    actual_dependencies = sorted(
        item.get("name", "") for item in dependencies.get("direct_open_xml_dependencies", [])
    )
    if actual_dependencies != expected_dependencies:
        failures.append("direct dependency inventory drifted")
    if any(item.get("executes-content") is not False for item in dependencies.get("direct_open_xml_dependencies", [])):
        failures.append("dependency execution authority broadened")
    for field in (
        "external_media_resolution", "office_automation", "macro_execution",
        "native_visual_evidence_admitted", "native_accessibility_evidence_admitted",
    ):
        if dependencies.get(field) is not False:
            failures.append(f"unsupported capability admitted: {field}")
    if dependencies.get("native_renderers") != [] or dependencies.get("network_providers") != []:
        failures.append("unadmitted renderer or provider appeared")

    generator = (ROOT / "capabilities/knowledge/src/presentation_generation.rs").read_text(encoding="utf-8")
    inspector = (ROOT / "capabilities/knowledge/src/presentation_ooxml.rs").read_text(encoding="utf-8")
    for fragment in (
        "pub fn generate_presentation(", "pub fn edit_generated_presentation(",
        "PresentationBlock::Plot", "fn plot_xml(", "native_render_required: true", "proposal_only: true",
        "network_access_performed: false", "execution_performed: false",
    ):
        if fragment not in generator:
            failures.append(f"generation boundary fragment absent: {fragment}")
    for fragment in (
        "pub fn inspect_pptx(", "PresentationFindingKind::MacroContent",
        "PresentationFindingKind::EmbeddedExecutableContent",
        "PresentationFindingKind::ExternalMedia", "PresentationFindingKind::ActiveAction",
        "followed: false", "execution_performed: false",
    ):
        if fragment not in inspector:
            failures.append(f"inspection boundary fragment absent: {fragment}")
    registry = (ROOT / "scripts/validate_planning_schemas.mjs").read_text(encoding="utf-8")
    for record_type in ("presentation-inspection", "generated-presentation", "edited-presentation"):
        if f'"{record_type}"' not in registry:
            failures.append(f"runtime record absent: {record_type}")
    return failures


if __name__ == "__main__":
    problems = validate()
    if problems:
        for problem in problems:
            print(f"error: {problem}")
        raise SystemExit(1)
    print(f"validated {sum(EXPECTED_COUNTS.values())} Sprint 64 presentation cases")
