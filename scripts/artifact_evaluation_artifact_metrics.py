#!/usr/bin/env python3
"""Build and verify Story 2.3.3.1 artifact golden metrics."""

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


EVALUATION_DIR: Final = ROOT / "fixtures" / "artifact-evaluation" / "v1"
TEXT_PATH: Final = EVALUATION_DIR / "text-reference-manifest.json"
DOCUMENT_PATH: Final = EVALUATION_DIR / "document-variant-manifest.json"
CONTEXT_PATH: Final = ROOT / "fixtures" / "artifact-admission" / "v1" / "context-accounting-manifests.json"
LIFECYCLE_PATH: Final = EVALUATION_DIR / "lifecycle-scenarios.json"
RESOURCE_PATH: Final = ROOT / "fixtures" / "artifact-admission" / "v1" / "resource-gate-observations.json"
OUTPUT_PATH: Final = EVALUATION_DIR / "artifact-golden-metrics.json"
METRIC_IDS: Final = (
    "extraction_coverage",
    "provenance_accuracy",
    "section_fidelity",
    "context_inclusion",
    "context_omission",
    "token_budget_reconciliation",
    "latency_boundary",
    "peak_memory_boundary",
    "cancellation",
    "cleanup",
)
ADMITTING_DISPOSITIONS: Final = ("included", "summarized", "truncated")


def load(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def fraction(numerator: int, denominator: int) -> dict[str, int]:
    return {"numerator": numerator, "denominator": denominator}


def metric(metric_id: str, *, unit: str, golden: dict[str, Any], source_scope: list[str], measurement_scope: str) -> dict[str, Any]:
    return {
        "metric_id": metric_id,
        "unit": unit,
        "measurement_scope": measurement_scope,
        "source_scope": source_scope,
        "golden": golden,
        "product_observation": None,
        "product_measurement_status": "not_executed",
        "support_claim": "none",
    }


def build_metrics() -> list[dict[str, Any]]:
    text = load(TEXT_PATH)
    documents = load(DOCUMENT_PATH)
    context = load(CONTEXT_PATH)
    lifecycle = load(LIFECYCLE_PATH)
    resources = load(RESOURCE_PATH)

    text_cases = text["cases"]
    document_cases = documents["cases"]
    extraction_cases = text_cases + document_cases
    provenance_matches = sum(case["provenance"]["source_sha256"] == case["byte_identity"]["sha256"] for case in document_cases)
    sections = [section for case in document_cases for section in case["expected_sections"]]
    faithful_sections = sum(
        bool(section["section_id"] and section["coordinate"] and len(section["content_sha256"]) == 64 and section["state"] in {"exact_text", "exact_structure", "ocr_required"})
        for section in sections
    )

    manifests = [scenario["context_manifest"] for scenario in context["scenarios"]]
    context_items = [item for manifest in manifests for item in manifest["items"]]
    admitted = [item for item in context_items if item["disposition"] in ADMITTING_DISPOSITIONS]
    omitted = [item for item in context_items if item["disposition"] not in ADMITTING_DISPOSITIONS]
    reconciled = sum(
        sum(item["token_count"] for item in manifest["items"]) == manifest["total_input_tokens"] == scenario["accounted_input_tokens"]
        and manifest["total_input_tokens"] + manifest["reserved_output_tokens"] + manifest["safety_margin_tokens"] <= scenario["context_window_tokens"]
        for scenario, manifest in zip(context["scenarios"], manifests, strict=True)
    )
    total_tokens = sum(manifest["total_input_tokens"] for manifest in manifests)

    resource_by_dimension = {item["dimension"]: item for item in resources["resource_observations"]}
    time_boundary = resource_by_dimension["time"]
    memory_boundary = resource_by_dimension["memory"]
    lifecycle_by_id = {item["scenario_id"]: item for item in lifecycle["scenarios"]}
    cancellation_scenario = lifecycle_by_id["artifact-evaluation-cancellation-v1"]
    cancellation_observation = next(item for item in resources["lifecycle_observations"] if item["event"] == "cancellation")
    cancellation_checks = (
        cancellation_scenario["expected"]["terminal_state"] == "cancelled"
        and cancellation_scenario["expected"]["residue_count"] == 0
        and cancellation_scenario["expected"]["derivative_published"] is False,
        cancellation_observation["residue_after_cleanup"] == 0
        and cancellation_observation["worker_stopped"] is True
        and cancellation_observation["owned_root_removed"] is True,
    )
    cleanup_lifecycle = [lifecycle_by_id[f"artifact-evaluation-{name}-v1"] for name in ("cancellation", "retention", "deletion")]
    cleanup_scenario_checks = (
        cleanup_lifecycle[0]["expected"]["residue_count"] == 0,
        cleanup_lifecycle[1]["expected"]["residue_count"] == 0 and cleanup_lifecycle[1]["expected"]["persisted_payload_count"] == 0,
        cleanup_lifecycle[2]["expected"]["remaining_reference_count"] == 0 and cleanup_lifecycle[2]["expected"]["resurrection_allowed"] is False,
    )
    cleanup_observation_checks = tuple(
        item["residue_after_cleanup"] == 0 and item["worker_stopped"] is True and item["owned_root_removed"] is True
        for item in resources["lifecycle_observations"]
    )

    return [
        metric(
            "extraction_coverage",
            unit="fixture_terminal_dispositions",
            golden={"fraction": fraction(len(extraction_cases), len(extraction_cases)), "expected_case_count": len(extraction_cases), "silent_omission_allowed": False},
            source_scope=[str(TEXT_PATH.relative_to(ROOT)), str(DOCUMENT_PATH.relative_to(ROOT))],
            measurement_scope="golden_fixture_oracle",
        ),
        metric(
            "provenance_accuracy",
            unit="source_identity_bindings",
            golden={"fraction": fraction(provenance_matches, len(document_cases)), "expected_binding_count": len(document_cases), "identity_substitution_allowed": False},
            source_scope=[str(DOCUMENT_PATH.relative_to(ROOT))],
            measurement_scope="golden_fixture_oracle",
        ),
        metric(
            "section_fidelity",
            unit="exact_expected_sections",
            golden={"fraction": fraction(faithful_sections, len(sections)), "expected_section_count": len(sections), "coordinate_hash_or_state_drift_allowed": False},
            source_scope=[str(DOCUMENT_PATH.relative_to(ROOT))],
            measurement_scope="golden_fixture_oracle",
        ),
        metric(
            "context_inclusion",
            unit="admitting_context_items",
            golden={"fraction": fraction(len(admitted), len(admitted)), "expected_item_count": len(admitted), "admitting_dispositions": list(ADMITTING_DISPOSITIONS)},
            source_scope=[str(CONTEXT_PATH.relative_to(ROOT))],
            measurement_scope="golden_fixture_oracle",
        ),
        metric(
            "context_omission",
            unit="explicit_non_admitting_context_items",
            golden={"fraction": fraction(len(omitted), len(omitted)), "expected_item_count": len(omitted), "reason_required": True, "silent_omission_allowed": False},
            source_scope=[str(CONTEXT_PATH.relative_to(ROOT))],
            measurement_scope="golden_fixture_oracle",
        ),
        metric(
            "token_budget_reconciliation",
            unit="complete_context_manifests",
            golden={"fraction": fraction(reconciled, len(manifests)), "manifest_count": len(manifests), "accounted_input_tokens": total_tokens, "counter_disagreement_allowed": False},
            source_scope=[str(CONTEXT_PATH.relative_to(ROOT))],
            measurement_scope="golden_fixture_oracle",
        ),
        metric(
            "latency_boundary",
            unit=time_boundary["unit"],
            golden={"fraction": fraction(1 if time_boundary["peak_admitted"] == time_boundary["ceiling"] and time_boundary["overflow_admitted"] is False else 0, 1), "inclusive_ceiling": time_boundary["ceiling"], "first_denied_attempt": time_boundary["first_denied_attempt"], "wall_clock_persisted": False},
            source_scope=[str(RESOURCE_PATH.relative_to(ROOT))],
            measurement_scope="synthetic_logical_resource_boundary_not_product_performance",
        ),
        metric(
            "peak_memory_boundary",
            unit=memory_boundary["unit"],
            golden={"fraction": fraction(1 if memory_boundary["peak_admitted"] == memory_boundary["ceiling"] and memory_boundary["overflow_admitted"] is False else 0, 1), "inclusive_ceiling": memory_boundary["ceiling"], "first_denied_attempt": memory_boundary["first_denied_attempt"]},
            source_scope=[str(RESOURCE_PATH.relative_to(ROOT))],
            measurement_scope="synthetic_logical_resource_boundary_not_product_peak_memory",
        ),
        metric(
            "cancellation",
            unit="cancellation_oracles",
            golden={"fraction": fraction(sum(cancellation_checks), len(cancellation_checks)), "expected_terminal_state": "cancelled", "expected_residue_count": 0, "false_completion_allowed": False},
            source_scope=[str(LIFECYCLE_PATH.relative_to(ROOT)), str(RESOURCE_PATH.relative_to(ROOT))],
            measurement_scope="golden_fixture_and_synthetic_local_lifecycle_oracle",
        ),
        metric(
            "cleanup",
            unit="cleanup_oracles",
            golden={"fraction": fraction(sum(cleanup_scenario_checks) + sum(cleanup_observation_checks), len(cleanup_scenario_checks) + len(cleanup_observation_checks)), "scenario_oracle_count": len(cleanup_scenario_checks), "local_lifecycle_observation_count": len(cleanup_observation_checks), "expected_residue_count": 0},
            source_scope=[str(LIFECYCLE_PATH.relative_to(ROOT)), str(RESOURCE_PATH.relative_to(ROOT))],
            measurement_scope="golden_fixture_and_synthetic_local_lifecycle_oracle",
        ),
    ]


def build_suite() -> dict[str, Any]:
    dependencies = [TEXT_PATH, DOCUMENT_PATH, CONTEXT_PATH, LIFECYCLE_PATH, RESOURCE_PATH]
    value = {
        "schema_version": 1,
        "suite_id": "artifact-evaluation-artifact-golden-metrics-v1",
        "task_id": "2.3.3.1",
        "generated_on": "2026-08-30",
        "status": "golden-fixture-and-synthetic-local-metrics",
        "synthetic_only": True,
        "product_parser_executed": False,
        "product_runtime_executed": False,
        "product_performance_measured": False,
        "platform_support_claim": "none",
        "required_metric_ids": list(METRIC_IDS),
        "metric_count": len(METRIC_IDS),
        "dependencies": [{"path": str(path.relative_to(ROOT)), "sha256": sha256_bytes(path.read_bytes())} for path in dependencies],
        "generator": {"path": "scripts/artifact_evaluation_artifact_metrics.py", "sha256": sha256_bytes(Path(__file__).read_bytes())},
        "metrics": build_metrics(),
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
        return ["artifact metric suite must be an object"]
    failures: list[str] = []
    if not valid_hash(value, "suite_sha256"):
        failures.append("artifact metric suite self-hash is invalid")
    metrics = value.get("metrics", [])
    if not isinstance(metrics, list):
        return failures + ["artifact metrics must be a list"]
    if [item.get("metric_id") for item in metrics if isinstance(item, dict)] != list(METRIC_IDS):
        failures.append("artifact metric set is incomplete or reordered")
    if value.get("metric_count") != len(METRIC_IDS):
        failures.append("artifact metric count is incomplete")
    for item in metrics:
        if not isinstance(item, dict):
            failures.append("artifact metric must be an object")
            continue
        ratio = item.get("golden", {}).get("fraction", {})
        if ratio.get("numerator") != ratio.get("denominator") or not isinstance(ratio.get("denominator"), int) or ratio.get("denominator", 0) <= 0:
            failures.append(f"artifact golden metric is not exact: {item.get('metric_id')}")
        if item.get("product_observation") is not None or item.get("product_measurement_status") != "not_executed" or item.get("support_claim") != "none":
            failures.append(f"artifact metric makes a product or support overclaim: {item.get('metric_id')}")
    overclaim_fields = ("product_parser_executed", "product_runtime_executed", "product_performance_measured")
    if value.get("synthetic_only") is not True or any(value.get(field) is not False for field in overclaim_fields) or value.get("platform_support_claim") != "none":
        failures.append("artifact metric suite makes a product, performance, or platform overclaim")
    return failures


def check() -> list[str]:
    try:
        actual = load(OUTPUT_PATH)
        expected = build_suite()
    except (OSError, ValueError, json.JSONDecodeError) as error:
        return [f"cannot validate artifact golden metrics: {error}"]
    failures = validate_suite(actual)
    if actual != expected:
        failures.append("checked artifact golden metrics are stale, incomplete, or widened")
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
            print(f"Artifact golden metric validation failed: {failure}", file=sys.stderr)
        return 1
    print(f"Validated {len(METRIC_IDS)} artifact golden metrics")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
