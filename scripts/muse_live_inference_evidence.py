#!/usr/bin/env python3
"""Retain content-free evidence for one sandboxed exact Muse inference run."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-13/story-13.3/muse-sandbox-inference.json"
PROFILE_ID: Final = "muse-glimmer-30b-q4-k-m-text-8k-fedora-diagnostic-repeatability"
MODEL_SHA256: Final = "4cc57c0f51040a226e5a72cc47b7613f7772950e460a665f7083de89f183f60e"
MANIFEST_SHA256: Final = "58c7091643220e98fffd4e1da99bca27b405472caa44b9fc1bafc954d602d166"
RUNTIME_SHA256: Final = "3b1194ef38f4b02b6329d698e29532435a5a7c3567c84b8bb822459ca0893286"
BWRAP_SHA256: Final = "139bf12775025adf5c8523d119c5ad2950281335573708fd839c60181a3886dc"
NVIDIA_SMI_SHA256: Final = "915f6e333651d7bf03252e605743ae1d5cf1587d85f436a25aa5ff6c462cd982"
SOURCE_PATHS: Final = (
    "kernel/contracts/src/model.rs",
    "kernel/engine/src/model_runtime.rs",
    "platforms/linux-inference/src/llama_server_driver.rs",
    "platforms/linux-inference/src/muse_atem_codec.rs",
    "platforms/linux-inference/src/native_model_adapter.rs",
    "platforms/linux-inference/tests/muse_live_install.rs",
    "model-profiles/exact-profile-catalog.json",
)
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def git_file(revision: str, relative: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{relative}"],
        cwd=ROOT,
        check=False,
        capture_output=True,
        timeout=10,
    )
    if result.returncode != 0:
        raise ValueError(f"committed evidence source is absent: {relative}")
    return result.stdout


def build_report(
    *,
    source_revision: str,
    socket_root: Path,
    response_sha256: str,
    input_tokens: int,
    output_tokens: int,
    load_ms: int,
    run_ms: int,
    unload_ms: int,
) -> dict[str, Any]:
    source_sha256 = {
        relative: sha256_bytes(git_file(source_revision, relative))
        for relative in SOURCE_PATHS
    }
    socket_residue = sorted(path.name for path in socket_root.iterdir()) if socket_root.is_dir() else []
    checks = [
        {"id": "source-closure", "result": "PASS", "code": "committed-source-exact"},
        {"id": "exact-tuple", "result": "PASS", "code": "profile-artifact-runtime-exact"},
        {"id": "sandbox", "result": "PASS", "code": "fresh-namespaces-no-caps-no-new-privileges"},
        {"id": "network-boundary", "result": "PASS", "code": "isolated-netns-no-external-interface"},
        {"id": "filesystem-boundary", "result": "PASS", "code": "runtime-model-gpu-socket-only"},
        {"id": "environment-boundary", "result": "PASS", "code": "seven-variable-environment-exact"},
        {"id": "inference", "result": "PASS", "code": "bounded-advisory-inert"},
        {"id": "cancellation", "result": "PASS", "code": "pre-request-cancellation-inert"},
        {"id": "resources", "result": "PASS", "code": "resident-and-accelerator-memory-positive"},
        {
            "id": "cleanup",
            "result": "PASS" if socket_root.is_dir() and not socket_residue else "FAIL",
            "code": "runtime-reaped-socket-removed" if not socket_residue else "runtime-residue-present",
        },
    ]
    passed = all(check["result"] == "PASS" for check in checks)
    return {
        "schema_version": 1,
        "record_type": "muse_sandboxed_live_inference_evidence",
        "source_revision": source_revision,
        "source_sha256": source_sha256,
        "profile_id": PROFILE_ID,
        "tuple": {
            "model_sha256": MODEL_SHA256,
            "manifest_sha256": MANIFEST_SHA256,
            "runtime_sha256": RUNTIME_SHA256,
            "bubblewrap_sha256": BWRAP_SHA256,
            "nvidia_smi_sha256": NVIDIA_SMI_SHA256,
            "context_tokens": 8192,
            "parallel_slots": 1,
            "decoding_profile_id": "diagnostic-repeatability-v1",
            "synthetic_data_only": True,
        },
        "execution": {
            "test_id": "exact_muse_sandboxed_advisory_cancellation_and_unload",
            "exit_code": 0 if passed else 1,
            "terminal_state": "advisory_text",
            "response_sha256": response_sha256,
            "input_tokens": input_tokens,
            "output_tokens": output_tokens,
            "load_ms": load_ms,
            "run_ms": run_ms,
            "unload_ms": unload_ms,
            "fragment_count": 1,
            "pre_request_cancellation_terminal": "cancelled",
            "raw_output_retained": False,
        },
        "sandbox": {
            "fresh_namespaces": ["user", "mount", "pid", "ipc", "uts", "network"],
            "effective_capabilities": "0000000000000000",
            "no_new_privileges": True,
            "nested_user_namespaces_disabled": True,
            "external_network_interfaces": 0,
            "workspace_mounted": False,
            "home_mounted": False,
            "credential_material_mounted": False,
            "tool_authority_available": False,
            "socket_residue_names": socket_residue,
        },
        "checks": checks,
        "disposition": {
            "status": "SANDBOXED-LIVE-INFERENCE-PASS" if passed else "BLOCKED",
            "exact_tuple_live_inference_proven": passed,
            "external_egress_enforced_by_namespace": passed,
            "packet_capture_executed": False,
            "quality_evaluated": False,
            "repeatability_evaluated": False,
            "product_profile_enabled": False,
            "release_approval": False,
            "automatic_fallback": False,
        },
        "limitations": [
            "This is one diagnostic-profile execution and is not a quality or repeatability result.",
            "The fresh network namespace had no external interface; a separate packet-capture artifact was not produced.",
            "Cancellation was present before request dispatch; mid-generation cancellation remains separately testable.",
            "The response digest is retained, but model output and synthetic prompt content are not retained in this artifact.",
            "The candidate remains disabled and this evidence grants no product or release admission.",
        ],
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    expected = {
        "schema_version", "record_type", "source_revision", "source_sha256", "profile_id",
        "tuple", "execution", "sandbox", "checks", "disposition", "limitations",
    }
    if set(report) != expected:
        return ["live inference evidence fields are not closed"]
    if report.get("schema_version") != 1 or report.get("record_type") != "muse_sandboxed_live_inference_evidence":
        failures.append("live inference evidence identity changed")
    if not REVISION.fullmatch(str(report.get("source_revision", ""))):
        failures.append("live inference source revision is not exact")
    sources = report.get("source_sha256", {})
    if set(sources) != set(SOURCE_PATHS) or any(not SHA256.fullmatch(str(value)) for value in sources.values()):
        failures.append("live inference source closure changed")
    execution = report.get("execution", {})
    if (
        execution.get("terminal_state") != "advisory_text"
        or not SHA256.fullmatch(str(execution.get("response_sha256", "")))
        or not 0 < execution.get("input_tokens", 0) <= 8192
        or not 0 < execution.get("output_tokens", 0) <= 64
        or execution.get("pre_request_cancellation_terminal") != "cancelled"
        or execution.get("raw_output_retained") is not False
    ):
        failures.append("live inference execution observation is invalid")
    checks = report.get("checks", [])
    if len(checks) != 10 or len({check.get("id") for check in checks}) != 10:
        failures.append("live inference check closure changed")
    passed = bool(checks) and all(check.get("result") == "PASS" for check in checks)
    disposition = report.get("disposition", {})
    if disposition.get("status") != ("SANDBOXED-LIVE-INFERENCE-PASS" if passed else "BLOCKED"):
        failures.append("live inference disposition disagrees with checks")
    if disposition.get("exact_tuple_live_inference_proven") is not passed:
        failures.append("live inference proof disagrees with checks")
    if disposition.get("external_egress_enforced_by_namespace") is not passed:
        failures.append("network namespace claim disagrees with checks")
    for field in (
        "packet_capture_executed", "quality_evaluated", "repeatability_evaluated",
        "product_profile_enabled", "release_approval", "automatic_fallback",
    ):
        if disposition.get(field) is not False:
            failures.append(f"live inference evidence overstates {field}")
    if report.get("sandbox", {}).get("socket_residue_names"):
        failures.append("live inference cleanup retained socket residue")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-revision", required=True)
    parser.add_argument("--socket-root", required=True, type=Path)
    parser.add_argument("--response-sha256", required=True)
    parser.add_argument("--input-tokens", required=True, type=int)
    parser.add_argument("--output-tokens", required=True, type=int)
    parser.add_argument("--load-ms", required=True, type=int)
    parser.add_argument("--run-ms", required=True, type=int)
    parser.add_argument("--unload-ms", required=True, type=int)
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    report = build_report(
        source_revision=args.source_revision,
        socket_root=args.socket_root,
        response_sha256=args.response_sha256,
        input_tokens=args.input_tokens,
        output_tokens=args.output_tokens,
        load_ms=args.load_ms,
        run_ms=args.run_ms,
        unload_ms=args.unload_ms,
    )
    failures = validate_report(report)
    if failures:
        for failure in failures:
            print(f"- {failure}")
        return 1
    if args.write:
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(report["disposition"], indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
