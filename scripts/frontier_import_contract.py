#!/usr/bin/env python3
"""Validate the closed Sprint 52 frontier-import corpus and source boundary."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
CORPUS_PATH: Final = ROOT / "docs/verification/sprint-52-frontier-import-corpus.json"
ENGINE_PATH: Final = ROOT / "kernel/engine/src/frontier_import.rs"
CONTRACT_PATH: Final = ROOT / "kernel/contracts/src/frontier_import.rs"
HOST_PATH: Final = ROOT / "shells/host/src/frontier_import_coordinator.rs"
REQUIRED_MANIFEST_FAILURES: Final = {
    "missing-required-field",
    "unknown-field",
    "malformed-json",
    "oversized-manifest",
    "version-mismatch",
    "wrong-request-hash",
    "manifest-hash-drift",
    "unsorted-identities",
    "duplicate-identity",
    "unsupported-artifact-media",
    "authority-granted",
    "completion-credit",
    "outbound-network-required",
    "operation-authority-mismatch",
}
REQUIRED_ARTIFACT_ATTACKS: Final = {
    "prompt-injection",
    "malicious-command",
    "path-escape",
    "absolute-path",
    "hidden-binary",
    "secret-canary",
    "fabricated-test-result",
    "fabricated-citation",
    "overbroad-file-change",
    "authority-request",
    "external-link",
    "artifact-hash-mismatch",
}
REQUIRED_STATE_CHANGES: Final = {
    "request-packet-changed",
    "workspace-state-changed",
    "model-state-changed",
    "policy-changed",
    "permissions-unchecked",
    "citation-stale",
    "citation-conflict",
    "citation-missing",
}
REQUIRED_FLOWS: Final = {
    "claim",
    "file-proposal",
    "command-proposal",
    "tool-proposal",
    "test-result",
    "decision",
    "link-reference",
}
REQUIRED_MUTATIONS: Final = [
    "report-hash",
    "step-outcome",
    "disagreement",
    "re-escalation-reason",
    "capability-feedback",
    "applied-effect-count",
    "duplicate-effect-count",
    "outbound-network",
    "receipt-hash",
]


def read() -> Any:
    return json.loads(CORPUS_PATH.read_text(encoding="utf-8"))


def validate(value: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict):
        return ["frontier import corpus must be an object"]
    expected_fields = {
        "schema_version",
        "corpus_id",
        "manifest_failures",
        "artifact_attacks",
        "state_change_cases",
        "local_flow_cases",
        "round_trip_mutations",
        "expanded_case_counts",
        "authority_granted",
        "effect_count",
        "outbound_network_used",
    }
    if set(value) != expected_fields:
        failures.append("frontier import corpus field closure drifted")
    if (
        value.get("schema_version") != 1
        or value.get("corpus_id") != "sprint-52-frontier-import-v1"
        or value.get("authority_granted") is not False
        or value.get("effect_count") != 0
        or value.get("outbound_network_used") is not False
    ):
        failures.append("frontier import corpus identity or no-effect truth drifted")
    manifests = value.get("manifest_failures", [])
    artifacts = value.get("artifact_attacks", [])
    states = value.get("state_change_cases", [])
    flows = value.get("local_flow_cases", [])
    mutations = value.get("round_trip_mutations", [])
    observed = {
        "manifest_failures": len(manifests),
        "artifact_attacks": len(artifacts),
        "state_change_cases": len(states),
        "local_flow_cases": len(flows),
        "round_trip_mutations": len(mutations),
        "total": len(manifests) + len(artifacts) + len(states) + len(flows) + len(mutations),
    }
    if observed != value.get("expanded_case_counts") or observed != {
        "manifest_failures": 14,
        "artifact_attacks": 12,
        "state_change_cases": 8,
        "local_flow_cases": 7,
        "round_trip_mutations": 9,
        "total": 50,
    }:
        failures.append("frontier import corpus cardinality drifted")
    if {item.get("case_id") for item in manifests} != REQUIRED_MANIFEST_FAILURES:
        failures.append("manifest failure closure drifted")
    if {item.get("case_id") for item in artifacts} != REQUIRED_ARTIFACT_ATTACKS:
        failures.append("artifact attack closure drifted")
    if {item.get("case_id") for item in states} != REQUIRED_STATE_CHANGES:
        failures.append("state-change closure drifted")
    if {item.get("case_id") for item in flows} != REQUIRED_FLOWS:
        failures.append("local-flow closure drifted")
    if mutations != REQUIRED_MUTATIONS:
        failures.append("round-trip mutation closure drifted")
    if any(item.get("expected") not in {
        "reject_manifest",
        "quarantined_nonexecuting",
        "rejected_before_review",
        "rejected_nonpersisting",
        "proposal_requires_trusted_validation",
        "unknown_blocked",
        "proposal_requires_exact_write_preview",
        "proposal_requires_fresh_grant",
        "quarantine_all_steps",
        "unknown_blocked_with_disagreement",
    } for item in [*manifests, *artifacts, *states]):
        failures.append("unknown hostile-case disposition")
    engine = ENGINE_PATH.read_text(encoding="utf-8")
    contract = CONTRACT_PATH.read_text(encoding="utf-8")
    host = HOST_PATH.read_text(encoding="utf-8")
    required_source = {
        "external_content_untrusted": contract,
        "authority_granted": contract,
        "completion_credit": contract,
        "outbound_network_required": contract,
        "canonical_state_changed: false": engine,
        "grant_count: 0": engine,
        "tool_call_count: 0": engine,
        "file_write_count: 0": engine,
        "completion_credit_count: 0": engine,
        "outbound_network_used: false": engine,
        "classify_task(route.work_packet, route.intent)": host,
        "ToolDispatcher::new(registry).dispatch": host,
        "fresh_grant_pending": host,
        "exact_preimage_pending": host,
        "trusted_validation_pending": host,
        "evidence_assignment_pending": host,
        "user_approval_pending": host,
        "DirectoryFrontierImportCheckpointStore": host,
        "applied_effect_count: 0": host,
    }
    for marker, source in required_source.items():
        if marker not in source:
            failures.append(f"frontier import source marker absent: {marker}")
    return failures


def main() -> int:
    failures = validate(read())
    if failures:
        print("frontier import contract failed:")
        for failure in failures:
            print(f"- {failure}")
        return 1
    print("frontier import contract validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
