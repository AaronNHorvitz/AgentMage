#!/usr/bin/env python3
"""Build and verify the Story 2.3.3.3 versioned golden gate."""

from __future__ import annotations

import argparse
import copy
import json
import sys
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.engineering_artifact_admission_fixtures import (  # noqa: E402
    ZERO_SHA256,
    canonical_json,
    sealed,
    sha256_bytes,
    write_atomic,
)


FIXTURE_DIR: Final = ROOT / "fixtures" / "artifact-evaluation" / "v1"
TEXT_PATH: Final = FIXTURE_DIR / "text-reference-manifest.json"
DOCUMENT_PATH: Final = FIXTURE_DIR / "document-variant-manifest.json"
LIFECYCLE_PATH: Final = FIXTURE_DIR / "lifecycle-scenarios.json"
PLAN_PATH: Final = FIXTURE_DIR / "workflow-plan-fixtures.json"
CRASH_PATH: Final = FIXTURE_DIR / "workflow-crash-points.json"
TERMINAL_PATH: Final = FIXTURE_DIR / "workflow-terminal-outcomes.json"
ARTIFACT_METRIC_PATH: Final = FIXTURE_DIR / "artifact-golden-metrics.json"
WORKFLOW_METRIC_PATH: Final = FIXTURE_DIR / "workflow-golden-metrics.json"
OUTPUT_PATH: Final = FIXTURE_DIR / "golden-manifest-v1.json"
INPUT_PATHS: Final = (
    TEXT_PATH,
    DOCUMENT_PATH,
    LIFECYCLE_PATH,
    PLAN_PATH,
    CRASH_PATH,
    TERMINAL_PATH,
    ARTIFACT_METRIC_PATH,
    WORKFLOW_METRIC_PATH,
)
FORBIDDEN_RAW_KEYS: Final = ("raw_secret", "credential_value", "private_key_material", "unredacted_password")
SIDE_EFFECT_CONTRACT: Final = {
    "active_content_executions": 0,
    "approval_bypasses": 0,
    "authority_minted": False,
    "duplicate_effects": 0,
    "external_relationship_fetches": 0,
    "network_calls": 0,
    "product_effects_executed": 0,
}


def load(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def outcome(fixture_id: str, family: str, state: str, success_terminal: bool) -> dict[str, Any]:
    return {
        "fixture_id": fixture_id,
        "family": family,
        "expected_terminal_state": state,
        "success_terminal": success_terminal,
    }


def build_outcome_ledger() -> list[dict[str, Any]]:
    text = load(TEXT_PATH)
    documents = load(DOCUMENT_PATH)
    lifecycle = load(LIFECYCLE_PATH)
    plans = load(PLAN_PATH)
    crashes = load(CRASH_PATH)
    terminals = load(TERMINAL_PATH)
    ledger: list[dict[str, Any]] = []
    ledger.extend(outcome(item["fixture_id"], "text_reference", item["expected_disposition"], item["expected_disposition"] == "captured") for item in text["cases"])
    ledger.extend(outcome(item["fixture_id"], "document_variant", item["expected_disposition"], item["expected_disposition"] == "captured") for item in documents["cases"])
    ledger.extend(outcome(item["scenario_id"], "artifact_lifecycle", item["expected"]["terminal_state"], item["expected"]["terminal_state"] == "captured") for item in lifecycle["scenarios"])
    ledger.extend(outcome(item["fixture_id"], "workflow_plan", item["expected"]["terminal_state"], item["expected"]["terminal_state"] == "verified_success") for item in plans["fixtures"])
    ledger.extend(outcome(item["fixture_id"], "workflow_crash", item["expected"]["recovery_state"], item["expected"]["terminal_event_durable"] is True) for item in crashes["fixtures"])
    ledger.extend(outcome(item["fixture_id"], "workflow_terminal", item["expected"]["terminal_result"], False) for item in terminals["fixtures"])
    return ledger


def build_manifest() -> dict[str, Any]:
    ledger = build_outcome_ledger()
    value = {
        "schema_version": 1,
        "golden_manifest_version": "1.0.0",
        "manifest_id": "artifact-evaluation-golden-gate-v1",
        "task_id": "2.3.3.3",
        "generated_on": "2026-08-30",
        "status": "golden_integrity_gate_pass",
        "synthetic_only": True,
        "product_gate_claim": "none",
        "product_runtime_executed": False,
        "effect_executed": False,
        "required_inputs": [
            {
                "path": str(path.relative_to(ROOT)),
                "sha256": sha256_bytes(path.read_bytes()),
                "required": True,
            }
            for path in INPUT_PATHS
        ],
        "metric_manifest_versions": {
            "artifact": load(ARTIFACT_METRIC_PATH)["golden_manifest_version"],
            "workflow": load(WORKFLOW_METRIC_PATH)["golden_manifest_version"],
        },
        "outcome_count": len(ledger),
        "success_terminal_count": sum(item["success_terminal"] for item in ledger),
        "non_success_terminal_count": sum(not item["success_terminal"] for item in ledger),
        "outcomes": ledger,
        "expected_side_effects": SIDE_EFFECT_CONTRACT,
        "gate_policy": {
            "all_required_inputs_present": True,
            "dependency_hash_match_required": True,
            "every_outcome_visible": True,
            "non_success_relabel_as_success_allowed": False,
            "raw_secret_fields_allowed": False,
            "expected_side_effect_drift_allowed": False,
        },
        "generator": {"path": "scripts/artifact_evaluation_golden_gate.py", "sha256": sha256_bytes(Path(__file__).read_bytes())},
        "manifest_sha256": ZERO_SHA256,
    }
    return sealed(value, "manifest_sha256")


def valid_hash(record: dict[str, Any], field: str) -> bool:
    unhashed = copy.deepcopy(record)
    recorded = unhashed.get(field)
    unhashed[field] = ZERO_SHA256
    return recorded == sha256_bytes(canonical_json(unhashed))


def raw_key_paths(value: Any, path: str = "$") -> list[str]:
    findings: list[str] = []
    if isinstance(value, dict):
        for key, child in value.items():
            child_path = f"{path}.{key}"
            if key != "raw_secret_fields_allowed" and any(fragment in key.lower() for fragment in FORBIDDEN_RAW_KEYS):
                findings.append(child_path)
            findings.extend(raw_key_paths(child, child_path))
    elif isinstance(value, list):
        for index, child in enumerate(value):
            findings.extend(raw_key_paths(child, f"{path}[{index}]"))
    return findings


def validate_manifest(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["golden manifest must be an object"]
    failures: list[str] = []
    if not valid_hash(value, "manifest_sha256"):
        failures.append("golden manifest self-hash is invalid")
    expected_inputs = [str(path.relative_to(ROOT)) for path in INPUT_PATHS]
    inputs = value.get("required_inputs", [])
    if not isinstance(inputs, list) or [item.get("path") for item in inputs if isinstance(item, dict)] != expected_inputs:
        failures.append("golden manifest has a missing, extra, or reordered required input")
    else:
        for item, path in zip(inputs, INPUT_PATHS, strict=True):
            if item.get("required") is not True or not path.is_file() or item.get("sha256") != sha256_bytes(path.read_bytes()):
                failures.append(f"golden required input is absent or hash-mismatched: {item.get('path')}")
    if value.get("golden_manifest_version") != "1.0.0" or value.get("metric_manifest_versions") != {"artifact": "1.0.0", "workflow": "1.0.0"}:
        failures.append("golden manifest or metric manifest version is unsupported")
    expected_ledger = build_outcome_ledger()
    if value.get("outcomes") != expected_ledger:
        failures.append("golden manifest hides, changes, reorders, or falsely passes an expected outcome")
    if value.get("outcome_count") != len(expected_ledger):
        failures.append("golden outcome count is incomplete")
    if value.get("success_terminal_count") != sum(item["success_terminal"] for item in expected_ledger):
        failures.append("golden success-terminal count is false")
    if value.get("non_success_terminal_count") != sum(not item["success_terminal"] for item in expected_ledger):
        failures.append("golden non-success-terminal count is hidden or false")
    if value.get("expected_side_effects") != SIDE_EFFECT_CONTRACT:
        failures.append("golden expected side effects changed")
    if raw_key_paths(value):
        failures.append("golden manifest contains a forbidden raw-secret field")
    expected_policy = {
        "all_required_inputs_present": True,
        "dependency_hash_match_required": True,
        "every_outcome_visible": True,
        "non_success_relabel_as_success_allowed": False,
        "raw_secret_fields_allowed": False,
        "expected_side_effect_drift_allowed": False,
    }
    if value.get("gate_policy") != expected_policy:
        failures.append("golden gate policy is incomplete or widened")
    if value.get("status") != "golden_integrity_gate_pass" or value.get("synthetic_only") is not True or value.get("product_gate_claim") != "none" or value.get("product_runtime_executed") is not False or value.get("effect_executed") is not False:
        failures.append("golden manifest makes a product, runtime, or effect overclaim")
    return failures


def check() -> list[str]:
    try:
        actual = load(OUTPUT_PATH)
        expected = build_manifest()
    except (OSError, ValueError, json.JSONDecodeError) as error:
        return [f"cannot validate versioned golden manifest: {error}"]
    failures = validate_manifest(actual)
    if actual != expected:
        failures.append("checked golden manifest is stale, incomplete, or widened")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    if args.write:
        write_atomic(OUTPUT_PATH, canonical_json(build_manifest()))
    failures = check()
    if failures:
        for failure in failures:
            print(f"Versioned golden gate validation failed: {failure}", file=sys.stderr)
        return 1
    print(f"Validated versioned golden gate with {len(build_outcome_ledger())} visible outcomes")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
