#!/usr/bin/env python3
"""Generate and validate source-bound local Story 22.3 evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-22/story-22.3"
RAW_PATH: Final = EVIDENCE_DIR / "source-preparation-results.log"
GOLDEN_PATH: Final = EVIDENCE_DIR / "source-preparation-goldens.json"
REPORT_PATH: Final = EVIDENCE_DIR / "source-preparation-report.json"
GENERATOR: Final = (
    "cargo", "run", "-p", "agentmage-host", "--example", "story_22_3_evidence", "--locked",
)
COMMANDS: Final = (
    ("cargo", "test", "-p", "agentmage-kernel-engine", "source_preparation", "--lib", "--locked"),
    ("cargo", "test", "-p", "agentmage-capability-read-only", "artifact::tests", "--locked"),
    ("cargo", "test", "-p", "agentmage-host", "source_artifact_runtime", "--locked"),
    ("node", "--test", "tests/test_source_preparation_schemas.mjs"),
    (
        "cargo", "clippy", "-p", "agentmage-kernel-engine", "-p",
        "agentmage-capability-read-only", "-p", "agentmage-host", "--all-targets",
        "--all-features", "--locked", "--", "-D", "warnings",
    ),
)
MARKERS: Final = (
    "story_22_3_twenty_five_mib_log_is_bounded_and_visibly_truncated ... ok",
    "story_22_3_persisted_restart_release_and_delete_are_exact_and_fail_closed ... ok",
    "story_22_3_process_stop_publishes_no_partial_source_and_exact_restart_recovers ... ok",
    "story_22_3_context_accounts_every_source_section_duplicate_restriction_and_budget ... ok",
    "all_seven_fake_operations_are_deterministic_typed_and_receipted ... ok",
    "story_22_3_all_native_text_log_tools_use_one_production_dispatcher ... ok",
    "prepared source and context schemas accept closed canonical records",
    "prepared source schemas reject paths, unknown fields, widening, and missing accounting",
    "Finished `dev` profile",
)
SOURCES: Final = (
    "kernel/engine/src/source_preparation.rs",
    "kernel/engine/src/model_orchestration_profile.rs",
    "capabilities/read-only/src/artifact.rs",
    "shells/host/src/source_artifact_runtime.rs",
    "shells/host/examples/story_22_3_evidence.rs",
    "schemas/runtime/prepared-source-manifest.schema.json",
    "schemas/runtime/prepared-source-context-manifest.schema.json",
    "tests/test_source_preparation_schemas.mjs",
    "docs/architecture/source-artifact-preparation-and-context.md",
    "scripts/story_22_3_source_preparation_evidence.py",
    "tests/test_story_22_3_source_preparation_evidence.py",
)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def artifact(relative: str) -> dict[str, Any]:
    path = ROOT / relative
    return {"path": relative, "byte_length": path.stat().st_size, "sha256": sha256(path)}


def validate_golden(value: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict) or value.get("record_type") != "agentmage-story-22-3-source-preparation-goldens":
        return ["golden record type is invalid"]
    performance = value.get("performance", {})
    if performance.get("input_bytes") != 25 * 1024 * 1024:
        failures.append("25 MiB source boundary was not retained")
    if not 0 <= performance.get("elapsed_ms", -1) < performance.get("elapsed_ceiling_ms", 0):
        failures.append("25 MiB performance ceiling failed")
    if performance.get("ingest_bytes_per_second", 0) < performance.get(
        "minimum_ingest_bytes_per_second", 1
    ):
        failures.append("25 MiB ingest throughput floor failed")
    for measured, ceiling in (
        ("initial_ingest_latency_us", "initial_ingest_latency_ceiling_us"),
        ("time_to_first_useful_section_us", "first_useful_section_latency_ceiling_us"),
        ("cleanup_latency_us", "cleanup_latency_ceiling_us"),
    ):
        if not 0 <= performance.get(measured, -1) < performance.get(ceiling, 0):
            failures.append(f"performance ceiling failed: {measured}")
    if not 0 < performance.get("used_tokens", 0) <= performance.get("allocated_tokens", 0):
        failures.append("token allocation was not exact and bounded")
    results = value.get("native_tool_results", [])
    expected = {"artifact.list", "artifact.metadata", "artifact.sections", "artifact.search", "artifact.get_log_errors"}
    if {item.get("tool") for item in results} != expected:
        failures.append("native result golden catalog is incomplete")
    if any(not item.get("production_execution") for item in results):
        failures.append("a native result did not identify production execution")
    encoded = json.dumps(value, sort_keys=True)
    for prohibited in ("/home/", "/var/home/", "file://", "BEGIN PRIVATE KEY"):
        if prohibited in encoded:
            failures.append(f"golden contains prohibited canary: {prohibited}")
    if value.get("canary_scan") != {
        "raw_path_count": 0, "network_access_count": 0, "workspace_mutation_count": 0,
    }:
        failures.append("canary scan is not closed")
    return failures


def expected_report() -> dict[str, Any]:
    golden = json.loads(GOLDEN_PATH.read_text(encoding="utf-8"))
    performance = golden["performance"]
    return {
        "schema_version": 1,
        "record_type": "agentmage-story-22-3-source-preparation-evidence",
        "story_id": "22.3",
        "generated_on": "2026-08-31",
        "status": "PASS_LOCAL_PRODUCTION_TEXT_LOG",
        "protocols": ["RV-08", "RV-16", "RV-17", "RV-18"],
        "commands": [" ".join(GENERATOR)] + [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "performance": performance,
        "artifacts": [artifact(path) for path in SOURCES]
        + [artifact(RAW_PATH.relative_to(ROOT).as_posix()), artifact(GOLDEN_PATH.relative_to(ROOT).as_posix())],
        "product_truth": {
            "bounded_text_log_admission_complete": True,
            "complete_source_section_accounting_complete": True,
            "exact_tokenizer_and_window_reconciliation_complete": True,
            "policy_persisted_restart_complete": True,
            "native_artifact_operations_complete": True,
            "registered_operation_count": 7,
            "page_sheet_operation_count": 0,
            "workspace_mutation_count": 0,
            "network_access_count": 0,
            "currently_admitted_profile_count": 0,
            "installed_package_campaign_complete": False,
            "windows_campaign_complete": False,
            "physical_fault_campaign_complete": False,
            "independent_review_complete": False,
            "release_claim": "none",
        },
        "remaining_external_work": [
            "repeat the corpus for the first independently admitted real model profile",
            "run installed-package and Windows source-artifact campaigns",
            "retain physical storage-fault and independent human security review evidence",
        ],
    }


def validate() -> list[str]:
    failures = [f"missing source: {path}" for path in SOURCES if not (ROOT / path).is_file()]
    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        golden = json.loads(GOLDEN_PATH.read_text(encoding="utf-8"))
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return failures + [f"cannot read retained Story 22.3 evidence: {error}"]
    failures.extend(f"raw evidence missing marker: {marker}" for marker in MARKERS if marker not in raw)
    for prohibited in ("test result: FAILED", "error: could not compile", "not ok"):
        if prohibited in raw:
            failures.append(f"raw evidence contains prohibited marker: {prohibited}")
    failures += validate_golden(golden)
    if report != expected_report():
        failures.append("Story 22.3 report is stale or widened")
    return failures


def capture() -> int:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    generated = subprocess.run(GENERATOR, cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
    records = [f"$ {' '.join(GENERATOR)}\n{generated.stderr}"]
    if generated.returncode != 0:
        RAW_PATH.write_text("\n".join(records), encoding="utf-8")
        return generated.returncode
    try:
        golden = json.loads(generated.stdout)
    except json.JSONDecodeError:
        RAW_PATH.write_text("\n".join(records), encoding="utf-8")
        return 1
    GOLDEN_PATH.write_text(json.dumps(golden, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    for command in COMMANDS:
        result = subprocess.run(command, cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=False)
        records.append(f"$ {' '.join(command)}\n{result.stdout}")
        if result.returncode != 0:
            RAW_PATH.write_text("\n".join(records), encoding="utf-8")
            return result.returncode
    RAW_PATH.write_text("\n".join(records), encoding="utf-8")
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
    print("Story 22.3 local source preparation evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
