#!/usr/bin/env python3
"""Generate and validate the exact authority-free v0.6 executive skill pack."""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-54/executive-skill-pack.json"
SKILLS: Final = [
    "cycle_briefing",
    "trackers",
    "priority_assistant",
    "evidence_briefs",
    "correspondence_review",
    "local_message_triage",
    "portfolio_review",
    "privacy_audit",
]
ZERO_EFFECT_FIELDS: Final = [
    "product_registration",
    "inbox_access",
    "network_access",
    "notification_access",
    "send_access",
    "schedule_access",
    "source_mutation",
]


def build() -> Any:
    result = subprocess.run(
        [
            "cargo",
            "run",
            "-p",
            "agentmage-capability-knowledge",
            "--example",
            "executive_skill_pack",
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
        return ["executive skill pack must be an object"]
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
        failures.append("executive skill pack field closure drifted")
    if (
        value.get("schema_version") != 1
        or value.get("record_type") != "agentmage-executive-skill-pack"
        or value.get("version") != "0.6.0"
    ):
        failures.append("executive skill pack identity drifted")
    if value.get("skills") != SKILLS:
        failures.append("executive skill inventory drifted")
    manifests = value.get("manifests", [])
    if (
        not isinstance(manifests, list)
        or len(manifests) != len(SKILLS)
        or value.get("prompt_count") != len(SKILLS)
        or value.get("template_count") != len(SKILLS)
    ):
        failures.append("executive skill pack cardinality drifted")
        return failures
    authority = value.get("authority", {})
    if not isinstance(authority, dict) or not authority or any(authority.values()):
        failures.append("executive skill pack gained authority")
    for field in ZERO_EFFECT_FIELDS:
        if value.get(field) is not False:
            failures.append(f"executive skill pack effect overclaim: {field}")
    for skill, manifest in zip(SKILLS, manifests, strict=True):
        files = manifest.get("files", [])
        if (
            manifest.get("skill_id") != f"skill-executive-{skill.replace('_', '-')}"
            or manifest.get("trust_state") != "admitted"
            or manifest.get("version") != "0.6.0"
            or len(files) != 2
            or [item.get("kind") for item in files] != ["prompt", "template"]
        ):
            failures.append(f"executive skill manifest drifted: {skill}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    arguments = parser.parse_args()
    current = build()
    failures = validate(current)
    if not arguments.write:
        if not OUTPUT.is_file():
            failures.append("executive skill pack artifact is absent")
        elif json.loads(OUTPUT.read_text(encoding="utf-8")) != current:
            failures.append("executive skill pack artifact is stale")
    if failures:
        print("executive skill contract failed:")
        for failure in failures:
            print(f"- {failure}")
        return 1
    if arguments.write:
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_text(
            json.dumps(current, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
    print("executive skill contract validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
