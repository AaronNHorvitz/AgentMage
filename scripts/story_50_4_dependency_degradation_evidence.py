#!/usr/bin/env python3
"""Generate and validate source-bound local Story 50.4 degradation evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-50/story-50.4-dependency-degradation"
RAW_PATH: Final = EVIDENCE_DIR / "local-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "report.json"
COMMANDS: Final = (
    ("cargo", "test", "-p", "agentmage-kernel-engine", "--test", "dependency_degradation", "--all-features", "--locked"),
    ("cargo", "clippy", "-p", "agentmage-kernel-engine", "--all-targets", "--all-features", "--locked", "--", "-D", "warnings"),
    ("python3", "-m", "pytest", "-q", "tests/test_story_50_4_dependency_degradation_evidence.py", "tests/test_task_graph.py"),
    ("python3", "scripts/validate_docs.py"),
)
SOURCES: Final = (
    "ENGINEERING-RUNTIME.md",
    "requirements/registry.json",
    "kernel/engine/src/dependency_degradation.rs",
    "kernel/engine/tests/dependency_degradation.rs",
    "docs/architecture/runtime-dependency-degradation.md",
    "scripts/story_50_4_dependency_degradation_evidence.py",
    "tests/test_story_50_4_dependency_degradation_evidence.py",
)


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def artifact(relative: str) -> dict[str, Any]:
    path = ROOT / relative
    return {"path": relative, "byte_length": path.stat().st_size, "sha256": digest(path)}


def expected_report() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "agentmage-story-50-4-dependency-degradation-evidence",
        "story_id": "50.4",
        "requirement_ids": ["AM-DEG-001", "AT-DEG-001"],
        "generated_on": "2026-08-31",
        "status": "PASS_LOCAL_DEPENDENCY_DEGRADATION",
        "commands": [" ".join(command) for command in COMMANDS],
        "artifacts": [artifact(path) for path in SOURCES] + [artifact(RAW_PATH.relative_to(ROOT).as_posix())],
        "product_truth": {
            "all_dependency_families_classified": True,
            "required_loss_blocks": True,
            "optional_loss_visible_and_bounded": True,
            "disabled_and_quarantined_visible": True,
            "unavailable_substitute_visible": True,
            "fresh_qualified_substitution_only": True,
            "authority_broadening_permitted": False,
            "security_or_verification_weakening_permitted": False,
            "silent_omission_or_route_change": False,
            "false_success_authority": False,
            "live_dependency_removal_campaign_complete": False,
            "qualified_model_or_endpoint_campaign_complete": False,
            "installed_client_campaign_complete": False,
            "independent_review_complete": False,
            "windows_validation_complete": False,
            "macos_validation_complete": False,
            "release_claim": "none",
        },
        "remaining_external_work": [
            "repeat lifecycle removal against admitted live model and endpoint tuples",
            "repeat against installed Chat, Verified Chat, CLI, and headless clients",
            "run final local Windows validation against the exact final candidate commit",
            "run deferred macOS validation against the exact final candidate commit",
            "obtain independent runtime, security, and release review",
        ],
    }


def validate_report(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["Story 50.4 degradation report is not an object"]
    truth = value.get("product_truth", {})
    required_true = (
        "all_dependency_families_classified", "required_loss_blocks",
        "optional_loss_visible_and_bounded", "disabled_and_quarantined_visible",
        "unavailable_substitute_visible", "fresh_qualified_substitution_only",
    )
    required_false = (
        "authority_broadening_permitted", "security_or_verification_weakening_permitted",
        "silent_omission_or_route_change", "false_success_authority",
        "live_dependency_removal_campaign_complete", "qualified_model_or_endpoint_campaign_complete",
        "installed_client_campaign_complete", "independent_review_complete",
        "windows_validation_complete", "macos_validation_complete",
    )
    failures = [f"{field} must remain true" for field in required_true if truth.get(field) is not True]
    failures.extend(f"{field} must remain false" for field in required_false if truth.get(field) is not False)
    if truth.get("release_claim") != "none":
        failures.append("Story 50.4 degradation evidence cannot claim a release")
    return failures


def validate() -> list[str]:
    failures = [f"missing source: {path}" for path in SOURCES if not (ROOT / path).is_file()]
    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return failures + [f"cannot read retained Story 50.4 degradation evidence: {error}"]
    failures.extend(
        f"command {index} did not retain a passing exit"
        for index in range(len(COMMANDS)) if f"COMMAND_{index}_EXIT=0" not in raw
    )
    for marker in (
        "every_dependency_family_is_classified_and_available ... ok",
        "optional_loss_remains_permitted_but_never_silent ... ok",
        "fresh_narrower_stronger_same_kind_substitute_is_explicitly_selected ... ok",
        "missing_stale_hidden_cross_kind_or_weaker_substitution_is_denied ... ok",
    ):
        if marker not in raw:
            failures.append(f"raw evidence lacks fixture marker: {marker}")
    for prohibited in ("test result: FAILED", "not ok", "BEGIN PRIVATE KEY"):
        if prohibited in raw:
            failures.append(f"raw evidence contains prohibited marker: {prohibited}")
    failures.extend(validate_report(report))
    if report != expected_report():
        failures.append("Story 50.4 degradation report is stale or widened")
    return failures


def capture() -> int:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    RAW_PATH.write_text("Story 50.4 degradation capture in progress\n", encoding="utf-8")
    REPORT_PATH.write_text("{}\n", encoding="utf-8")
    records: list[str] = []
    for index, command in enumerate(COMMANDS):
        result = subprocess.run(command, cwd=ROOT, text=True, stdout=subprocess.PIPE,
                                stderr=subprocess.STDOUT, check=False)
        records.append(f"$ {' '.join(command)}\n{result.stdout}\nCOMMAND_{index}_EXIT={result.returncode}")
        if result.returncode != 0:
            RAW_PATH.write_text("\n\n".join(records) + "\n", encoding="utf-8")
            return result.returncode
    RAW_PATH.write_text("\n\n".join(records) + "\n", encoding="utf-8")
    REPORT_PATH.write_text(json.dumps(expected_report(), indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    if args.write and capture() != 0:
        return 1
    failures = validate()
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    print("Story 50.4 local dependency-degradation evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
