#!/usr/bin/env python3
"""Validate frozen Sprint 49 candidate, role, and routing artifacts."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
ROUTING: Final = ROOT / "model-profiles/routing"
HISTORICAL: Final = ROUTING / "historical-later-candidates.json"
CORPUS: Final = ROUTING / "role-benchmark-corpus-v1.json"
TABLE: Final = ROUTING / "measured-routing-decision-table-v1.json"
INVENTORY: Final = (
    ROOT / "model-profiles/catalogs/2026-08-14/candidate-inventory.json"
)
EXPECTED_ROLES: Final = {
    "dialogue",
    "tool_selection",
    "summarization",
    "repository_map",
    "embedding",
    "reranking",
    "patch_generation",
    "citation_verification",
    "planning",
    "coding",
    "retrieval",
    "document",
}
HISTORICAL_FIELDS: Final = {
    "candidate_id",
    "display_name",
    "historical_reference",
    "frozen_source_entry_id",
    "frozen_source_revision",
    "artifact_listing_sha256",
    "publisher",
    "publisher_control",
    "license_reference",
    "origin_disposition",
    "exact_profile_manifest",
    "exact_artifact_sha256",
    "exact_tokenizer_sha256",
    "exact_template_sha256",
    "exact_codec_sha256",
    "exact_runtime_sha256",
    "exact_context_sha256",
    "exact_decoding_sha256",
    "exact_resource_sha256",
    "exact_role_evidence_sha256",
    "exact_platform_evidence_sha256",
    "disposition",
    "selectable",
    "automatic_fallback",
    "preserved",
    "blockers",
}


def load(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def file_sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def validate_historical(value: dict[str, Any], inventory: dict[str, Any]) -> None:
    if set(value) != {
        "schema_version",
        "record_type",
        "enabled_profile_count",
        "automatic_routing_enabled",
        "candidates",
    }:
        raise ValueError("historical candidate record is not closed")
    if (
        value["schema_version"] != 1
        or value["record_type"] != "agentmage_historical_later_model_candidates"
        or value["enabled_profile_count"] != 0
        or value["automatic_routing_enabled"] is not False
    ):
        raise ValueError("historical candidate record overclaims product state")
    candidates = value["candidates"]
    if [item["candidate_id"] for item in candidates] != [
        "gemma-4-26b-later-candidate",
        "devstral-small-2-later-candidate",
    ]:
        raise ValueError("named historical candidate inventory drifted")
    if any(set(item) != HISTORICAL_FIELDS for item in candidates):
        raise ValueError("historical candidate fields drifted")
    exact_fields = {
        field
        for field in HISTORICAL_FIELDS
        if field.startswith("exact_")
    }
    for item in candidates:
        if (
            item["selectable"]
            or item["automatic_fallback"]
            or item["preserved"] is not True
            or not item["blockers"]
            or any(item[field] is not None for field in exact_fields)
        ):
            raise ValueError("historical candidate gained unsupported authority")
    gemma = candidates[0]
    inventory_by_id = {item["entry_id"]: item for item in inventory["entries"]}
    source = inventory_by_id.get(gemma["frozen_source_entry_id"])
    if source is None:
        raise ValueError("Gemma historical source entry is absent")
    if any(
        gemma[field] != source[source_field]
        for field, source_field in {
            "historical_reference": "repository",
            "frozen_source_revision": "revision",
            "artifact_listing_sha256": "artifact_listing_sha256",
            "publisher": "publisher",
            "publisher_control": "publisher_control",
            "license_reference": "license_use_terms",
        }.items()
    ):
        raise ValueError("Gemma historical source attribution drifted")
    devstral = candidates[1]
    if (
        devstral["historical_reference"] != "ai/devstral-small-2:24B"
        or devstral["disposition"] != "UNRESOLVED"
        or devstral["origin_disposition"] != "unresolved"
    ):
        raise ValueError("Devstral unresolved historical boundary drifted")


def validate_corpus(value: dict[str, Any]) -> None:
    if set(value) != {
        "schema_version",
        "record_type",
        "corpus_id",
        "minimum_repeated_trials",
        "minimum_primary_improvement_bps",
        "minimum_verification_improvement_bps",
        "manual_selection_baseline_required",
        "exact_tuple_comparability_required",
        "model_confidence_is_routing_evidence",
        "roles",
        "required_exact_tuple_fields",
    }:
        raise ValueError("role corpus is not closed")
    if (
        value["schema_version"] != 1
        or value["record_type"] != "agentmage_routing_role_benchmark_corpus"
        or value["minimum_repeated_trials"] != 30
        or value["minimum_primary_improvement_bps"] != 250
        or value["minimum_verification_improvement_bps"] != 150
        or value["manual_selection_baseline_required"] is not True
        or value["exact_tuple_comparability_required"] is not True
        or value["model_confidence_is_routing_evidence"] is not False
    ):
        raise ValueError("role corpus threshold or authority drift")
    roles = value["roles"]
    if {item["role"] for item in roles} != EXPECTED_ROLES:
        raise ValueError("role corpus is incomplete")
    if len(roles) != len(EXPECTED_ROLES):
        raise ValueError("role corpus contains duplicate roles")
    for item in roles:
        if set(item) != {"role", "suite_ids"} or len(item["suite_ids"]) != len(
            set(item["suite_ids"])
        ):
            raise ValueError("role suite inventory is malformed")
        for required in ("reliability", "failure", "latency", "memory", "energy_optional"):
            if required not in item["suite_ids"]:
                raise ValueError(f"role suite is incomplete: {item['role']}")
    fields = value["required_exact_tuple_fields"]
    if len(fields) != len(set(fields)) or len(fields) != 19:
        raise ValueError("exact tuple field inventory drifted")


def validate_table(value: dict[str, Any]) -> None:
    if set(value) != {
        "schema_version",
        "record_type",
        "router",
        "authority_source",
        "frontier_transfer",
        "invisible_fallback",
        "provider_marketplace",
        "automatic_install",
        "model_self_selection",
        "model_confidence_used",
        "rules",
        "visible_budgets",
    }:
        raise ValueError("routing decision table is not closed")
    if any(
        value[field]
        for field in (
            "frontier_transfer",
            "invisible_fallback",
            "provider_marketplace",
            "automatic_install",
            "model_self_selection",
            "model_confidence_used",
        )
    ):
        raise ValueError("routing table contains hidden or remote authority")
    rules = value["rules"]
    if len(rules) != 16 or any(
        set(item) != {"condition", "decision", "fallback", "visible"}
        or item["fallback"]
        or item["visible"] is not True
        for item in rules
    ):
        raise ValueError("routing rule inventory drifted")
    conditions = [item["condition"] for item in rules]
    if len(conditions) != len(set(conditions)):
        raise ValueError("routing conditions are not unique")
    budgets = value["visible_budgets"]
    if [item["budget"] for item in budgets] != ["fast", "standard", "deep", "verify"]:
        raise ValueError("visible budget inventory drifted")
    if any(
        set(item)
        != {
            "budget",
            "max_context_tokens",
            "max_tool_proposals",
            "review",
            "local_only",
        }
        or item["local_only"] is not True
        for item in budgets
    ):
        raise ValueError("visible budget boundary drifted")


def validate_all(
    historical: dict[str, Any],
    corpus: dict[str, Any],
    table: dict[str, Any],
    inventory: dict[str, Any],
) -> None:
    validate_historical(historical, inventory)
    validate_corpus(corpus)
    validate_table(table)


def main() -> int:
    validate_all(load(HISTORICAL), load(CORPUS), load(TABLE), load(INVENTORY))
    print(
        json.dumps(
            {
                "historical_sha256": file_sha256(HISTORICAL),
                "role_corpus_sha256": file_sha256(CORPUS),
                "decision_table_sha256": file_sha256(TABLE),
                "enabled_profile_count": 0,
                "automatic_routing_enabled": False,
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
