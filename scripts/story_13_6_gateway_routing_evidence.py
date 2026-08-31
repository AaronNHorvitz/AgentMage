#!/usr/bin/env python3
"""Build the local Story 13.6 codec, routing, and no-silent-fallback evidence."""

from __future__ import annotations
import argparse, hashlib, json, subprocess, sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-13/story-13.6"
RAW_PATH: Final = EVIDENCE_DIR / "gateway-routing-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "gateway-routing-report.json"
COMMANDS: Final = (
    ("cargo", "test", "-p", "agentmage-kernel-engine", "story_13_6", "--locked"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "model_gateway::tests", "--locked"),
    ("cargo", "clippy", "-p", "agentmage-kernel-engine", "--all-targets", "--all-features", "--locked", "--", "-D", "warnings"),
)
MARKERS: Final = (
    "story_13_6_codec_preserves_supported_ordered_semantics ... ok",
    "story_13_6_unknown_malformed_and_unsupported_streams_fail_visibly ... ok",
    "story_13_6_route_selection_audits_health_quota_limits_and_disclosure ... ok",
    "story_13_6_fallback_is_denied_by_default_and_requires_exact_equivalent_policy ... ok",
    "story_13_6_health_quota_and_control_drift_cannot_silently_fallback ... ok",
    "exact_profiles_and_explicit_routes_are_digest_bound ... ok",
    "strict_local_can_never_fall_back_remotely ... ok",
    "every_json_protocol_uses_a_closed_shape_and_extracts_text ... ok",
    "remote_profile_rejects_http_embedded_credentials_and_redirects ... ok",
    "Finished `dev` profile",
)
PATHS: Final = (
    "kernel/engine/src/gateway_candidate_identity.rs", "kernel/engine/src/gateway_routing.rs",
    "kernel/engine/src/model_gateway.rs", "docs/architecture/gateway-codec-routing-fallback.md",
    "scripts/story_13_6_gateway_routing_evidence.py", "tests/test_story_13_6_gateway_routing_evidence.py",
)
TRUTH: Final = {
    "versioned_codec_capability_contract_complete": True, "unknown_semantic_fails_visibly": True,
    "ordered_stream_validation_complete": True, "deterministic_route_audit_complete": True,
    "health_resource_quota_cost_disclosure_gates_complete": True,
    "fallback_enabled_by_default": False, "silent_local_to_remote_transition_count": 0,
    "silent_cross_remote_transition_count": 0, "rv54_routing_slice_complete": True,
    "live_remote_endpoint_executed": False, "credential_value_accessed": False,
    "story_completion_claim": True, "sprint_completion_claim": False, "release_claim": "none",
}
def sha256(path: Path) -> str: return hashlib.sha256(path.read_bytes()).hexdigest()
def artifact(path: str) -> dict[str, Any]:
    value = ROOT / path; return {"path": path, "byte_length": value.stat().st_size, "sha256": sha256(value)}
def expected_report() -> dict[str, Any]:
    return {"schema_version": 1, "record_type": "agentmage-story-13-6-gateway-routing-evidence", "story_id": "13.6", "protocol_id": "RV-54", "generated_on": "2026-08-31", "status": "PASS_LOCAL_CODEC_ROUTING_CONTRACT", "commands": [" ".join(command) for command in COMMANDS], "required_markers": list(MARKERS), "codec_semantics": ["message-parts", "streaming", "structured-output", "tool-proposals", "usage", "cancellation", "health", "concurrency", "typed-failures"], "routing_gates": ["user-profile", "classification", "disclosure", "role", "capability", "health", "resource", "quota", "cost", "platform", "context", "concurrency", "qualification", "policy", "equivalent-controls"], "artifacts": [artifact(path) for path in PATHS] + [artifact(RAW_PATH.relative_to(ROOT).as_posix())], "product_truth": dict(TRUTH), "limitations": ["deterministic fixtures exercise no live remote endpoint and access no credential value", "installed cross-platform and production endpoint qualification remain separate release gates", "a protocol completion event is not represented as workflow completion"]}
def render(value: dict[str, Any]) -> str: return json.dumps(value, indent=2, sort_keys=True) + "\n"
def validate_sources() -> list[str]: return [f"missing retained source: {path}" for path in PATHS if not (ROOT / path).is_file()]
def validate_raw(value: str) -> list[str]:
    failures = [f"raw results missing marker: {marker}" for marker in MARKERS if marker not in value]
    for prohibited in ("test result: FAILED", "error: could not compile", "warning:"):
        if prohibited in value: failures.append(f"raw results contain prohibited marker: {prohibited}")
    return failures
def validate_report(value: Any) -> list[str]: return [] if value == expected_report() else ["Story 13.6 report is stale or widened"]
def capture() -> tuple[str, int]:
    chunks=[]
    for command in COMMANDS:
        result=subprocess.run(command,cwd=ROOT,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,check=False); chunks.append(f"$ {' '.join(command)}\n{result.stdout}")
        if result.returncode: return "\n".join(chunks),result.returncode
    return "\n".join(chunks),0
def main() -> int:
    parser=argparse.ArgumentParser(); parser.add_argument("--write",action="store_true"); args=parser.parse_args(); failures=validate_sources()
    if args.write:
        raw,returncode=capture(); EVIDENCE_DIR.mkdir(parents=True,exist_ok=True); RAW_PATH.write_text(raw,encoding="utf-8"); failures+=validate_raw(raw)
        if returncode==0 and not failures: REPORT_PATH.write_text(render(expected_report()),encoding="utf-8")
    else:
        try: raw=RAW_PATH.read_text(encoding="utf-8"); report=json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        except (OSError,json.JSONDecodeError) as error: failures.append(f"cannot read retained evidence: {error}")
        else: failures+=validate_raw(raw)+validate_report(report)
    if failures: print("\n".join(failures),file=sys.stderr); return 1
    print("Story 13.6 gateway codec, routing, fallback, and RV-54 slice validated"); return 0
if __name__=="__main__": raise SystemExit(main())
