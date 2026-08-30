#!/usr/bin/env python3
"""Validate and retain the consolidated Story 5.2 policy-evidence map."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE: Final = ROOT / "artifacts/sprints/sprint-5/story-5.2"
REPORT: Final = EVIDENCE / "policy-verification-report.json"
DOC: Final = ROOT / "docs/verification/story-5-2-policy-evidence.md"
SOURCE_REPORTS: Final = (
    "artifacts/sprints/sprint-5/story-5.2/effect-class-taxonomy-report.json",
    "artifacts/sprints/sprint-5/story-5.2/retry-repair-policy-report.json",
    "artifacts/sprints/sprint-5/story-5.2/workflow-supervision-report.json",
)
REQUIRED_HEADINGS: Final = (
    "## Effect and retry decision table",
    "## Registered-operation coverage",
    "## Admission and termination state",
    "## Mutation and race results",
    "## Reviewer-protocol mapping",
)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def artifact(path: str) -> dict[str, Any]:
    value = ROOT / path
    return {"path": path, "byte_length": value.stat().st_size, "sha256": sha256(value)}


def load(path: str) -> dict[str, Any]:
    return json.loads((ROOT / path).read_text(encoding="utf-8"))


def expected_report() -> dict[str, Any]:
    taxonomy, retry, supervision = (load(path) for path in SOURCE_REPORTS)
    mappings = taxonomy["operation_effect_mappings"]
    return {
        "schema_version": 1,
        "record_type": "agentmage-story-5.2-policy-verification-evidence",
        "story_id": "5.2",
        "task_id": "5.2.4.3",
        "generated_on": "2026-08-30",
        "status": "pass-local-contract-evidence",
        "decision_table": {
            "effect_classes": taxonomy["effect_classes"],
            "failure_classes": taxonomy["failure_classes"],
            "registered_operation_count": len(mappings),
            "operation_effect_mappings": mappings,
        },
        "mutation_results": retry["mutation_contract"],
        "race_results": retry["race_contract"],
        "supervision_results": {
            "budget_dimensions": supervision["budget_dimensions"],
            "repeated_state_contract": supervision["repeated_state_contract"],
            "termination_contract": supervision["termination_contract"],
        },
        "review_protocols": [
            {"protocol": "RV-12", "status": "demonstrated-story-scope", "product_complete": False},
            {"protocol": "RV-17", "status": "partial-contribution", "product_complete": False},
            {"protocol": "RV-25", "status": "partial-prerequisite-only", "product_complete": False},
        ],
        "artifacts": [artifact(path) for path in SOURCE_REPORTS]
        + [artifact("docs/verification/story-5-2-policy-evidence.md")],
        "product_truth": {
            "synthetic_data_only": True,
            "external_effect_executed": False,
            "native_platform_claim": "none",
            "independent_review_claim": "none",
            "story_completion_claim": False,
            "sprint_completion_claim": False,
            "release_claim": "none",
        },
    }


def validate(value: Any) -> list[str]:
    failures: list[str] = []
    if value != expected_report():
        failures.append("consolidated policy report is stale, incomplete, reordered, or widened")
    text = DOC.read_text(encoding="utf-8")
    for heading in REQUIRED_HEADINGS:
        if heading not in text:
            failures.append(f"policy evidence document missing heading: {heading}")
    if text.count("```mermaid") != 2:
        failures.append("policy evidence document must retain exactly two state diagrams")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    for command in (
        [sys.executable, "scripts/effect_class_taxonomy_evidence.py"],
        [sys.executable, "scripts/retry_repair_policy_evidence.py"],
        [sys.executable, "scripts/workflow_supervision_evidence.py"],
    ):
        if subprocess.run(command, cwd=ROOT, check=False).returncode != 0:
            return 1
    if args.write:
        EVIDENCE.mkdir(parents=True, exist_ok=True)
        REPORT.write_text(json.dumps(expected_report(), indent=2, sort_keys=True) + "\n", encoding="utf-8")
    try:
        value = json.loads(REPORT.read_text(encoding="utf-8"))
        failures = validate(value)
    except (OSError, json.JSONDecodeError) as error:
        failures = [str(error)]
    if failures:
        for failure in failures:
            print(f"Story 5.2 policy evidence failed: {failure}", file=sys.stderr)
        return 1
    print("Story 5.2 policy evidence validated through Sub-task 5.2.4.3")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
