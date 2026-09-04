#!/usr/bin/env python3
"""Validate the retained Sprint 67 common-receipt contract."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Final


ROOT: Final = Path(__file__).resolve().parents[1]
MATRIX: Final = ROOT / "docs/verification/sprint-67-fidelity-unsupported-matrix.json"
DEPENDENCIES: Final = ROOT / "docs/verification/sprint-67-artifact-dependency-manifest.json"
PARSER_CORPUS: Final = ROOT / "docs/verification/sprint-66-parser-corpus.json"
OPERATION_KINDS: Final = (
    "converter", "generator", "parser", "redactor", "renderer", "transcriber", "verifier",
)


def validate() -> list[str]:
    """Return deterministic contract failures."""

    failures: list[str] = []
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    dependencies = json.loads(DEPENDENCIES.read_text(encoding="utf-8"))
    corpus = json.loads(PARSER_CORPUS.read_text(encoding="utf-8"))
    rows = matrix.get("operation_classes", [])
    if [row.get("operation_kind") for row in rows] != list(OPERATION_KINDS):
        failures.append("operation class inventory drifted")
    if any(row.get("local_receipt_contract") is not True for row in rows):
        failures.append("local receipt class missing")
    if any(row.get("actual_every_type_integration") is not False for row in rows):
        failures.append("every-type integration overclaimed")
    if matrix.get("receipt_effect_authority") != {
        "filesystem": False, "network": False, "content_execution": False,
    }:
        failures.append("receipt effect authority broadened")
    for field in (
        "audio_engines", "audio_models", "codec_packages", "network_providers",
        "filesystem_adapters", "content_execution_adapters",
    ):
        if dependencies.get(field) != []:
            failures.append(f"unadmitted dependency appeared: {field}")
    if sum(len(corpus.get(group, [])) for group in (
        "positive_cases", "prohibited_cases", "invalid_cases", "boundary_cases",
        "hostile_cases", "evidence_limits",
    )) != 140:
        failures.append("Sprint 66 hostile parser corpus drifted")
    source = (ROOT / "capabilities/knowledge/src/artifact_receipt.rs").read_text(encoding="utf-8")
    for fragment in (
        "pub fn build_common_artifact_receipt(", "pub fn verify_common_artifact_receipt(",
        "pub fn validate_audio_transcription_observation(", "ArtifactOperationKind::Converter",
        "ArtifactOperationKind::Transcriber", "accuracy_evidence_required: true",
        "network_access_performed", "filesystem_effect_performed",
    ):
        if fragment not in source:
            failures.append(f"receipt boundary fragment absent: {fragment}")
    return failures


if __name__ == "__main__":
    problems = validate()
    if problems:
        for problem in problems:
            print(f"error: {problem}")
        raise SystemExit(1)
    print("validated seven receipt classes and the retained 140-case hostile parser corpus")
