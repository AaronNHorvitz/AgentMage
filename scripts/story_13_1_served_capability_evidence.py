#!/usr/bin/env python3
"""Build deterministic CTX-SERVED evidence without claiming native qualification."""

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
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-13/story-13.1"
RAW_PATH: Final = EVIDENCE_DIR / "served-capability-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "served-capability-report.json"
COMMANDS: Final = (
    (
        "cargo",
        "test",
        "-p",
        "agentmage-kernel-engine",
        "--locked",
        "served_capabilit",
    ),
    (
        "cargo",
        "test",
        "-p",
        "agentmage-platform-linux-inference",
        "--locked",
        "undersized_served_capacity_refuses_and_retains_no_loaded_binding",
    ),
    (
        "cargo",
        "test",
        "-p",
        "agentmage-platform-linux-inference",
        "--locked",
        "llama_server_driver::tests::",
    ),
)
MARKERS: Final = (
    "served_capabilities_reject_missing_forged_undersized_and_unsafe_facts ... ok",
    "health_rejects_every_served_capability_drift_before_dispatch ... ok",
    "served_capability_stable_sleep_reobservation_and_reload_generation_are_explicit ... ok",
    "undersized_served_capacity_refuses_and_retains_no_loaded_binding ... ok",
    "process_generation_is_read_from_the_live_process_identity ... ok",
    "serving_properties_reject_missing_undersized_identity_and_sleep_drift ... ok",
)
SOURCE_PATHS: Final = (
    "kernel/contracts/src/lib.rs",
    "kernel/contracts/src/model.rs",
    "kernel/engine/src/model_runtime.rs",
    "kernel/engine/src/runtime_loop.rs",
    "platforms/linux-inference/src/llama_server_driver.rs",
    "platforms/linux-inference/src/model_install_verifier.rs",
    "platforms/linux-inference/src/native_model_adapter.rs",
    "requirements/context-safety-registration.json",
    "scripts/story_13_1_served_capability_evidence.py",
    "tests/test_story_13_1_served_capability_evidence.py",
)
REJECTION_MATRIX: Final = (
    "missing-observation",
    "profile-served-capacity-mismatch",
    "forged-profile-manifest-artifact-runtime",
    "unsafe-parallel-shared-cache",
    "endpoint-drift",
    "sleep-resume-stable-process-generation",
    "process-restart-generation-drift",
    "adapter-reload-generation",
    "tokenizer-drift",
    "template-drift",
    "slot-drift",
    "cache-policy-drift",
)
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def resolve_revision(revision: str) -> str:
    result = subprocess.run(
        ("git", "rev-parse", "--verify", f"{revision}^{{commit}}"),
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
        timeout=10,
    )
    value = result.stdout.strip()
    if result.returncode != 0 or not REVISION.fullmatch(value):
        raise ValueError("served-capability source revision is unavailable")
    return value


def git_file(revision: str, relative: str) -> bytes:
    result = subprocess.run(
        ("git", "show", f"{revision}:{relative}"),
        cwd=ROOT,
        capture_output=True,
        check=False,
        timeout=10,
    )
    if result.returncode != 0:
        raise ValueError(f"committed served-capability source is absent: {relative}")
    return result.stdout


def capture() -> tuple[str, int]:
    chunks = []
    for command in COMMANDS:
        result = subprocess.run(
            command,
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
            timeout=900,
        )
        chunks.append(f"$ {' '.join(command)}\n{result.stdout}")
        if result.returncode != 0:
            return "\n".join(chunks).rstrip() + "\n", result.returncode
    return "\n".join(chunks).rstrip() + "\n", 0


def validate_raw(value: str) -> list[str]:
    failures = [f"served-capability results missing marker: {marker}" for marker in MARKERS if marker not in value]
    for prohibited in ("test result: FAILED", "error: could not compile", "panicked at"):
        if prohibited in value:
            failures.append(f"served-capability results contain prohibited marker: {prohibited}")
    return failures


def expected_report(source_revision: str, raw: bytes) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "agentmage-story-13-1-served-capability-evidence",
        "story_id": "13.1",
        "task_id": "13.1.5",
        "source_revision": source_revision,
        "source_sha256": {
            path: sha256_bytes(git_file(source_revision, path)) for path in SOURCE_PATHS
        },
        "raw_results": {
            "path": RAW_PATH.relative_to(ROOT).as_posix(),
            "sha256": sha256_bytes(raw),
        },
        "commands": [" ".join(command) for command in COMMANDS],
        "ctx_served": {
            "deterministic_adapter_matrix": list(REJECTION_MATRIX),
            "stable_refusal_codes": [
                "model.served-capability.missing",
                "model.served-capability.profile-mismatch",
                "model.served-capability.drift",
            ],
            "zero_generation_dispatch_on_refusal": True,
            "single_slot_covered": True,
            "parallel_isolated_slots_covered": True,
            "parallel_shared_cache_refused": True,
            "native_launch_derived_from_profile": True,
            "native_process_and_command_line_reobserved": True,
            "native_read_only_properties_reobserved": True,
        },
        "claims": {
            "local_contract_complete": True,
            "native_boundary_campaign_executed": False,
            "native_model_loaded_by_this_campaign": False,
            "model_profile_enabled": False,
            "platform_qualified": False,
            "product_support_claim": False,
            "release_claim": "none",
        },
        "status": "PASS_LOCAL_FIXTURES_BLOCKED_NATIVE_BOUNDARY",
        "remaining_prerequisite": (
            "Run CTX-SERVED against the exact separately authorized artifact/runtime/venue tuple; "
            "retain attributable restart, reload, reassignment, tokenizer/template, slot, and cache-policy evidence."
        ),
        "limitations": [
            "The process-generation unit case observes only the test process identity and does not load a model.",
            "Deterministic adapter results do not qualify a model, runtime, platform, or product release.",
            "No running demo service, model configuration, artifact, or USTE process was changed.",
        ],
    }


def validate_report(report: Any, source_revision: str, raw: bytes) -> list[str]:
    try:
        expected = expected_report(source_revision, raw)
    except ValueError as error:
        return [str(error)]
    if report != expected:
        return ["served-capability report is stale, incomplete, or widened"]
    source_hashes = report.get("source_sha256", {})
    if set(source_hashes) != set(SOURCE_PATHS) or any(
        not SHA256.fullmatch(str(value)) for value in source_hashes.values()
    ):
        return ["served-capability source closure is invalid"]
    return []


def render(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, sort_keys=True) + "\n"


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
            REPORT_PATH.write_text(render(expected_report(revision, raw)), encoding="utf-8")
    else:
        try:
            raw = RAW_PATH.read_bytes()
            raw_text = raw.decode()
            report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
            failures.append(f"cannot read served-capability evidence: {error}")
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
    print("CTX-SERVED local fixtures pass; exact native boundary evidence remains open")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
