#!/usr/bin/env python3
"""Generate and validate the exact authority-free v0.4 coding skill pack."""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-50/coding-skill-pack.json"
SKILLS: Final = [
    "repository_cartographer",
    "feature_trace",
    "change_impact",
    "debugging",
    "test_and_verification",
    "repository_documentation",
    "bug_reproduction",
    "git_history_analysis",
    "bounded_review",
]
PROHIBITED: Final = [
    "arbitrary-shell",
    "automatic-commit",
    "automatic-dependency-upgrade",
    "automatic-deploy",
    "automatic-merge",
    "automatic-migration",
    "automatic-pr-publication",
    "automatic-push",
    "automatic-refactor",
    "automatic-release",
    "frontier-handoff",
    "network-publication",
    "unattended-write",
]


def build() -> Any:
    result = subprocess.run(
        [
            "cargo",
            "run",
            "-p",
            "agentmage-capability-knowledge",
            "--example",
            "coding_skill_pack",
            "--locked",
            "--quiet",
        ],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
        timeout=600,
    )
    return json.loads(result.stdout)


def validate(value: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict):
        return ["coding skill pack must be an object"]
    expected_fields = {
        "schema_version",
        "record_type",
        "version",
        "definitions",
        "assessments",
        "manifests",
        "product_registration",
        "authority_enabled",
        "network_access",
        "automatic_publication",
    }
    if set(value) != expected_fields:
        failures.append("coding skill pack field closure drifted")
    if (
        value.get("schema_version") != 1
        or value.get("record_type") != "agentmage-coding-skill-pack"
        or value.get("version") != "0.4.0"
    ):
        failures.append("coding skill pack identity drifted")
    for field in (
        "product_registration",
        "authority_enabled",
        "network_access",
        "automatic_publication",
    ):
        if value.get(field) is not False:
            failures.append(f"coding skill pack authority overclaim: {field}")

    definitions = value.get("definitions", [])
    assessments = value.get("assessments", [])
    manifests = value.get("manifests", [])
    if not all(isinstance(items, list) and len(items) == len(SKILLS) for items in (
        definitions,
        assessments,
        manifests,
    )):
        failures.append("coding skill pack cardinality drifted")
        return failures
    if [item.get("skill") for item in definitions] != SKILLS:
        failures.append("coding skill inventory drifted")
    if [item.get("skill") for item in assessments] != SKILLS:
        failures.append("coding skill assessment inventory drifted")
    for definition, assessment, manifest in zip(
        definitions, assessments, manifests, strict=True
    ):
        if (
            assessment.get("admission") != "admitted"
            or assessment.get("findings") != []
            or assessment.get("authority_granted") is not False
        ):
            failures.append(f"coding skill was not admitted safely: {definition.get('skill')}")
        authority = definition.get("authority", {})
        if not isinstance(authority, dict) or any(authority.values()):
            failures.append(f"coding skill gained authority: {definition.get('skill')}")
        if definition.get("prohibited_operations") != PROHIBITED:
            failures.append(f"coding skill exclusions drifted: {definition.get('skill')}")
        if (
            manifest.get("trust_state") != "admitted"
            or manifest.get("version") != "0.4.0"
            or len(manifest.get("files", [])) != 1
        ):
            failures.append(f"coding skill manifest drifted: {definition.get('skill')}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    arguments = parser.parse_args()
    current = build()
    failures = validate(current)
    if not arguments.write:
        if not OUTPUT.is_file():
            failures.append("coding skill pack artifact is absent")
        elif json.loads(OUTPUT.read_text(encoding="utf-8")) != current:
            failures.append("coding skill pack artifact is stale")
    if failures:
        print("coding skill contract failed:")
        for failure in failures:
            print(f"- {failure}")
        return 1
    if arguments.write:
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_text(
            json.dumps(current, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
    print("coding skill contract validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
