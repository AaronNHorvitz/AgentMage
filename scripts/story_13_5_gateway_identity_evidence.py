#!/usr/bin/env python3
"""Build the Story 13.5 candidate-neutral gateway identity evidence."""

from __future__ import annotations

import argparse, hashlib, json, subprocess, sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-13/story-13.5"
RAW_PATH: Final = EVIDENCE_DIR / "gateway-identity-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "gateway-identity-report.json"
COMMANDS: Final = (
    ("cargo", "test", "-p", "agentmage-kernel-engine", "story_13_5", "--locked"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "model_gateway::tests", "--locked"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "every_descriptive_family_is_sealed_as_non_authoritative", "--locked"),
    ("node", "--test", "tests/test_disabled_gateway_candidate_schema.mjs"),
    ("cargo", "clippy", "-p", "agentmage-kernel-engine", "--all-targets", "--all-features", "--locked", "--", "-D", "warnings"),
)
MARKERS: Final = (
    "story_13_5_all_endpoint_classes_remain_disabled_without_fallback ... ok",
    "story_13_5_unknown_operator_and_partial_remote_tuple_fail_before_route ... ok",
    "story_13_5_cross_class_credential_and_nested_identity_substitution_fail ... ok",
    "story_13_5_every_independent_identity_changes_whole_tuple_digest ... ok",
    "exact_profiles_and_explicit_routes_are_digest_bound ... ok",
    "strict_local_can_never_fall_back_remotely ... ok",
    "every_json_protocol_uses_a_closed_shape_and_extracts_text ... ok",
    "remote_profile_rejects_http_embedded_credentials_and_redirects ... ok",
    "every_descriptive_family_is_sealed_as_non_authoritative ... ok",
    "disabled gateway schema accepts all four complete candidate classes",
    "disabled gateway schema rejects activation, fallback, missing identity, and widening",
    "Finished `dev` profile",
)
PATHS: Final = (
    "kernel/contracts/src/model.rs", "kernel/contracts/src/engineering_records.rs",
    "kernel/engine/src/gateway_candidate_identity.rs", "kernel/engine/src/model_gateway.rs",
    "kernel/engine/src/authority.rs", "schemas/model/disabled-gateway-candidate.schema.json",
    "docs/architecture/candidate-neutral-model-gateway-identity.md",
    "scripts/story_13_5_gateway_identity_evidence.py",
    "tests/test_story_13_5_gateway_identity_evidence.py",
    "tests/test_disabled_gateway_candidate_schema.mjs",
)
TRUTH: Final = {
    "candidate_tuple_identity_complete": True,
    "model_request_event_proposal_usage_error_cancellation_contracts_present": True,
    "all_four_endpoint_classes_represented": True,
    "unknown_operator_refused": True,
    "partial_or_cross_class_tuple_refused": True,
    "model_proposal_has_authority": False,
    "enabled_candidate_count": 0,
    "selected_route_count": 0,
    "automatic_fallback_enabled": False,
    "rv54_identity_slice_complete": True,
    "live_endpoint_qualification_complete": False,
    "routing_rv54_slice_complete": False,
    "story_completion_claim": True,
    "sprint_completion_claim": False,
    "release_claim": "none",
}

def sha256(path: Path) -> str: return hashlib.sha256(path.read_bytes()).hexdigest()
def artifact(path: str) -> dict[str, Any]:
    value = ROOT / path
    return {"path": path, "byte_length": value.stat().st_size, "sha256": sha256(value)}

def expected_report() -> dict[str, Any]:
    return {
        "schema_version": 1, "record_type": "agentmage-story-13-5-gateway-identity-evidence",
        "story_id": "13.5", "protocol_id": "RV-54", "generated_on": "2026-08-31",
        "status": "PASS_LOCAL_IDENTITY_DISABLED_BASELINE",
        "commands": [" ".join(command) for command in COMMANDS], "required_markers": list(MARKERS),
        "independent_identities": ["model", "runtime-adapter", "protocol-codec", "endpoint", "route", "operator", "credential-reference", "qualification"],
        "endpoint_classes": ["strict_local", "local_network_private", "remote_private", "remote_managed"],
        "artifacts": [artifact(path) for path in PATHS] + [artifact(RAW_PATH.relative_to(ROOT).as_posix())],
        "product_truth": dict(TRUTH),
        "limitations": [
            "identity admission deliberately activates no endpoint and executes no inference",
            "live endpoint qualification and routing/fallback RV-54 scenarios remain later-story gates",
            "cross-platform installed-product and release completion are not claimed",
        ],
    }

def render(value: dict[str, Any]) -> str: return json.dumps(value, indent=2, sort_keys=True) + "\n"
def validate_sources() -> list[str]: return [f"missing retained source: {path}" for path in PATHS if not (ROOT / path).is_file()]
def validate_raw(value: str) -> list[str]:
    failures = [f"raw results missing marker: {marker}" for marker in MARKERS if marker not in value]
    for prohibited in ("test result: FAILED", "not ok", "error: could not compile", "warning:"):
        if prohibited in value: failures.append(f"raw results contain prohibited marker: {prohibited}")
    return failures
def validate_report(value: Any) -> list[str]: return [] if value == expected_report() else ["Story 13.5 report is stale or widened"]
def capture() -> tuple[str, int]:
    chunks = []
    for command in COMMANDS:
        result = subprocess.run(command, cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=False)
        chunks.append(f"$ {' '.join(command)}\n{result.stdout}")
        if result.returncode: return "\n".join(chunks), result.returncode
    return "\n".join(chunks), 0

def main() -> int:
    parser = argparse.ArgumentParser(); parser.add_argument("--write", action="store_true"); args = parser.parse_args()
    failures = validate_sources()
    if args.write:
        raw, returncode = capture(); EVIDENCE_DIR.mkdir(parents=True, exist_ok=True); RAW_PATH.write_text(raw, encoding="utf-8")
        failures += validate_raw(raw)
        if returncode == 0 and not failures: REPORT_PATH.write_text(render(expected_report()), encoding="utf-8")
    else:
        try: raw = RAW_PATH.read_text(encoding="utf-8"); report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error: failures.append(f"cannot read retained evidence: {error}")
        else: failures += validate_raw(raw) + validate_report(report)
    if failures: print("\n".join(failures), file=sys.stderr); return 1
    print("Story 13.5 candidate-neutral gateway identity and RV-54 slice validated"); return 0

if __name__ == "__main__": raise SystemExit(main())
