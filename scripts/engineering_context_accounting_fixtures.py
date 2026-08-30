#!/usr/bin/env python3
"""Build and verify complete context-accounting scenarios for Story 2.4.2.2."""

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


FIXTURE_DIR: Final = ROOT / "fixtures" / "artifact-admission" / "v1"
LINEAGE_PATH: Final = FIXTURE_DIR / "lineage-manifest.json"
OUTPUT_PATH: Final = FIXTURE_DIR / "context-accounting-manifests.json"
SCENARIOS: Final = (
    "token_budget_overflow",
    "retrieval",
    "summary",
    "duplicate",
    "stale",
    "restricted",
    "omitted",
    "model_profile_change",
)
TARGETS: Final = {
    "token_budget_overflow": "admission-local-file-1",
    "retrieval": "admission-paste-1",
    "summary": "admission-pdf-1",
    "duplicate": "admission-duplicate-1",
    "stale": "admission-stale-1",
    "restricted": "admission-virtual-uri-1",
    "omitted": "admission-log-1",
    "model_profile_change": "admission-text-1",
}
EXPECTED_TARGET_DISPOSITIONS: Final = {
    "token_budget_overflow": "truncated",
    "retrieval": "included",
    "summary": "summarized",
    "duplicate": "duplicate",
    "stale": "stale",
    "restricted": "restricted",
    "omitted": "omitted",
    "model_profile_change": "omitted",
}


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def fixture_cases() -> list[dict[str, Any]]:
    return read_json(LINEAGE_PATH)["cases"]


def available_range(case: dict[str, Any], maximum: int = 64) -> dict[str, int]:
    if not case["ranges"]:
        raise ValueError(f"scenario target has no captured range: {case['fixture_id']}")
    source_range = case["ranges"][0]["source_range"]
    end = min(source_range["end_byte_exclusive"], source_range["start_byte"] + maximum)
    return {"start_byte": source_range["start_byte"], "end_byte_exclusive": end}


def default_item(case: dict[str, Any]) -> dict[str, Any]:
    disposition = case["source_records"]["context_disposition"]
    original = disposition["disposition"]
    if original in {"duplicate", "stale", "unsupported", "unavailable", "restricted"}:
        return {
            "artifact_id": case["source_artifact_id"],
            "disposition": original,
            "ranges": [],
            "token_count": 0,
            "reason_code": disposition["reason_code"],
            "reason": disposition["reason"],
        }
    return {
        "artifact_id": case["source_artifact_id"],
        "disposition": "omitted",
        "ranges": [],
        "token_count": 0,
        "reason_code": "scenario_not_selected",
        "reason": "The complete synthetic manifest accounts for this source without selecting it.",
    }


def target_item(scenario: str, case: dict[str, Any]) -> dict[str, Any]:
    artifact_id = case["source_artifact_id"]
    if scenario == "token_budget_overflow":
        byte_range = available_range(case, 32)
        return {
            "artifact_id": artifact_id,
            "disposition": "truncated",
            "ranges": [byte_range],
            "token_count": 8,
            "reason_code": "token_budget_exhausted",
            "reason": "Only the exact bounded prefix fits after reserved output and safety margin.",
        }
    if scenario == "retrieval":
        byte_range = available_range(case, 64)
        return {
            "artifact_id": artifact_id,
            "disposition": "included",
            "ranges": [byte_range],
            "token_count": 16,
            "reason_code": None,
            "reason": None,
        }
    if scenario == "summary":
        byte_range = available_range(case, 64)
        return {
            "artifact_id": artifact_id,
            "disposition": "summarized",
            "ranges": [byte_range],
            "token_count": 8,
            "reason_code": "bounded_summary_fixture",
            "reason": "A fixture summary accounts for the source range without claiming parser execution.",
        }
    if scenario == "restricted":
        return {
            "artifact_id": artifact_id,
            "disposition": "restricted",
            "ranges": [],
            "token_count": 0,
            "reason_code": "scenario_policy_restricted",
            "reason": "Scenario policy denies model inclusion without changing source classification.",
        }
    if scenario == "model_profile_change":
        return {
            "artifact_id": artifact_id,
            "disposition": "omitted",
            "ranges": [],
            "token_count": 0,
            "reason_code": "model_profile_changed",
            "reason": "Prior token estimates are invalid and no stale context is reused.",
        }
    if scenario == "omitted":
        return {
            "artifact_id": artifact_id,
            "disposition": "omitted",
            "ranges": [],
            "token_count": 0,
            "reason_code": "explicit_scenario_omission",
            "reason": "The source is deliberately omitted and remains visible in the manifest.",
        }
    return default_item(case)


def scenario_record(scenario: str, cases: list[dict[str, Any]]) -> dict[str, Any]:
    target_fixture_id = TARGETS[scenario]
    items = [
        target_item(scenario, case) if case["fixture_id"] == target_fixture_id else default_item(case)
        for case in cases
    ]
    accounted_tokens = sum(item["token_count"] for item in items)
    profile = "model-profile-fixture-b" if scenario == "model_profile_change" else "model-profile-fixture-a"
    manifest = sealed(
        {
            "schema_version": 2,
            "context_manifest_id": f"context-{scenario}-v1",
            "session_id": "session-context-accounting-v1",
            "turn_id": f"turn-{scenario}-v1",
            "model_profile_id": profile,
            "source_artifact_count": len(items),
            "items": items,
            "total_input_tokens": accounted_tokens,
            "reserved_output_tokens": 32,
            "safety_margin_tokens": 16,
            "manifest_sha256": ZERO_SHA256,
        },
        "manifest_sha256",
    )
    requested = 2048 if scenario == "token_budget_overflow" else accounted_tokens
    return {
        "scenario_id": scenario,
        "target_fixture_id": target_fixture_id,
        "target_artifact_id": next(case["source_artifact_id"] for case in cases if case["fixture_id"] == target_fixture_id),
        "expected_target_disposition": EXPECTED_TARGET_DISPOSITIONS[scenario],
        "prior_model_profile_id": "model-profile-fixture-a" if scenario == "model_profile_change" else None,
        "model_profile_change_invalidates_prior_manifest": scenario == "model_profile_change",
        "requested_input_tokens": requested,
        "accounted_input_tokens": accounted_tokens,
        "context_window_tokens": accounted_tokens + 48,
        "complete_manifest_count": 1,
        "context_manifest": manifest,
        "execution_claim": "synthetic_manifest_contract_only",
    }


def build_suite() -> dict[str, Any]:
    cases = fixture_cases()
    value = {
        "schema_version": 1,
        "suite_id": "engineering-context-accounting-v1",
        "task_id": "2.4.2.2",
        "generated_on": "2026-08-29",
        "status": "synthetic-context-accounting-contract",
        "synthetic_only": True,
        "model_request_executed": False,
        "product_context_delivery_claim": "none",
        "lineage_manifest": {
            "path": str(LINEAGE_PATH.relative_to(ROOT)),
            "sha256": sha256_bytes(LINEAGE_PATH.read_bytes()),
        },
        "generator": {
            "path": "scripts/engineering_context_accounting_fixtures.py",
            "sha256": sha256_bytes(Path(__file__).read_bytes()),
        },
        "required_scenarios": list(SCENARIOS),
        "source_count_per_manifest": len(cases),
        "scenarios": [scenario_record(scenario, cases) for scenario in SCENARIOS],
    }
    value["suite_sha256"] = sha256_bytes(canonical_json(value))
    return value


def validate_suite(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["context accounting suite must be an object"]
    failures: list[str] = []
    unhashed = copy.deepcopy(value)
    recorded = unhashed.pop("suite_sha256", None)
    if recorded != sha256_bytes(canonical_json(unhashed)):
        failures.append("context accounting suite self-hash is invalid")
    if value != build_suite():
        failures.append("context accounting suite is stale, incomplete, reordered, or widened")
    scenarios = value.get("scenarios", [])
    if value.get("required_scenarios") != list(SCENARIOS) or [item.get("scenario_id") for item in scenarios] != list(SCENARIOS):
        failures.append("context accounting scenario closure drifted")
    expected_source_count = value.get("source_count_per_manifest")
    for scenario in scenarios if isinstance(scenarios, list) else []:
        manifest = scenario.get("context_manifest", {})
        items = manifest.get("items", [])
        if scenario.get("complete_manifest_count") != 1 or len(items) != expected_source_count or manifest.get("source_artifact_count") != expected_source_count:
            failures.append(f"context manifest is incomplete: {scenario.get('scenario_id')}")
        if len({item.get("artifact_id") for item in items}) != len(items):
            failures.append(f"context manifest repeats an artifact: {scenario.get('scenario_id')}")
        accounted = sum(item.get("token_count", 0) for item in items)
        if accounted != manifest.get("total_input_tokens") or accounted != scenario.get("accounted_input_tokens"):
            failures.append(f"context tokens do not reconcile: {scenario.get('scenario_id')}")
        if accounted + manifest.get("reserved_output_tokens", 0) + manifest.get("safety_margin_tokens", 0) != scenario.get("context_window_tokens"):
            failures.append(f"context window partitions do not reconcile: {scenario.get('scenario_id')}")
        target = next((item for item in items if item.get("artifact_id") == scenario.get("target_artifact_id")), None)
        if target is None or target.get("disposition") != scenario.get("expected_target_disposition"):
            failures.append(f"context target path was not exercised: {scenario.get('scenario_id')}")
        if scenario.get("scenario_id") == "token_budget_overflow" and scenario.get("requested_input_tokens", 0) <= accounted:
            failures.append("token overflow fixture does not exceed the admitted budget")
        if scenario.get("scenario_id") == "model_profile_change" and (
            scenario.get("prior_model_profile_id") == manifest.get("model_profile_id")
            or scenario.get("model_profile_change_invalidates_prior_manifest") is not True
        ):
            failures.append("model profile change did not invalidate prior context")
    if value.get("synthetic_only") is not True or value.get("model_request_executed") is not False or value.get("product_context_delivery_claim") != "none":
        failures.append("context accounting suite makes a product execution or delivery overclaim")
    return failures


def check() -> list[str]:
    try:
        actual = read_json(OUTPUT_PATH)
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot validate context accounting suite: {error}"]
    return validate_suite(actual)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    if args.write:
        write_atomic(OUTPUT_PATH, canonical_json(build_suite()))
    failures = check()
    if failures:
        for failure in failures:
            print(f"Context accounting fixture validation failed: {failure}", file=sys.stderr)
        return 1
    suite = read_json(OUTPUT_PATH)
    print(f"Validated {len(suite['scenarios'])} complete context-accounting manifests")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
