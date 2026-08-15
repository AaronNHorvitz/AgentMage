#!/usr/bin/env python3
"""Generate and validate the exact authority-free Markdown artifact skill pack."""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-57/markdown-artifact-skill-pack.json"
SKILLS: Final = [
    "meeting_cleanup",
    "status_report",
    "standup_script",
    "task_document",
    "handoff",
    "decision_record",
    "evidence_report",
]
WORKFLOW_IDS: Final = [
    "markdown-meeting-cleanup",
    "markdown-status-report",
    "markdown-standup-script",
    "markdown-task-document",
    "markdown-handoff",
    "markdown-decision-record",
    "markdown-evidence-report",
]
ZERO_EFFECT_FIELDS: Final = [
    "product_registration",
    "filesystem_access",
    "network_access",
    "execution_access",
    "renderer_access",
    "source_mutation",
    "citation_invention",
    "acronym_expansion",
    "remote_asset_fetch",
]


def build() -> Any:
    result = subprocess.run(
        [
            "cargo",
            "run",
            "-p",
            "agentmage-capability-knowledge",
            "--example",
            "markdown_artifact_skill_pack",
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
        return ["Markdown artifact skill pack must be an object"]
    expected_fields = {
        "schema_version",
        "record_type",
        "version",
        "skills",
        "manifests",
        "prompt_count",
        "template_count",
        "authority",
        *ZERO_EFFECT_FIELDS,
    }
    if set(value) != expected_fields:
        failures.append("Markdown artifact skill pack field closure drifted")
    if (
        value.get("schema_version") != 1
        or value.get("record_type") != "agentmage-markdown-artifact-skill-pack"
        or value.get("version") != "0.6.0"
    ):
        failures.append("Markdown artifact skill pack identity drifted")
    if value.get("skills") != SKILLS:
        failures.append("Markdown artifact skill inventory drifted")
    manifests = value.get("manifests", [])
    if (
        not isinstance(manifests, list)
        or len(manifests) != len(SKILLS)
        or value.get("prompt_count") != len(SKILLS)
        or value.get("template_count") != len(SKILLS)
    ):
        failures.append("Markdown artifact skill pack cardinality drifted")
        return failures
    authority = value.get("authority", {})
    if not isinstance(authority, dict) or not authority or any(authority.values()):
        failures.append("Markdown artifact skill pack gained authority")
    for field in ZERO_EFFECT_FIELDS:
        if value.get(field) is not False:
            failures.append(f"Markdown artifact skill pack effect overclaim: {field}")
    for skill, workflow_id, manifest in zip(SKILLS, WORKFLOW_IDS, manifests, strict=True):
        files = manifest.get("files", [])
        if (
            manifest.get("skill_id") != f"skill-{workflow_id}"
            or manifest.get("trust_state") != "admitted"
            or manifest.get("version") != "0.6.0"
            or len(files) != 2
            or [item.get("kind") for item in files] != ["prompt", "template"]
        ):
            failures.append(f"Markdown artifact skill manifest drifted: {skill}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    arguments = parser.parse_args()
    current = build()
    failures = validate(current)
    if not arguments.write:
        if not OUTPUT.is_file():
            failures.append("Markdown artifact skill pack artifact is absent")
        elif json.loads(OUTPUT.read_text(encoding="utf-8")) != current:
            failures.append("Markdown artifact skill pack artifact is stale")
    if failures:
        print("Markdown artifact skill contract failed:")
        for failure in failures:
            print(f"- {failure}")
        return 1
    if arguments.write:
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_text(
            json.dumps(current, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
    print("Markdown artifact skill contract validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
