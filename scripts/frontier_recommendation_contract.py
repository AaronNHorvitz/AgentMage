#!/usr/bin/env python3
"""Validate the closed Sprint 51 frontier recommendation artifacts."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
CORPUS_PATH: Final = ROOT / "docs/verification/sprint-51-frontier-corpus.json"
RUST_HANDOFF: Final = ROOT / "kernel/contracts/src/handoff.rs"
TYPESCRIPT_HANDOFF: Final = ROOT / "shells/vscode/src/handoff.ts"
TRIGGERS: Final = {
    "measured_capability_failure",
    "repeated_validation_failure",
    "contradiction",
    "rejected_verification",
    "exhausted_budget",
    "material_clarification_need",
}
DELIVERY_ACTIONS: Final = [
    "codex_invocation",
    "tab_activation",
    "chat_population",
    "clipboard_write",
    "uri_launch",
    "local_runtime_delivery",
    "raw_runtime_delivery",
    "network_call",
    "automatic_submission",
    "file_upload",
    "browser_control",
    "scheduled_delivery",
    "routed_delivery",
    "standing_consent_delivery",
]


def read() -> Any:
    return json.loads(CORPUS_PATH.read_text(encoding="utf-8"))


def rust_variant(wire: str) -> str:
    return "".join(part.title() for part in wire.split("_"))


def validate(value: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict):
        return ["frontier corpus must be an object"]
    expected_fields = {
        "schema_version",
        "corpus_id",
        "tier_cases",
        "disclosure_cases",
        "prohibited_delivery_attempts",
        "review_mutations",
        "expanded_case_counts",
        "external_delivery_attempted",
    }
    if set(value) != expected_fields:
        failures.append("frontier corpus field closure drifted")
    if (
        value.get("schema_version") != 1
        or value.get("corpus_id") != "sprint-51-frontier-recommendation-v1"
        or value.get("external_delivery_attempted") is not False
    ):
        failures.append("frontier corpus identity or no-delivery truth drifted")
    tier_cases = value.get("tier_cases", [])
    disclosures = value.get("disclosure_cases", [])
    deliveries = value.get("prohibited_delivery_attempts", [])
    mutations = value.get("review_mutations", [])
    counts = value.get("expanded_case_counts", {})
    observed = {
        "tier_cases": len(tier_cases),
        "disclosure_cases": len(disclosures),
        "prohibited_delivery_attempts": len(deliveries),
        "review_mutations": len(mutations),
        "total": len(tier_cases) + len(disclosures) + len(deliveries) + len(mutations),
    }
    if observed != counts or observed != {
        "tier_cases": 13,
        "disclosure_cases": 10,
        "prohibited_delivery_attempts": 14,
        "review_mutations": 8,
        "total": 45,
    }:
        failures.append("frontier corpus cardinality drifted")
    if len({case.get("case_id") for case in tier_cases}) != len(tier_cases):
        failures.append("frontier tier case identity duplicated")
    for case in tier_cases:
        tier = case.get("expected_tier")
        trigger = case.get("expected_trigger")
        if tier == "frontier_recommended":
            if case.get("local_model_attempted") is not True or trigger not in TRIGGERS:
                failures.append(f"unmeasured or unapproved recommendation: {case.get('case_id')}")
        elif trigger is not None:
            failures.append(f"non-frontier tier has a trigger: {case.get('case_id')}")
    if [item.get("action") for item in deliveries] != DELIVERY_ACTIONS:
        failures.append("prohibited delivery action closure drifted")
    if any(item.get("expected") != "local_denial_receipt" for item in deliveries):
        failures.append("delivery attempt does not fail locally")
    rust = RUST_HANDOFF.read_text(encoding="utf-8")
    typescript = TYPESCRIPT_HANDOFF.read_text(encoding="utf-8")
    for action in DELIVERY_ACTIONS:
        if rust_variant(action) not in rust:
            failures.append(f"Rust delivery denial is absent: {action}")
        if f'"{action}"' not in typescript:
            failures.append(f"TypeScript delivery denial is absent: {action}")
    required_disclosures = {
        "credential-canary",
        "private-file",
        "unrelated-workspace",
        "hidden-metadata",
        "absolute-path",
        "excessive-excerpt",
        "authority-object",
        "prompt-injection",
        "missing-current-state",
        "complete-redaction",
    }
    if {item.get("case_id") for item in disclosures} != required_disclosures:
        failures.append("frontier disclosure attack closure drifted")
    return failures


def main() -> int:
    failures = validate(read())
    if failures:
        print("frontier recommendation contract failed:")
        for failure in failures:
            print(f"- {failure}")
        return 1
    print("frontier recommendation contract validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
