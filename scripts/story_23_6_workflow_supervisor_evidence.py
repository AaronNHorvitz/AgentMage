#!/usr/bin/env python3
"""Generate and validate source-bound local Story 23.6 evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-23/story-23.6"
RAW_PATH: Final = EVIDENCE_DIR / "workflow-supervisor-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "workflow-supervisor-report.json"
COMMANDS: Final = (
    (
        "cargo", "test", "-p", "agentmage-kernel-engine",
        "verified_workflow_supervisor", "--all-features", "--locked", "--", "--nocapture",
    ),
    (
        "cargo", "test", "-p", "agentmage-host",
        "workflow_supervisor", "--all-features", "--locked",
    ),
    (
        "cargo", "test", "-p", "agentmage-host",
        "runtime_parity_tests", "--all-features", "--locked",
    ),
    (
        "cargo", "test", "-p", "agentmage-kernel-engine",
        "story_23_4_", "--all-features", "--locked",
    ),
    (
        "cargo", "test", "-p", "agentmage-host",
        "story_22_5_prepared_source_flows", "--all-features", "--locked",
    ),
    (
        "cargo", "test", "-p", "agentmage-host",
        "story_23_4_fake_model_composes_multiple_reads", "--all-features", "--locked",
    ),
    ("python3", "scripts/validate_docs.py"),
)
SOURCES: Final = (
    "kernel/engine/src/verified_workflow_supervisor.rs",
    "kernel/engine/src/lib.rs",
    "shells/host/src/workflow_supervisor.rs",
    "shells/host/src/coding_client.rs",
    "shells/host/src/coding_harness.rs",
    "shells/host/src/runtime_read_tests.rs",
    "shells/host/src/lib.rs",
    "docs/architecture/verified-workflow-supervisor.md",
    "docs/architecture/foundational-artifact-and-workflow-runtime.md",
    "scripts/story_23_6_workflow_supervisor_evidence.py",
    "tests/test_story_23_6_workflow_supervisor_evidence.py",
)


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def artifact(relative: str) -> dict[str, Any]:
    path = ROOT / relative
    return {
        "path": relative,
        "byte_length": path.stat().st_size,
        "sha256": digest(path),
    }


def expected_report() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "agentmage-story-23-6-workflow-supervisor-evidence",
        "story_id": "23.6",
        "generated_on": "2026-08-31",
        "status": "PASS_LOCAL_VERIFIED_WORKFLOW_SUPERVISOR",
        "commands": [" ".join(command) for command in COMMANDS],
        "artifacts": [artifact(path) for path in SOURCES]
        + [artifact(RAW_PATH.relative_to(ROOT).as_posix())],
        "product_truth": {
            "closed_workflow_and_policy_admission": True,
            "dependency_ready_selection_only": True,
            "common_coordinator_driver_only": True,
            "second_model_or_tool_loop": False,
            "three_dependent_artifact_steps": True,
            "verifier_owned_success_only": True,
            "fresh_retry_identity_enforcement": True,
            "uncertain_effect_replay": False,
            "safe_checkpoint_no_replay": True,
            "client_or_model_completion_authority": False,
            "presentation_changes_canonical_result": False,
            "qualified_production_model_complete": False,
            "independent_review_complete": False,
            "windows_validation_complete": False,
            "macos_validation_complete": False,
            "release_claim": "none",
        },
        "retained_fixture_markers": [
            "STORY_23_6_SUCCESS=",
            "STORY_23_6_RETRY=",
            "STORY_23_6_TERMINALS=",
            "STORY_23_6_RESTART=",
        ],
        "remaining_external_work": [
            "repeat the final candidate with an independently qualified production model",
            "run final local Windows validation against the exact final candidate commit",
            "run deferred macOS validation against the exact final candidate commit",
            "obtain independent security and release review",
        ],
    }


def validate_report(value: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict):
        return ["Story 23.6 report is not an object"]
    truth = value.get("product_truth", {})
    required_true = (
        "closed_workflow_and_policy_admission",
        "dependency_ready_selection_only",
        "common_coordinator_driver_only",
        "three_dependent_artifact_steps",
        "verifier_owned_success_only",
        "fresh_retry_identity_enforcement",
        "safe_checkpoint_no_replay",
    )
    required_false = (
        "second_model_or_tool_loop",
        "uncertain_effect_replay",
        "client_or_model_completion_authority",
        "presentation_changes_canonical_result",
        "qualified_production_model_complete",
        "independent_review_complete",
        "windows_validation_complete",
        "macos_validation_complete",
    )
    failures.extend(
        f"{field} must remain true" for field in required_true if truth.get(field) is not True
    )
    failures.extend(
        f"{field} must remain false" for field in required_false if truth.get(field) is not False
    )
    if truth.get("release_claim") != "none":
        failures.append("Story 23.6 cannot claim a release")
    return failures


def validate() -> list[str]:
    failures = [f"missing source: {path}" for path in SOURCES if not (ROOT / path).is_file()]
    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return failures + [f"cannot read retained Story 23.6 evidence: {error}"]
    for index in range(len(COMMANDS)):
        if f"COMMAND_{index}_EXIT=0" not in raw:
            failures.append(f"command {index} did not retain a passing exit")
    for marker in expected_report()["retained_fixture_markers"]:
        if marker not in raw:
            failures.append(f"raw evidence lacks fixture marker: {marker}")
    for prohibited in ("test result: FAILED", "not ok", "BEGIN PRIVATE KEY"):
        if prohibited in raw:
            failures.append(f"raw evidence contains prohibited marker: {prohibited}")
    failures.extend(validate_report(report))
    if report != expected_report():
        failures.append("Story 23.6 report is stale or widened")
    return failures


def capture() -> int:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    # Keep documentation-link validation truthful during first capture; final source-bound
    # contents replace these non-passing placeholders only after every command succeeds.
    RAW_PATH.write_text("Story 23.6 evidence capture in progress\n", encoding="utf-8")
    REPORT_PATH.write_text("{}\n", encoding="utf-8")
    records: list[str] = []
    environment = os.environ.copy()
    environment["AGENTMAGE_STORY_23_6_EVIDENCE"] = "1"
    for index, command in enumerate(COMMANDS):
        result = subprocess.run(
            command,
            cwd=ROOT,
            env=environment,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
        )
        records.append(
            f"$ {' '.join(command)}\n{result.stdout}\nCOMMAND_{index}_EXIT={result.returncode}"
        )
        if result.returncode != 0:
            RAW_PATH.write_text("\n\n".join(records) + "\n", encoding="utf-8")
            return result.returncode
    RAW_PATH.write_text("\n\n".join(records) + "\n", encoding="utf-8")
    REPORT_PATH.write_text(
        json.dumps(expected_report(), indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
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
    print("Story 23.6 local verified-workflow-supervisor evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
