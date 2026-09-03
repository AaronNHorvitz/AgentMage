#!/usr/bin/env python3
"""Validate and retain the non-activating Story 49.2 adapter matrix."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
MATRIX: Final = ROOT / "model-profiles/routing/alternate-runtime-matrix-v1.json"
REPORT: Final = ROOT / "artifacts/sprints/sprint-49/story-49.2/adapter-evaluation-report.json"
FAMILIES: Final = [
    "llama_cpp", "ollama", "lm_studio", "vllm", "sglang", "tgi",
    "ray_serve_llm", "kserve", "openai_responses_compatible",
    "anthropic_messages_compatible",
]
SEMANTICS: Final = [
    "message_parts", "structured_output", "tool_proposals", "stream_events",
    "usage", "cancellation", "errors", "health", "concurrency", "version_behavior",
]
PARITY_CASES: Final = [
    "field_preservation", "ordering", "backpressure", "cancellation",
    "malformed_events", "resource_limits", "typed_error_mapping",
    "version_change_staleness",
]
SOURCES: Final = (
    "kernel/engine/src/model_adapter_evaluation.rs",
    "kernel/engine/src/gateway_routing.rs",
    "model-profiles/routing/alternate-runtime-matrix-v1.json",
    "docs/verification/story-49-2-local-results.md",
    "scripts/alternate_runtime_matrix.py",
    "tests/test_alternate_runtime_matrix.py",
)


def load(path: Path = MATRIX) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def validate(value: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict):
        return ["matrix is not an object"]
    expected_keys = {
        "schema_version", "record_type", "activation_authority", "enabled_route_count",
        "silent_fallback", "canonical_semantics", "semantic_dispositions",
        "fake_parity_cases", "fake_parity_status", "performance_status", "rv54_status",
        "candidates", "staleness_policy",
    }
    if set(value) != expected_keys:
        failures.append("matrix fields drifted")
    if (
        value.get("schema_version") != 1
        or value.get("record_type") != "agentmage_alternate_runtime_evaluation_matrix"
        or value.get("activation_authority") is not False
        or value.get("enabled_route_count") != 0
        or value.get("silent_fallback") is not False
    ):
        failures.append("matrix gained activation, route, or fallback authority")
    if value.get("canonical_semantics") != SEMANTICS:
        failures.append("canonical semantic inventory drifted")
    dispositions = value.get("semantic_dispositions", {})
    if set(dispositions) != {"unverified_live_runtime", "unavailable"}:
        failures.append("semantic dispositions drifted")
    else:
        for status, expected in (("unverified_live_runtime", "unverified"), ("unavailable", "unavailable")):
            if set(dispositions[status]) != set(SEMANTICS) or set(dispositions[status].values()) != {expected}:
                failures.append(f"semantic map is incomplete or widened: {status}")
    candidates = value.get("candidates")
    candidate_fields = {
        "candidate_id", "family", "exact_version", "protocol_surfaces", "semantic_status",
        "disposition", "gaps", "remediation", "platform_qualification", "support_claim",
        "route_enabled",
    }
    if not isinstance(candidates, list) or [item.get("family") for item in candidates if isinstance(item, dict)] != FAMILIES:
        failures.append("candidate family inventory drifted")
    else:
        for index, candidate in enumerate(candidates):
            if set(candidate) != candidate_fields:
                failures.append(f"candidate fields drifted: {index}")
                continue
            if (
                not isinstance(candidate["candidate_id"], str)
                or not candidate["protocol_surfaces"]
                or candidate["semantic_status"] not in dispositions
                or candidate["disposition"] not in {"candidate_unqualified", "unavailable"}
                or not candidate["gaps"]
                or not isinstance(candidate["remediation"], str)
                or candidate["platform_qualification"] is not False
                or candidate["support_claim"] is not False
                or candidate["route_enabled"] is not False
            ):
                failures.append(f"candidate overclaims or lacks a disposition: {index}")
        if candidates and (
            candidates[0].get("exact_version") != "b10333"
            or candidates[0].get("semantic_status") != "unverified_live_runtime"
            or any(item.get("exact_version") is not None for item in candidates[1:])
            or any(item.get("semantic_status") != "unavailable" for item in candidates[1:])
        ):
            failures.append("exact-version availability truth drifted")
    if value.get("fake_parity_cases") != PARITY_CASES or value.get("fake_parity_status") != "pass_contract_only":
        failures.append("fake parity coverage drifted")
    if value.get("performance_status") != "unavailable_no_live_candidate":
        failures.append("performance evidence overclaim")
    if value.get("rv54_status") != "pass_identity_and_no_silent_fallback_only":
        failures.append("RV-54 scope drifted")
    policy = value.get("staleness_policy", {})
    if (
        policy.get("identity_inputs") != ["candidate_id", "family", "exact_version", "protocol_surfaces"]
        or policy.get("version_change_invalidates_prior_evidence") is not True
        or policy.get("codec_change_invalidates_prior_evidence") is not True
        or policy.get("endpoint_change_invalidates_prior_evidence") is not True
    ):
        failures.append("staleness policy drifted")
    return failures


def git_bytes(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, capture_output=True,
        check=False, timeout=30,
    )
    if result.returncode:
        raise ValueError(f"evidence source unavailable: {path}")
    return result.stdout


def expected_report(revision: str) -> dict[str, Any]:
    sources = {path: git_bytes(revision, path) for path in SOURCES}
    matrix = json.loads(sources["model-profiles/routing/alternate-runtime-matrix-v1.json"])
    failures = validate(matrix)
    return {
        "schema_version": 1,
        "record_type": "agentmage-story-49-2-adapter-evaluation-report",
        "source_revision": revision,
        "source_sha256": {path: hashlib.sha256(data).hexdigest() for path, data in sources.items()},
        "candidate_family_count": len(matrix.get("candidates", [])),
        "canonical_semantic_count": len(matrix.get("canonical_semantics", [])),
        "fake_parity_case_count": len(matrix.get("fake_parity_cases", [])),
        "live_candidate_count": 0,
        "enabled_route_count": 0,
        "supported_adapter_count": 0,
        "silent_fallback": False,
        "failures": failures,
        "status": "PASS_NON_ACTIVATING_CONTRACT" if not failures else "FAIL",
        "limitations": [
            "Fake parity proves the canonical contract only, not any named runtime.",
            "No unresolved candidate is assigned an implementation or version.",
            "No adapter, model, endpoint, route, platform, or fallback is supported or enabled.",
            "Live parity, performance, lifecycle, removal, and platform evidence remain absent.",
        ],
    }


def validate_report(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["report is not an object"]
    revision = value.get("source_revision")
    if not isinstance(revision, str) or len(revision) != 40 or any(char not in "0123456789abcdef" for char in revision):
        return ["source revision invalid"]
    try:
        return [] if value == expected_report(revision) else ["report is stale, incomplete, or widened"]
    except (ValueError, json.JSONDecodeError) as error:
        return [str(error)]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    revision = subprocess.check_output(["git", "rev-parse", arguments.source_revision], cwd=ROOT, text=True).strip()
    failures = validate(load())
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    if arguments.write:
        REPORT.parent.mkdir(parents=True, exist_ok=True)
        REPORT.write_text(json.dumps(expected_report(revision), indent=2, sort_keys=True) + "\n")
    if REPORT.exists():
        report_failures = validate_report(json.loads(REPORT.read_text(encoding="utf-8")))
        if report_failures:
            print("\n".join(report_failures), file=sys.stderr)
            return 1
    print("Story 49.2 alternate-runtime matrix passed without activation")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
