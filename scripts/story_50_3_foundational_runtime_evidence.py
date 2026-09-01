#!/usr/bin/env python3
"""Generate and validate local Story 50.3 foundational-runtime evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-50/story-50.3-foundational-runtime"
RAW_PATH: Final = EVIDENCE_DIR / "local-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "report.json"
COMMANDS: Final = (
    (
        "cargo", "test", "-p", "agentmage-host",
        "story_50_3_artifact_heavy_steps_use_one_supervisor_and_common_coordinator",
        "--all-features", "--locked",
    ),
    (
        "cargo", "test", "-p", "agentmage-kernel-engine",
        "verified_workflow_supervisor::tests", "--all-features", "--locked",
    ),
    (
        "cargo", "test", "-p", "agentmage-kernel-engine",
        "source_preparation::tests", "--all-features", "--locked",
    ),
    (
        "python3", "-c",
        "import json; from pathlib import Path; from scripts import runtime_hardening_load as c; "
        "a=c.ROOT/Path('artifacts/sprints/sprint-50/story-50.2-runtime-load-worker'); "
        "p=json.loads((c.ROOT/c.PROFILE).read_text()); r=json.loads((a/'report.json').read_text()); "
        "f=c.report_failures(r,p,a); assert not f,f; print('Retained runtime hardening load evidence validated')",
    ),
    ("python3", "scripts/runtime_component_removal.py", "--verify"),
    ("python3", "scripts/story_23_6_workflow_supervisor_evidence.py"),
    ("python3", "scripts/artifact_evaluation_text_log_fixtures.py"),
    ("cargo", "test", "-p", "agentmage-host", "feature_activation", "--all-features", "--locked"),
    ("cargo", "test", "-p", "agentmage-host", "story_50_3_disabled_artifact_and_retrieval_features_register_no_tools", "--all-features", "--locked"),
    ("npm", "--prefix", "shells/vscode", "test"),
    ("python3", "scripts/runtime_feature_activation.py"),
)
SOURCES: Final = (
    "shells/host/src/runtime_read_tests.rs",
    "shells/host/src/workflow_supervisor.rs",
    "kernel/engine/src/source_preparation.rs",
    "kernel/engine/src/verified_workflow_supervisor.rs",
    "fixtures/runtime-hardening/v1/linux-reference-load-profile.json",
    "artifacts/sprints/sprint-50/story-50.2-runtime-load-worker/report.json",
    "artifacts/sprints/sprint-50/story-50.2-component-removal/report.json",
    "scripts/story_50_3_foundational_runtime_evidence.py",
    "tests/test_story_50_3_foundational_runtime_evidence.py",
    "architecture/runtime-feature-activation.json",
    "shells/host/src/feature_activation.rs",
    "shells/host/src/runtime_tools.rs",
    "shells/vscode/src/feature_activation.ts",
    "shells/vscode/src/extension.ts",
    "shells/vscode/package.json",
    "shells/vscode/test/feature_activation.test.ts",
    "scripts/runtime_feature_activation.py",
    "tests/test_runtime_feature_activation.py",
)


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def artifact(relative: str) -> dict[str, Any]:
    path = ROOT / relative
    return {"path": relative, "byte_length": path.stat().st_size, "sha256": digest(path)}


def expected_report() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "agentmage-story-50-3-foundational-runtime-evidence",
        "story_id": "50.3",
        "generated_on": "2026-08-31",
        "status": "PASS_LOCAL_TEXT_LOG_WORKFLOW_CORE",
        "commands": [" ".join(command) for command in COMMANDS],
        "artifacts": [artifact(path) for path in SOURCES]
        + [artifact(RAW_PATH.relative_to(ROOT).as_posix())],
        "product_truth": {
            "prepared_text_log_ingress": True,
            "three_dependency_ready_artifact_steps": True,
            "one_verified_workflow_supervisor": True,
            "one_common_runtime_coordinator_per_attempt": True,
            "current_source_context_and_native_artifact_receipts": True,
            "verifier_owned_completion": True,
            "fresh_attempt_run_tool_grant_and_receipt_identities": True,
            "bounded_pressure_campaign_retained": True,
            "fault_restart_and_no_replay_campaign_retained": True,
            "component_removal_campaign_retained": True,
            "independent_feature_activation_complete": True,
            "false_completion_or_unsafe_retry": False,
            "structured_document_parser_qualification": False,
            "qualified_production_model_evaluation": False,
            "installed_client_campaign_complete": False,
            "independent_review_complete": False,
            "windows_validation_complete": False,
            "macos_validation_complete": False,
            "complete_foundational_runtime_claim": False,
            "release_claim": "none",
        },
        "remaining_external_or_later_work": [
            "evaluate one independently qualified production model tuple when one is admitted",
            "run installed native Chat and CLI workflows on supported platform candidates",
            "qualify later DOCX, PDF/OCR, and XLSX parser adapters in their owning sprints",
            "obtain independent runtime, security, and release review",
            "run final local Windows validation against the exact final candidate commit",
            "run deferred macOS validation against the exact final candidate commit",
        ],
    }


def validate_report(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["Story 50.3 report is not an object"]
    truth = value.get("product_truth", {})
    required_true = (
        "prepared_text_log_ingress",
        "three_dependency_ready_artifact_steps",
        "one_verified_workflow_supervisor",
        "one_common_runtime_coordinator_per_attempt",
        "current_source_context_and_native_artifact_receipts",
        "verifier_owned_completion",
        "fresh_attempt_run_tool_grant_and_receipt_identities",
        "bounded_pressure_campaign_retained",
        "fault_restart_and_no_replay_campaign_retained",
        "component_removal_campaign_retained",
        "independent_feature_activation_complete",
    )
    required_false = (
        "false_completion_or_unsafe_retry",
        "structured_document_parser_qualification",
        "qualified_production_model_evaluation",
        "installed_client_campaign_complete",
        "independent_review_complete",
        "windows_validation_complete",
        "macos_validation_complete",
        "complete_foundational_runtime_claim",
    )
    failures = [f"{field} must remain true" for field in required_true if truth.get(field) is not True]
    failures.extend(
        f"{field} must remain false" for field in required_false if truth.get(field) is not False
    )
    if truth.get("release_claim") != "none":
        failures.append("Story 50.3 cannot claim a release")
    return failures


def validate() -> list[str]:
    failures = [f"missing source: {path}" for path in SOURCES if not (ROOT / path).is_file()]
    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return failures + [f"cannot read retained Story 50.3 evidence: {error}"]
    failures.extend(
        f"command {index} did not retain a passing exit"
        for index in range(len(COMMANDS))
        if f"COMMAND_{index}_EXIT=0" not in raw
    )
    for marker in (
        "story_50_3_artifact_heavy_steps_use_one_supervisor_and_common_coordinator ... ok",
        "three_dependency_steps_complete_in_definition_order ... ok",
        "Story 23.6 local verified-workflow-supervisor evidence validated",
        '"campaign_id": "story-50.2-component-removal-v1"',
        "Retained runtime hardening load evidence validated",
        "current_activation_is_closed_and_unavailable_features_are_off ... ok",
        "story_50_3_disabled_artifact_and_retrieval_features_register_no_tools ... ok",
        "compatibility registrations require independent exact true flags",
        "Validated 13 independent runtime feature activations",
    ):
        if marker not in raw:
            failures.append(f"raw evidence lacks fixture marker: {marker}")
    for prohibited in ("test result: FAILED", "not ok", "BEGIN PRIVATE KEY"):
        if prohibited in raw:
            failures.append(f"raw evidence contains prohibited marker: {prohibited}")
    failures.extend(validate_report(report))
    if report != expected_report():
        failures.append("Story 50.3 report is stale or widened")
    return failures


def capture() -> int:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    RAW_PATH.write_text("Story 50.3 evidence capture in progress\n", encoding="utf-8")
    REPORT_PATH.write_text("{}\n", encoding="utf-8")
    records: list[str] = []
    for index, command in enumerate(COMMANDS):
        result = subprocess.run(
            command, cwd=ROOT, text=True, stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT, check=False,
        )
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
    print("Story 50.3 local text/log workflow-core evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
