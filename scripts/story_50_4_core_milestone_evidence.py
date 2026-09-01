#!/usr/bin/env python3
"""Generate and validate the local M-FOUNDATIONAL-RUNTIME-CORE decision."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-50/story-50.4-core-milestone"
RAW_PATH: Final = EVIDENCE_DIR / "local-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "report.json"
COMMANDS: Final = (
    ("python3", "scripts/story_22_5_vertical_slice_evidence.py"),
    ("python3", "scripts/engineering_artifact_rv51_evidence.py"),
    ("python3", "scripts/story_16_4_tool_observation_evidence.py"),
    ("python3", "scripts/workflow_rv52_evidence.py"),
    ("python3", "scripts/story_13_5_gateway_identity_evidence.py"),
    ("python3", "scripts/story_13_6_gateway_routing_evidence.py"),
    ("python3", "scripts/story_23_7_verified_chat_evidence.py"),
    ("python3", "scripts/story_23_8_native_chat_compatibility_evidence.py"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "capability_registry::tests", "--all-features", "--locked"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "multi_agent::tests", "--all-features", "--locked"),
    ("npm", "run", "engineering-runtime:schemas:check"),
    ("python3", "scripts/requirement_coverage.py"),
    ("python3", "scripts/task_graph.py"),
)
SOURCES: Final = (
    "SECURITY-REVIEW.md", "ENGINEERING-RUNTIME.md", "MODEL-GATEWAY.md",
    "kernel/engine/src/dependency_degradation.rs", "kernel/engine/src/capability_registry.rs",
    "kernel/engine/src/multi_agent.rs", "shells/host/src/runtime_read_tests.rs",
    "artifacts/sprints/sprint-50/story-50.3-foundational-runtime/report.json",
    "artifacts/sprints/sprint-50/story-50.4-dependency-degradation/report.json",
    "scripts/story_50_4_core_milestone_evidence.py",
    "tests/test_story_50_4_core_milestone_evidence.py",
)


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def artifact(relative: str) -> dict[str, Any]:
    path = ROOT / relative
    return {"path": relative, "byte_length": path.stat().st_size, "sha256": digest(path)}


def expected_report() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "agentmage-foundational-runtime-core-decision",
        "milestone_id": "M-FOUNDATIONAL-RUNTIME-CORE",
        "generated_on": "2026-08-31",
        "status": "PASS_LOCAL_TEXT_LOG_WORKFLOW_CORE",
        "gate_closed": True,
        "commands": [" ".join(command) for command in COMMANDS],
        "protocols": [
            {"id": "RV-50", "status": "PASS_LOCAL_FAKE_RUNTIME"},
            {"id": "RV-51", "status": "PASS_LOCAL_TEXT_LOG_ONLY"},
            {"id": "RV-52", "status": "PASS_LOCAL_TOOL_LINEAGE"},
            {"id": "RV-53", "status": "PASS_LOCAL_CONTRACT_NO_QUALIFIED_TUPLE"},
            {"id": "RV-54", "status": "PASS_STRICT_LOCAL_ONLY_REMOTE_BLOCKED"},
            {"id": "RV-55", "status": "PASS_LOCAL_SOURCE_INSTALLED_BLOCKED"},
            {"id": "RV-56", "status": "PARTIAL_LATER_CAPABILITY_STORY"},
            {"id": "RV-57", "status": "PARTIAL_LATER_MULTI_AGENT_STORY"},
        ],
        "artifacts": [artifact(path) for path in SOURCES] + [artifact(RAW_PATH.relative_to(ROOT).as_posix())],
        "product_truth": {
            "text_log_ingestion_and_context_pass": True,
            "verified_workflow_supervision_pass": True,
            "dependency_degradation_and_reliability_pass": True,
            "schema_traceability_and_task_graph_pass": True,
            "structured_parser_milestone_complete": False,
            "capability_registry_protocol_complete": False,
            "multi_agent_protocol_complete": False,
            "qualified_model_endpoint_or_route": False,
            "remote_endpoint_protocol_complete": False,
            "installed_client_protocol_complete": False,
            "independent_review_complete": False,
            "windows_validation_complete": False,
            "macos_validation_complete": False,
            "complete_foundational_runtime_milestone": False,
            "release_claim": "none",
        },
        "exact_blockers": [
            "Story 49.2 alternate live runtime/codec qualification is unavailable",
            "DOCX, PDF/OCR, and XLSX parser gates belong to Stories 58.2, 60.2, and 62.2",
            "RV-56 and RV-57 remain owned by later capability and multi-agent stories",
            "installed-client, admitted-model, independent-review, Windows-final, and deferred macOS evidence is absent",
        ],
    }


def validate_report(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["core milestone report is not an object"]
    truth = value.get("product_truth", {})
    required_true = (
        "text_log_ingestion_and_context_pass", "verified_workflow_supervision_pass",
        "dependency_degradation_and_reliability_pass", "schema_traceability_and_task_graph_pass",
    )
    required_false = (
        "structured_parser_milestone_complete", "capability_registry_protocol_complete",
        "multi_agent_protocol_complete", "qualified_model_endpoint_or_route",
        "remote_endpoint_protocol_complete", "installed_client_protocol_complete",
        "independent_review_complete", "windows_validation_complete", "macos_validation_complete",
        "complete_foundational_runtime_milestone",
    )
    failures = [f"{field} must remain true" for field in required_true if truth.get(field) is not True]
    failures.extend(f"{field} must remain false" for field in required_false if truth.get(field) is not False)
    if value.get("gate_closed") is not True or value.get("status") != "PASS_LOCAL_TEXT_LOG_WORKFLOW_CORE":
        failures.append("local core gate decision is invalid")
    if truth.get("release_claim") != "none":
        failures.append("core milestone cannot claim a release")
    statuses = {item.get("id"): item.get("status") for item in value.get("protocols", []) if isinstance(item, dict)}
    if statuses.get("RV-56", "").startswith("PASS") or statuses.get("RV-57", "").startswith("PASS"):
        failures.append("later RV-56/RV-57 protocols were overclaimed")
    return failures


def validate() -> list[str]:
    failures = [f"missing source: {path}" for path in SOURCES if not (ROOT / path).is_file()]
    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return failures + [f"cannot read retained core milestone evidence: {error}"]
    failures.extend(f"command {index} did not retain a passing exit" for index in range(len(COMMANDS)) if f"COMMAND_{index}_EXIT=0" not in raw)
    for marker in (
        "Story 22.5 local Engineering Runtime vertical-slice evidence validated", "RV-51 evidence validated",
        "RV-52", "Story 13.6 gateway codec", "Story 23.7 local durable Verified Chat",
        "repository_review_executes_from_the_registered_manifest ... ok",
        "three_workers_run_concurrently_and_integrate_serially ... ok",
    ):
        if marker not in raw:
            failures.append(f"raw evidence lacks marker: {marker}")
    failures.extend(validate_report(report))
    if report != expected_report():
        failures.append("core milestone report is stale or widened")
    return failures


def capture() -> int:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    RAW_PATH.write_text("Core milestone capture in progress\n", encoding="utf-8")
    REPORT_PATH.write_text("{}\n", encoding="utf-8")
    records: list[str] = []
    for index, command in enumerate(COMMANDS):
        result = subprocess.run(command, cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=False)
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
    print("M-FOUNDATIONAL-RUNTIME-CORE local evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
