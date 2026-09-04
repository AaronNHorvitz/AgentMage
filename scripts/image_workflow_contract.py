#!/usr/bin/env python3
"""Validate the retained Sprint 65 image and visual-workflow contract."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Final


ROOT: Final = Path(__file__).resolve().parents[1]
CORPUS: Final = ROOT / "docs/verification/sprint-65-image-corpus.json"
DEPENDENCIES: Final = ROOT / "docs/verification/sprint-65-image-dependency-manifest.json"
GROUPS: Final = (
    "positive_cases", "prohibited_cases", "invalid_cases", "boundary_cases", "hostile_cases",
    "evidence_limits",
)
EXPECTED_COUNTS: Final = {
    "positive_cases": 19, "prohibited_cases": 12, "invalid_cases": 27,
    "boundary_cases": 17, "hostile_cases": 12, "evidence_limits": 12,
}


def validate() -> list[str]:
    """Return deterministic contract failures."""

    failures: list[str] = []
    corpus = json.loads(CORPUS.read_text(encoding="utf-8"))
    dependencies = json.loads(DEPENDENCIES.read_text(encoding="utf-8"))
    if set(corpus) != {"schema_version", "corpus_id", *GROUPS}:
        failures.append("corpus fields drifted")
    if corpus.get("schema_version") != 1 or corpus.get("corpus_id") != "sprint-65-image-contract-v1":
        failures.append("corpus identity drifted")
    seen: set[str] = set()
    for group in GROUPS:
        values = corpus.get(group, [])
        if len(values) != EXPECTED_COUNTS[group] or len(values) != len(set(values)):
            failures.append(f"{group} count or uniqueness drifted")
        if seen.intersection(values):
            failures.append(f"case appears in multiple groups: {sorted(seen.intersection(values))[0]}")
        seen.update(values)
    if sorted(item.get("name", "") for item in dependencies.get("direct_image_dependencies", [])) != ["serde", "serde_json", "sha2"]:
        failures.append("direct dependency inventory drifted")
    if any(item.get("executes-content") is not False for item in dependencies.get("direct_image_dependencies", [])):
        failures.append("dependency execution authority broadened")
    for field in (
        "screenshot_capture", "embedded_content_execution", "native_visual_evidence_admitted",
        "native_accessibility_evidence_admitted",
    ):
        if dependencies.get(field) is not False:
            failures.append(f"unsupported capability admitted: {field}")
    for field in ("native_viewers", "native_renderers", "network_providers"):
        if dependencies.get(field) != []:
            failures.append(f"unadmitted dependency appeared: {field}")
    source = (ROOT / "capabilities/knowledge/src/image_workflows.rs").read_text(encoding="utf-8")
    for fragment in (
        "pub fn inspect_image(", "pub fn decode_bmp_rgba(", "pub fn encode_bmp_rgba(",
        "pub fn prepare_image_view(", "pub fn redact_image(", "pub fn compare_images(",
        "pub fn preview_image_generation(", "pub fn record_image_generation(",
        "ImageWorkflowError::SensitiveContent", "ImageWorkflowError::RouteDenied",
        "filesystem_effect_performed: false", "network_access_performed: false",
        "execution_performed: false",
    ):
        if fragment not in source:
            failures.append(f"image boundary fragment absent: {fragment}")
    registry = (ROOT / "scripts/validate_planning_schemas.mjs").read_text(encoding="utf-8")
    for record_type in ("image-inspection", "image-redaction-receipt", "image-visual-comparison"):
        if f'"{record_type}"' not in registry:
            failures.append(f"runtime record absent: {record_type}")
    return failures


if __name__ == "__main__":
    problems = validate()
    if problems:
        for problem in problems:
            print(f"error: {problem}")
        raise SystemExit(1)
    print(f"validated {sum(EXPECTED_COUNTS.values())} Sprint 65 image cases")
