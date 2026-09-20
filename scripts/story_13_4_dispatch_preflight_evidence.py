#!/usr/bin/env python3
"""Build deterministic CTX-FIT/CTX-DISPATCH evidence without native qualification."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-13/story-13.4"
RAW_PATH: Final = EVIDENCE_DIR / "dispatch-preflight-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "dispatch-preflight-report.json"
COMMANDS: Final = (
    ("cargo", "test", "-p", "agentmage-kernel-engine", "--locked", "dispatch_preflight_", "--", "--test-threads=1"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "--locked", "prepared_request_mutation_stale_binding_and_token_drift_send_no_generation_bytes", "--", "--test-threads=1"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "--locked", "orchestration_plan_and_tool_heavy_unicode_render_reconcile_once", "--", "--test-threads=1"),
    ("cargo", "test", "-p", "agentmage-host", "--locked", "model_preflight_refusals_have_closed_codes_and_recovery", "--", "--test-threads=1"),
    ("cargo", "test", "-p", "agentmage-platform-linux-inference", "--locked", "--no-run"),
)
MARKERS: Final = (
    "dispatch_preflight_accepts_exact_fit_and_refuses_one_token_over_before_stream ... ok",
    "dispatch_preflight_refuses_7000_plus_2048_against_8192_and_invalid_margin ... ok",
    "prepared_request_mutation_stale_binding_and_token_drift_send_no_generation_bytes ... ok",
    "orchestration_plan_and_tool_heavy_unicode_render_reconcile_once ... ok",
    "model_preflight_refusals_have_closed_codes_and_recovery ... ok",
    "Executable tests/muse_live_evaluation.rs",
    "Executable tests/muse_live_install.rs",
)
SOURCE_PATHS: Final = (
    "kernel/contracts/src/lib.rs",
    "kernel/contracts/src/model.rs",
    "kernel/engine/src/model_orchestration_profile.rs",
    "kernel/engine/src/model_runtime.rs",
    "kernel/engine/src/runtime_loop.rs",
    "platforms/linux-inference/src/llama_server_driver.rs",
    "platforms/linux-inference/src/native_model_adapter.rs",
    "platforms/linux-inference/tests/muse_live_evaluation.rs",
    "platforms/linux-inference/tests/muse_live_install.rs",
    "shells/host/src/engineering_model.rs",
    "shells/host/src/engineering_runtime.rs",
    "scripts/story_13_4_dispatch_preflight_evidence.py",
    "tests/test_story_13_4_dispatch_preflight_evidence.py",
)
CTX_FIT: Final = (
    "exact-fit",
    "one-token-over",
    "7000-input-plus-2048-output-against-8192",
    "zero-margin",
    "checked-integer-bounds",
    "tool-heavy-unicode-special-token-render",
    "orchestration-plan-reconciliation",
)
CTX_DISPATCH: Final = (
    "mandatory-prepared-request",
    "modified-rendered-bytes",
    "stale-process-load-slot-binding",
    "dispatch-time-token-drift",
    "fresh-preparation-for-retry-summary-reconnect",
    "zero-generation-bytes-on-refusal",
)
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def resolve_revision(revision: str) -> str:
    result = subprocess.run(("git", "rev-parse", "--verify", f"{revision}^{{commit}}"), cwd=ROOT, text=True, capture_output=True, check=False, timeout=10)
    value = result.stdout.strip()
    if result.returncode != 0 or not REVISION.fullmatch(value):
        raise ValueError("dispatch-preflight source revision is unavailable")
    return value


def git_file(revision: str, relative: str) -> bytes:
    result = subprocess.run(("git", "show", f"{revision}:{relative}"), cwd=ROOT, capture_output=True, check=False, timeout=10)
    if result.returncode != 0:
        raise ValueError(f"committed dispatch-preflight source is absent: {relative}")
    return result.stdout


def capture() -> tuple[str, int]:
    chunks = []
    for command in COMMANDS:
        result = subprocess.run(command, cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=False, timeout=900)
        chunks.append(f"$ {' '.join(command)}\n{result.stdout}")
        if result.returncode != 0:
            return "\n".join(chunks).rstrip() + "\n", result.returncode
    return "\n".join(chunks).rstrip() + "\n", 0


def validate_raw(value: str) -> list[str]:
    failures = [f"dispatch-preflight results missing marker: {marker}" for marker in MARKERS if marker not in value]
    for prohibited in ("test result: FAILED", "error: could not compile", "panicked at"):
        if prohibited in value:
            failures.append(f"dispatch-preflight results contain prohibited marker: {prohibited}")
    return failures


def expected_report(source_revision: str, raw: bytes) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "agentmage-story-13-4-dispatch-preflight-evidence",
        "story_id": "13.4",
        "task_id": "13.4.5",
        "source_revision": source_revision,
        "source_sha256": {path: sha256_bytes(git_file(source_revision, path)) for path in SOURCE_PATHS},
        "raw_results": {"path": RAW_PATH.relative_to(ROOT).as_posix(), "sha256": sha256_bytes(raw)},
        "commands": [" ".join(command) for command in COMMANDS],
        "ctx_fit": list(CTX_FIT),
        "ctx_dispatch": list(CTX_DISPATCH),
        "stable_refusal_codes": [
            "model.prepared-request.capacity-exceeded",
            "model.prepared-request.mismatch",
            "model.prepared-request.stale-binding",
            "model.prepared-request.token-drift",
        ],
        "recovery": {
            "source_reduction_visible": True,
            "compaction_requires_g2_admission": True,
            "linked_thread_requires_checked_summary_and_sources": True,
            "approved_profile_only": True,
            "unchanged_retry": False,
            "server_context_shift": False,
            "silent_fallback": False,
        },
        "claims": {
            "local_contract_complete": True,
            "native_boundary_campaign_executed": False,
            "native_model_loaded": False,
            "model_profile_enabled": False,
            "platform_qualified": False,
            "release_claim": "none",
        },
        "status": "PASS_LOCAL_FIXTURES_BLOCKED_NATIVE_BOUNDARY",
        "remaining_prerequisite": "Run CTX-FIT and CTX-DISPATCH against the exact separately authorized native model/runtime/venue tuple and retain endpoint token reconciliation plus zero-generation refusal evidence.",
        "limitations": [
            "Deterministic fake-runtime counts prove the common gate but do not qualify a native model or platform.",
            "Ignored live Muse targets compile against the mandatory preflight contract; this evidence run does not execute them.",
            "No demo service, model configuration, artifact, or USTE process is changed.",
        ],
    }


def validate_report(report: Any, source_revision: str, raw: bytes) -> list[str]:
    try:
        expected = expected_report(source_revision, raw)
    except ValueError as error:
        return [str(error)]
    if report != expected:
        return ["dispatch-preflight report is stale, incomplete, or widened"]
    hashes = report.get("source_sha256", {})
    if set(hashes) != set(SOURCE_PATHS) or any(not SHA256.fullmatch(str(value)) for value in hashes.values()):
        return ["dispatch-preflight source closure is invalid"]
    return []


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-revision", default="HEAD")
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    failures: list[str] = []
    if args.write:
        try:
            revision = resolve_revision(args.source_revision)
        except ValueError as error:
            print(error, file=sys.stderr)
            return 1
        raw_text, returncode = capture()
        raw = raw_text.encode()
        failures += validate_raw(raw_text)
        if returncode == 0 and not failures:
            EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
            RAW_PATH.write_bytes(raw)
            REPORT_PATH.write_text(json.dumps(expected_report(revision, raw), indent=2, sort_keys=True) + "\n", encoding="utf-8")
    else:
        try:
            raw = RAW_PATH.read_bytes()
            raw_text = raw.decode()
            report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
            failures.append(f"cannot read dispatch-preflight evidence: {error}")
        else:
            try:
                revision = resolve_revision(str(report.get("source_revision", "")))
            except ValueError as error:
                failures.append(str(error))
                revision = ""
            failures += validate_raw(raw_text)
            if revision:
                failures += validate_report(report, revision, raw)
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    print("CTX-FIT and CTX-DISPATCH local contract pass; native boundary remains open")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
