#!/usr/bin/env python3
"""Build and verify Story 2.3.1.3 artifact lifecycle scenarios."""

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
ACCOUNTING_PATH: Final = ROOT / "fixtures" / "artifact-admission" / "v1" / "context-accounting-manifests.json"
OUTPUT_PATH: Final = FIXTURE_DIR / "lifecycle-scenarios.json"
CATEGORIES: Final = (
    "combined_artifact_token_overflow",
    "model_profile_change",
    "cancellation",
    "crash",
    "restart",
    "cache_stale",
    "refresh",
    "retention",
    "deletion",
)
PROHIBITED_EFFECTS: Final = {
    "network_calls": 0,
    "active_content_executions": 0,
    "external_relationship_fetches": 0,
    "undeclared_persistent_writes": 0,
    "source_mutations": 0,
    "false_completions": 0,
}


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def source_index() -> dict[str, dict[str, Any]]:
    index: dict[str, dict[str, Any]] = {}
    for manifest_path in (TEXT_PATH, DOCUMENT_PATH):
        manifest = read_json(manifest_path)
        for case in manifest["cases"]:
            byte_identity = case["byte_identity"]
            index[case["fixture_id"]] = {
                "fixture_id": case["fixture_id"],
                "category": case["category"],
                "sha256": None if byte_identity is None else byte_identity["sha256"],
                "byte_length": None if byte_identity is None else byte_identity["byte_length"],
                "manifest_path": str(manifest_path.relative_to(ROOT)),
                "manifest_sha256": sha256_bytes(manifest_path.read_bytes()),
            }
    return index


def context_binding(scenario_id: str) -> dict[str, Any]:
    accounting = read_json(ACCOUNTING_PATH)
    scenario = next(item for item in accounting["scenarios"] if item["scenario_id"] == scenario_id)
    manifest = scenario["context_manifest"]
    return {
        "scenario_id": scenario_id,
        "context_manifest_id": manifest["context_manifest_id"],
        "context_manifest_sha256": manifest["manifest_sha256"],
        "accounting_suite_sha256": sha256_bytes(ACCOUNTING_PATH.read_bytes()),
    }


def events(scenario_id: str, kinds: list[str]) -> list[dict[str, Any]]:
    result: list[dict[str, Any]] = []
    previous = ZERO_SHA256
    for sequence, kind in enumerate(kinds, start=1):
        event = sealed(
            {
                "scenario_id": scenario_id,
                "sequence": sequence,
                "event_kind": kind,
                "previous_event_sha256": previous,
                "event_sha256": ZERO_SHA256,
            },
            "event_sha256",
        )
        result.append(event)
        previous = event["event_sha256"]
    return result


def scenario(
    category: str,
    source_ids: list[str],
    event_kinds: list[str],
    expected: dict[str, Any],
    *,
    context: dict[str, Any] | None = None,
) -> dict[str, Any]:
    index = source_index()
    value = {
        "scenario_id": f"artifact-evaluation-{category}-v1",
        "category": category,
        "source_fixtures": [index[identity] for identity in source_ids],
        "events": events(f"artifact-evaluation-{category}-v1", event_kinds),
        "context_binding": context,
        "expected": expected,
        "prohibited_effects": PROHIBITED_EFFECTS,
        "scenario_sha256": ZERO_SHA256,
    }
    return sealed(value, "scenario_sha256")


def scenarios() -> list[dict[str, Any]]:
    common = ["eval-log-25mib-v1", "eval-pdf-digital-v1", "eval-docx-structured-v1", "eval-xlsx-structured-v1"]
    return [
        scenario(
            "combined_artifact_token_overflow",
            common,
            ["sources_admitted", "combined_budget_measured", "overflow_detected", "bounded_context_published"],
            {"terminal_state": "partial", "completion_allowed": False, "reason_code": "combined_context_overflow_visible", "all_sources_accounted": True},
            context=context_binding("token_budget_overflow"),
        ),
        scenario(
            "model_profile_change",
            common[:2],
            ["prior_profile_bound", "profile_changed", "prior_context_invalidated", "fresh_manifest_required"],
            {"terminal_state": "stale", "completion_allowed": False, "reason_code": "model_profile_binding_changed", "stale_context_reused": False},
            context=context_binding("model_profile_change"),
        ),
        scenario(
            "cancellation",
            ["eval-docx-structured-v1"],
            ["source_admitted", "extraction_started", "cancellation_requested", "owned_state_cleaned", "cancelled_terminal"],
            {"terminal_state": "cancelled", "completion_allowed": False, "reason_code": "user_cancellation", "derivative_published": False, "residue_count": 0},
        ),
        scenario(
            "crash",
            ["eval-pdf-mixed-v1"],
            ["source_admitted", "checkpoint_committed", "worker_crashed", "outcome_marked_uncertain"],
            {"terminal_state": "uncertain", "completion_allowed": False, "reason_code": "worker_crash_requires_reconciliation", "effect_replayed": False},
        ),
        scenario(
            "restart",
            ["eval-pdf-mixed-v1"],
            ["checkpoint_reopened", "source_identity_revalidated", "prior_attempt_reconciled", "fresh_attempt_admitted", "partial_result_rebuilt"],
            {"terminal_state": "partial", "completion_allowed": False, "reason_code": "restart_rebuilt_from_current_source", "effect_replayed": False, "source_revalidated": True},
        ),
        scenario(
            "cache_stale",
            ["eval-stale-v1"],
            ["cache_key_loaded", "current_source_observed", "source_hash_mismatch", "cache_entry_invalidated"],
            {"terminal_state": "stale", "completion_allowed": False, "reason_code": "cache_source_identity_changed", "cached_derivative_used": False},
        ),
        scenario(
            "refresh",
            ["eval-stale-v1"],
            ["stale_entry_invalidated", "current_source_captured", "fresh_derivative_created", "cache_key_rebound"],
            {"terminal_state": "captured", "completion_allowed": True, "reason_code": "fresh_source_rebound", "prior_derivative_used": False},
        ),
        scenario(
            "retention",
            ["eval-xlsx-structured-v1"],
            ["memory_only_source_active", "retention_policy_checked", "encrypted_persistence_not_authorized", "memory_source_expired", "owned_state_cleaned"],
            {"terminal_state": "expired", "completion_allowed": False, "reason_code": "memory_only_retention_expired", "persisted_payload_count": 0, "residue_count": 0},
        ),
        scenario(
            "deletion",
            ["eval-docx-structured-v1"],
            ["deletion_requested", "active_references_invalidated", "owned_payload_removed", "cache_entries_removed", "deletion_verified"],
            {"terminal_state": "deleted", "completion_allowed": False, "reason_code": "source_deleted", "remaining_reference_count": 0, "resurrection_allowed": False},
        ),
    ]


def build_suite() -> dict[str, Any]:
    value = {
        "schema_version": 1,
        "suite_id": "artifact-evaluation-lifecycle-scenarios-v1",
        "task_id": "2.3.1.3",
        "generated_on": "2026-08-30",
        "status": "synthetic-scenario-contract",
        "synthetic_only": True,
        "product_runtime_executed": False,
        "network_access": False,
        "required_categories": list(CATEGORIES),
        "scenario_count": len(CATEGORIES),
        "dependencies": [
            {"path": str(path.relative_to(ROOT)), "sha256": sha256_bytes(path.read_bytes())}
            for path in (TEXT_PATH, DOCUMENT_PATH, ACCOUNTING_PATH)
        ],
        "generator": {"path": "scripts/artifact_evaluation_lifecycle_scenarios.py", "sha256": sha256_bytes(Path(__file__).read_bytes())},
        "scenarios": scenarios(),
        "suite_sha256": ZERO_SHA256,
    }
    return sealed(value, "suite_sha256")


def valid_hash(record: dict[str, Any], field: str) -> bool:
    unhashed = copy.deepcopy(record)
    recorded = unhashed.get(field)
    unhashed[field] = ZERO_SHA256
    return recorded == sha256_bytes(canonical_json(unhashed))


def validate_suite(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["lifecycle scenario suite must be an object"]
    failures: list[str] = []
    if not valid_hash(value, "suite_sha256"):
        failures.append("lifecycle scenario suite self-hash is invalid")
    items = value.get("scenarios", [])
    if [item.get("category") for item in items if isinstance(item, dict)] != list(CATEGORIES):
        failures.append("lifecycle scenario categories are incomplete or reordered")
    if value.get("scenario_count") != len(CATEGORIES):
        failures.append("lifecycle scenario count is incomplete")
    for item in items if isinstance(items, list) else []:
        if not valid_hash(item, "scenario_sha256"):
            failures.append(f"lifecycle scenario hash is invalid: {item.get('category')}")
        previous = ZERO_SHA256
        for sequence, event in enumerate(item.get("events", []), start=1):
            if event.get("sequence") != sequence or event.get("previous_event_sha256") != previous or not valid_hash(event, "event_sha256"):
                failures.append(f"lifecycle event chain is invalid: {item.get('category')}")
                break
            previous = event["event_sha256"]
        if item.get("prohibited_effects") != PROHIBITED_EFFECTS:
            failures.append(f"lifecycle prohibited effects drifted: {item.get('category')}")
    if value.get("synthetic_only") is not True or value.get("product_runtime_executed") is not False or value.get("network_access") is not False:
        failures.append("lifecycle scenarios make a runtime or network overclaim")
    return failures


def check() -> list[str]:
    try:
        actual = read_json(OUTPUT_PATH)
        expected = build_suite()
    except (OSError, ValueError, json.JSONDecodeError) as error:
        return [f"cannot validate artifact lifecycle scenarios: {error}"]
    failures = validate_suite(actual)
    if actual != expected:
        failures.append("checked lifecycle scenario suite is stale, incomplete, or widened")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    if args.write:
        write_atomic(OUTPUT_PATH, canonical_json(build_suite()))
    failures = check()
    if failures:
        for failure in failures:
            print(f"Artifact lifecycle scenario validation failed: {failure}", file=sys.stderr)
        return 1
    print(f"Validated {len(CATEGORIES)} artifact lifecycle scenarios")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
