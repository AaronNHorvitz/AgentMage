#!/usr/bin/env python3
"""Build deterministic CTX-FINISH evidence for truthful local model results."""

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
RAW_PATH: Final = EVIDENCE_DIR / "finish-usage-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "finish-usage-report.json"
COMMANDS: Final = (
    (
        "cargo",
        "test",
        "-p",
        "agentmage-platform-linux-inference",
        "--locked",
        "llama_server_driver::tests::",
        "--",
        "--test-threads=1",
    ),
    (
        "cargo",
        "test",
        "-p",
        "agentmage-kernel-engine",
        "--locked",
        "ctx_finish",
        "--",
        "--test-threads=1",
    ),
    (
        "cargo",
        "test",
        "-p",
        "agentmage-kernel-engine",
        "--locked",
        "fake_muse_and_gemma_cover_every_runtime_terminal_and_hostile_response_state",
        "--",
        "--test-threads=1",
    ),
    (
        "cargo",
        "test",
        "-p",
        "agentmage-host",
        "--locked",
        "engineering_model::tests::current_local_profiles_cannot_cross_the_product_bridge",
        "--",
        "--test-threads=1",
    ),
)
MARKERS: Final = (
    "completion_streams_exact_sampling_tuple_and_classifies_inert_output ... ok",
    "ctx_finish_retains_incomplete_reasons_usage_and_bounded_partial_bytes ... ok",
    "live_probe_cancels_after_a_fragment_and_closes_the_stream_once ... ok",
    "deadline_interrupts_a_silent_stream_with_one_terminal_fragment ... ok",
    "ctx_finish_projects_stable_content_free_host_failure_codes ... ok",
    "fake_muse_and_gemma_cover_every_runtime_terminal_and_hostile_response_state ... ok",
    "current_local_profiles_cannot_cross_the_product_bridge ... ok",
)
SOURCE_PATHS: Final = (
    "kernel/contracts/src/lib.rs",
    "kernel/contracts/src/model.rs",
    "kernel/engine/src/model_runtime.rs",
    "kernel/engine/src/runtime_event.rs",
    "kernel/engine/src/runtime_loop.rs",
    "kernel/engine/src/runtime_loop_tests.rs",
    "platforms/linux-inference/src/llama_server_driver.rs",
    "shells/host/src/engineering_model.rs",
    "shells/host/src/engineering_runtime.rs",
    "scripts/story_13_1_finish_usage_evidence.py",
    "tests/test_story_13_1_finish_usage_evidence.py",
)
FINISH_MATRIX: Final = (
    "end-of-sequence",
    "configured-stop",
    "output-token-limit",
    "context-truncation",
    "reasoning-exhaustion",
    "cancellation",
    "deadline-exceeded",
    "transport-failure",
    "unknown",
)
USAGE_FACTS: Final = (
    "rendered-prompt",
    "cached-input",
    "evaluated-input",
    "generated-output",
    "reasoning-output",
    "output-reserve",
    "remaining-capacity",
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
        raise ValueError("finish-usage source revision is unavailable")
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
        raise ValueError(f"committed finish-usage source is absent: {relative}")
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
    failures = [
        f"finish-usage results missing marker: {marker}"
        for marker in MARKERS
        if marker not in value
    ]
    for prohibited in ("test result: FAILED", "error: could not compile", "panicked at"):
        if prohibited in value:
            failures.append(f"finish-usage results contain prohibited marker: {prohibited}")
    return failures


def expected_report(source_revision: str, raw: bytes) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "agentmage-story-13-1-finish-usage-evidence",
        "story_id": "13.1",
        "task_id": "13.1.6",
        "source_revision": source_revision,
        "source_sha256": {
            path: sha256_bytes(git_file(source_revision, path)) for path in SOURCE_PATHS
        },
        "raw_results": {
            "path": RAW_PATH.relative_to(ROOT).as_posix(),
            "sha256": sha256_bytes(raw),
        },
        "commands": [" ".join(command) for command in COMMANDS],
        "ctx_finish": {
            "finish_matrix": list(FINISH_MATRIX),
            "usage_facts": list(USAGE_FACTS),
            "provider_metrics_unavailable_is_explicit": True,
            "limit_stopped_json_is_incomplete": True,
            "limit_stopped_text_is_incomplete": True,
            "bounded_partial_transport_capture": True,
            "contradictory_usage_refused": True,
            "incomplete_proposal_dispatch": False,
            "hidden_overflow_retry": False,
            "host_diagnostic_contains_content": False,
        },
        "claims": {
            "local_contract_complete": True,
            "native_model_trial_executed": False,
            "model_profile_enabled": False,
            "platform_qualified": False,
            "product_support_claim": False,
            "release_claim": "none",
        },
        "status": "PASS_LOCAL_CONTRACT",
        "limitations": [
            "The matrix uses deterministic Unix-stream fixtures and fake admitted profiles.",
            "Reasoning exhaustion is exercised at the provider-neutral controller contract because the pinned b10423 server exposes no separate reasoning-token finish reason.",
            "No native model, running demo service, model configuration, artifact, or USTE process was changed.",
        ],
    }


def validate_report(report: Any, source_revision: str, raw: bytes) -> list[str]:
    try:
        expected = expected_report(source_revision, raw)
    except ValueError as error:
        return [str(error)]
    if report != expected:
        return ["finish-usage report is stale, incomplete, or widened"]
    source_hashes = report.get("source_sha256", {})
    if set(source_hashes) != set(SOURCE_PATHS) or any(
        not SHA256.fullmatch(str(value)) for value in source_hashes.values()
    ):
        return ["finish-usage source closure is invalid"]
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
            failures.append(f"cannot read finish-usage evidence: {error}")
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
    print("CTX-FINISH local contract and host projection pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
