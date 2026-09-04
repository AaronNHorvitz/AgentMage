#!/usr/bin/env python3
"""Validate the blocked v0.6 local administrative/artifact aggregate."""

from __future__ import annotations
import json
from pathlib import Path
from typing import Final

ROOT: Final = Path(__file__).resolve().parents[1]
SPRINTS: Final = tuple(range(54, 69))
BUNDLE: Final = ROOT / "docs/verification/v0.6-administrative-local-bundle.json"
MATRIX: Final = ROOT / "docs/verification/v0.6-cross-format-local-matrix.json"
GUIDES: Final = (
    "executive-assistant-local-workflows.md", "document-control-local-workflows.md",
    "meeting-records-local-workflows.md", "controlled-writes.md", "privacy.md",
    "markdown-artifact-local-workflows.md", "pdf-generation-review.md", "tabular-data-review.md",
    "word-artifact-local-workflows.md", "presentation-review.md", "image-redaction-and-visual-review.md",
    "additional-parser-review.md", "common-artifact-receipt-review.md", "local-database-review.md",
)


def validate() -> list[str]:
    """Return deterministic aggregate failures."""
    failures: list[str] = []
    bundle = json.loads(BUNDLE.read_text(encoding="utf-8"))
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    expected_reports = [f"artifacts/sprints/sprint-{number}/local-evidence-report.json" for number in SPRINTS if number != 64]
    if bundle.get("source_reports") != expected_reports: failures.append("source report inventory drifted")
    for path in expected_reports:
        report = json.loads((ROOT / path).read_text(encoding="utf-8"))
        if report.get("summary", {}).get("sprint_status") != "BLOCKED": failures.append(f"source sprint not blocked: {path}")
        if report.get("summary", {}).get("release_approval") is not False: failures.append(f"source release overclaim: {path}")
        if any(command.get("exit_code") != 0 for command in report.get("commands", [])): failures.append(f"source command failure: {path}")
        if not report.get("source_sha256"): failures.append(f"source binding absent: {path}")
    expected_sprint_64 = [
        "docs/verification/sprint-64-local-results.md",
        "docs/verification/sprint-64-presentation-corpus.json",
        "docs/verification/sprint-64-presentation-dependency-manifest.json",
    ]
    if bundle.get("supplemental_sprint_64_sources") != expected_sprint_64:
        failures.append("Sprint 64 committed source inventory drifted")
    if any(not (ROOT / path).is_file() for path in expected_sprint_64):
        failures.append("Sprint 64 committed source absent")
    for field in ("sending_enabled", "live_calendar_changes_enabled", "messaging_enabled", "external_database_access_enabled", "automatic_recipient_selection_enabled", "unattended_disposition_enabled", "release_approval"):
        if bundle.get(field) is not False: failures.append(f"disabled capability broadened: {field}")
    if matrix.get("sprints") != list(SPRINTS) or matrix.get("supported_format_count") != 0 or matrix.get("supported_platform_count") != 0 or matrix.get("release_approval") is not False:
        failures.append("cross-format support truth drifted")
    if any(row.get("release_supported") is not False for row in matrix.get("format_classes", [])):
        failures.append("format support overclaimed")
    for guide in GUIDES:
        if not (ROOT / "docs/guides" / guide).is_file(): failures.append(f"guide absent: {guide}")
    return failures


if __name__ == "__main__":
    problems = validate()
    if problems:
        for problem in problems: print(f"error: {problem}")
        raise SystemExit(1)
    print("validated 14 bound reports plus Sprint 64 committed sources, 8 workflows, and 10 format classes")
