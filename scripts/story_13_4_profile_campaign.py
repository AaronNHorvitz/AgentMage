#!/usr/bin/env python3
"""Build the exact-tuple Story 13.4 profile campaign ledger."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
OUTPUT_PATH: Final = ROOT / "artifacts/sprints/sprint-13/story-13.4/profile-campaign-ledger.json"
CONTEXT_PATH: Final = ROOT / "fixtures/artifact-admission/v1/context-accounting-manifests.json"
WORKFLOW_PATH: Final = ROOT / "fixtures/artifact-evaluation/v1/workflow-plan-fixtures.json"
MUSE_PATH: Final = ROOT / "artifacts/sprints/sprint-13/story-13.3/muse-profile-evaluation.json"
GEMMA_PATHS: Final = (
    ROOT / "model-profiles/candidates/gemma-4-e4b/feasibility-disposition.json",
    ROOT / "model-profiles/candidates/gemma-4-12b-unified/feasibility-disposition.json",
)
ENGINE_PATH: Final = ROOT / "kernel/engine/src/model_orchestration_profile.rs"
SCHEMA_PATH: Final = ROOT / "schemas/model/orchestration-profile.schema.json"
INVARIANT_CONTROLS: Final = (
    "policy",
    "grant",
    "approval",
    "side-effect",
    "retry",
    "verifier",
    "budget",
    "completion",
)


def load(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def retained(path: Path) -> dict[str, Any]:
    return {
        "path": path.relative_to(ROOT).as_posix(),
        "byte_length": path.stat().st_size,
        "sha256": sha256(path),
    }


def digest(value: Any) -> str:
    encoded = json.dumps(value, separators=(",", ":"), sort_keys=True).encode()
    return hashlib.sha256(encoded).hexdigest()


def expected_ledger() -> dict[str, Any]:
    context = load(CONTEXT_PATH)
    workflow = load(WORKFLOW_PATH)
    muse = load(MUSE_PATH)
    gemma = [load(path) for path in GEMMA_PATHS]
    invariant_identity = digest(
        {
            "controls": INVARIANT_CONTROLS,
            "engine_sha256": sha256(ENGINE_PATH),
            "schema_sha256": sha256(SCHEMA_PATH),
        }
    )
    fake_identity = {
        "profile_id": "model-profile-fixture-a",
        "model_manifest_sha256": "synthetic-fixture-no-model-manifest",
        "runtime_identity": "deterministic-contract-runner",
        "tokenizer_identity": "fixture-authoritative-counter-v1",
        "decoding_identity": "deterministic-fixture-no-inference",
        "orchestration_profile_sha256": digest(
            {
                "context_suite": context["suite_sha256"],
                "workflow_suite": workflow["suite_sha256"],
                "invariant_control_identity": invariant_identity,
            }
        ),
    }
    context_ledgers = [
        {
            "scenario_id": scenario["scenario_id"],
            "manifest_sha256": scenario["context_manifest"]["manifest_sha256"],
            "requested_input_tokens": scenario["requested_input_tokens"],
            "accounted_input_tokens": scenario["accounted_input_tokens"],
            "source_artifact_count": scenario["context_manifest"]["source_artifact_count"],
            "complete_manifest_count": scenario["complete_manifest_count"],
            "target_disposition": scenario["expected_target_disposition"],
        }
        for scenario in context["scenarios"]
    ]
    workflow_metrics = {
        "workflow_case_count": workflow["fixture_count"],
        "attempt_count": sum(item["expected"].get("attempt_count", 0) for item in workflow["fixtures"]),
        "malformed_call_case_count": sum(item["category"] == "malformed_call" for item in workflow["fixtures"]),
        "repair_case_count": sum(item["category"] == "deterministic_repair" for item in workflow["fixtures"]),
        "verified_completion_case_count": sum(
            item["expected"]["terminal_state"] == "verified_success" for item in workflow["fixtures"]
        ),
        "false_completion_case_count": sum(
            item["expected"]["terminal_state"] == "false_completion" for item in workflow["fixtures"]
        ),
        "cancellation_case_count": 0,
        "diagnosis_case_count": sum(
            item["expected"]["terminal_state"] != "verified_success" for item in workflow["fixtures"]
        ),
        "tool_call_schema_valid_case_count": sum(
            all(step["call_state"] == "schema_valid" for step in item["steps"]) for item in workflow["fixtures"]
        ),
        "latency_ms": 0,
        "maximum_resident_memory_bytes": 0,
        "measurement_scope": "deterministic contract execution; zero denotes no model process or cancellation fixture",
    }
    rejected = []
    rejected.append(
        {
            "candidate_id": "muse-glimmer-30b-q4-k-m-text-8k",
            "disposition": muse["disposition"]["status"],
            "profile_campaign_status": "NOT_RUN_REJECTED_PRELAUNCH",
            "exact_identity": muse["tuple"],
            "exact_identity_source": retained(MUSE_PATH),
            "retained_quality": {
                "trial_count": muse["quality"]["trial_count"],
                "false_completion_count": muse["quality"]["false_completion_count"],
                "repeatability_trial_count": muse["diagnostic_repeatability"]["trial_count"],
                "maximum_resident_memory_bytes": muse["resources"]["maximum_resident_memory_bytes"],
            },
            "context_ledger": None,
            "orchestration_identity": None,
            "profile_metrics": None,
            "reason": muse["disposition"]["reason"],
        }
    )
    for path, value in zip(GEMMA_PATHS, gemma, strict=True):
        rejected.append(
            {
                "candidate_id": value["profile_id"],
                "disposition": value["decision"]["status"],
                "profile_campaign_status": "NOT_RUN_REJECTED_PRELAUNCH",
                "exact_identity": {
                    "disposition_sha256": sha256(path),
                    "adapter_identities": {
                        name: adapter.get("identities")
                        for name, adapter in value["adapters"].items()
                        if adapter.get("execution_status") == "COMPLETE"
                    },
                    "runtime_settings": {
                        name: adapter.get("runtime_settings")
                        for name, adapter in value["adapters"].items()
                        if adapter.get("execution_status") == "COMPLETE"
                    },
                },
                "exact_identity_source": retained(path),
                "retained_quality": {
                    name: {
                        "trials_completed": adapter.get("trials_completed"),
                        "metrics": adapter.get("metrics"),
                    }
                    for name, adapter in value["adapters"].items()
                    if adapter.get("execution_status") == "COMPLETE"
                },
                "context_ledger": None,
                "orchestration_identity": None,
                "profile_metrics": None,
                "reason": "candidate disposition rejected before orchestration-profile admission",
            }
        )
    return {
        "schema_version": 1,
        "record_type": "agentmage-story-13-4-profile-campaign-ledger",
        "generated_on": "2026-09-02",
        "status": "PASS_EXACT_TUPLE_LEDGER_ZERO_ADMITTED_PROFILES",
        "tuple_aggregation_permitted": False,
        "automatic_fallback_enabled": False,
        "admitted_profile_count": 0,
        "enabled_profile_count": 0,
        "invariant_controls": list(INVARIANT_CONTROLS),
        "invariant_control_identity": invariant_identity,
        "fixture_profile": {
            "execution_status": "PASS_SYNTHETIC_CONTRACT",
            "synthetic_only": True,
            "model_request_executed": False,
            "exact_identity": fake_identity,
            "context_ledgers": context_ledgers,
            "context_metrics": {
                "scenario_count": len(context_ledgers),
                "source_count_per_manifest": context["source_count_per_manifest"],
                "complete_context_accounting_count": sum(item["complete_manifest_count"] for item in context_ledgers),
                "omission_or_truncation_scenario_count": sum(
                    item["target_disposition"] != "included" for item in context_ledgers
                ),
            },
            "workflow_metrics": workflow_metrics,
        },
        "rejected_candidates": rejected,
        "future_admitted_profiles": [],
        "retained_inputs": [
            retained(path)
            for path in (CONTEXT_PATH, WORKFLOW_PATH, MUSE_PATH, *GEMMA_PATHS, ENGINE_PATH, SCHEMA_PATH)
        ],
        "protocol_mappings": {
            "RV-13": {"status": "partial", "evidence": "fake exact context ledger; no admitted local profile"},
            "RV-14": {"status": "partial", "evidence": "fake workflow and malformed/repair cases; no admitted model run"},
            "RV-16": {"status": "partial", "evidence": "retained exact candidate quality/resource traces; rejected tuples not merged"},
            "RV-41": {"status": "partial", "evidence": "deterministic cancellation contract and retained candidate traces"},
        },
        "claims": {
            "profile_campaign_evidence_complete": True,
            "live_admitted_profile_corpus_complete": False,
            "cross_platform_profile_campaign_complete": False,
            "story_complete": False,
            "sprint_complete": False,
            "release": "none",
        },
    }


def render(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, sort_keys=True) + "\n"


def validate_sources() -> list[str]:
    paths = (CONTEXT_PATH, WORKFLOW_PATH, MUSE_PATH, *GEMMA_PATHS, ENGINE_PATH, SCHEMA_PATH)
    failures = [f"missing campaign source: {path.relative_to(ROOT)}" for path in paths if not path.is_file()]
    if failures:
        return failures
    context = load(CONTEXT_PATH)
    workflow = load(WORKFLOW_PATH)
    muse = load(MUSE_PATH)
    if context.get("synthetic_only") is not True or context.get("model_request_executed") is not False:
        failures.append("context corpus is not the retained non-model synthetic suite")
    if workflow.get("synthetic_only") is not True or workflow.get("product_runtime_executed") is not False:
        failures.append("workflow corpus is not the retained non-runtime synthetic suite")
    if muse.get("disposition", {}).get("status") != "REJECTED":
        failures.append("Muse candidate is no longer rejected")
    for path in GEMMA_PATHS:
        if load(path).get("decision", {}).get("status") != "REJECTED":
            failures.append(f"Gemma candidate is no longer rejected: {path.relative_to(ROOT)}")
    return failures


def validate_ledger(value: Any) -> list[str]:
    return [] if value == expected_ledger() else ["Story 13.4 profile campaign ledger is stale or widened"]


def write() -> None:
    failures = validate_sources()
    if failures:
        raise ValueError("\n".join(failures))
    OUTPUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT_PATH.write_text(render(expected_ledger()), encoding="utf-8")


def check() -> list[str]:
    failures = validate_sources()
    try:
        value = load(OUTPUT_PATH)
    except (OSError, json.JSONDecodeError) as error:
        failures.append(f"cannot read profile campaign ledger: {error}")
    else:
        failures.extend(validate_ledger(value))
    return failures
