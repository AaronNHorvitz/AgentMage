#!/usr/bin/env python3
"""Build and validate the canonical fuzz-result contract fixture."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
import tempfile
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
SCHEMA_PATH = ROOT / "schemas/testing/fuzz-result.schema.json"
EXAMPLE_PATH = ROOT / "schemas/testing/examples/fuzz-result.valid.json"
REPORT_PATH = ROOT / "artifacts/sprints/sprint-2/story-2.2/fuzz-result-schema-report.json"
REQUIRED_CONTRACT_FIELDS = (
    "seed",
    "coverage",
    "sanitizer-state",
    "crash-signature",
    "minimized-reproducer",
    "timeout-resource-event",
    "owner",
    "severity",
    "disposition",
    "evidence-hash",
    "redaction",
)


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-fuzz-result-contract-", dir=path.parent
    )
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
        temporary.chmod(0o644)
        os.replace(temporary, path)
    except BaseException:
        temporary.unlink(missing_ok=True)
        raise


def build_example(root: Path = ROOT) -> dict[str, Any]:
    registry_path = root / "fuzzing/target-registry.json"
    policy_path = root / "fuzzing/toolchain-policy.json"
    matrix_path = root / "fixtures/story-2.1/later-input-class-fixtures-v1.json"
    platform_path = root / (
        "artifacts/sprints/sprint-2/story-2.1/platform-result-recorder-report.json"
    )
    policy = read_json(policy_path)
    dictionary_hashes = [item["sha256"] for item in policy["dictionaries"]]
    minimized_content = b"synthetic/../escape"
    original_hash = sha256_bytes(b"synthetic/../../outside-boundary")
    minimized_hash = sha256_bytes(minimized_content)
    failure_basis = {
        "target_id": "FT-PATH-001",
        "failure_class": "path-escape",
        "sanitizer": "deterministic-security-oracle",
        "normalized_top_frames": ["fake_path_boundary::resolve"],
    }
    crash_signature = sha256_bytes(canonical_json(failure_basis))
    sanitizer_state = {
        "enabled": ["deterministic-security-oracle"],
        "triggered": ["path-escape-oracle"],
        "finding_count": 1,
    }
    sanitizer = {
        **sanitizer_state,
        "state_sha256": sha256_bytes(canonical_json(sanitizer_state)),
    }
    coverage_state = {
        "target_id": "FT-PATH-001",
        "seed": 2201001,
        "features_covered": 7,
        "new_features": 1,
        "corpus_entry_count": 7,
    }
    identity = sha256_bytes(canonical_json(coverage_state))[:20]
    value = {
        "schema_version": 1,
        "record_type": "fuzz-result",
        "record_id": f"am-fuzz-result-{identity}",
        "captured_at": "2024-01-01T00:10:00Z",
        "target": {
            "target_id": "FT-PATH-001",
            "boundary_class": "paths",
            "registry_sha256": sha256_file(registry_path),
            "harness_sha256": sha256_file(root / "scripts/fuzz_result_contract.py"),
            "corpus_sha256": sha256_file(matrix_path),
            "dictionary_sha256": dictionary_hashes,
            "build_sha256": sha256_file(root / "Cargo.lock"),
            "policy_sha256": sha256_file(policy_path),
            "platform_result_sha256": sha256_file(platform_path),
            "engine": {
                "engine_id": "agentmage-bounded-fake-fuzzer",
                "version": "1.0.0",
                "integrity": sha256_bytes(b"agentmage-bounded-fake-fuzzer-v1"),
            },
        },
        "run": {
            "mode": "fuzzing",
            "seed": 2201001,
            "duration_seconds": 60,
            "execution_count": 7,
            "maximum_input_bytes": 4096,
            "result_status": "path-escape",
        },
        "coverage": {
            "kind": "deterministic-oracle",
            "features_covered": 7,
            "new_features": 1,
            "corpus_entry_count": 7,
            "coverage_sha256": sha256_bytes(canonical_json(coverage_state)),
            "unavailable_reason": None,
        },
        "sanitizer": sanitizer,
        "failure": {
            "failure_class": "path-escape",
            "crash_signature": crash_signature,
            "raw_input_sha256": original_hash,
            "stack_sha256": None,
            "normalized_top_frames": ["fake_path_boundary::resolve"],
            "raw_input_retained": False,
        },
        "minimized_reproducer": {
            "status": "minimized",
            "original_sha256": original_hash,
            "minimized_sha256": minimized_hash,
            "path": f"fuzzing/regressions/FT-PATH-001/{minimized_hash}.bin",
            "bytes": len(minimized_content),
            "failure_signature_preserved": True,
            "raw_private_input_retained": False,
        },
        "timeout_resource_event": {
            "event": "none",
            "limit": None,
            "observed": None,
            "unit": None,
            "host_exhausted": False,
        },
        "ownership": {
            "owner": "kernel-path-boundary",
            "severity": "high",
            "disposition": "fix-required",
            "duplicate_of": None,
            "risk_decision": None,
        },
        "evidence": [
            {
                "path": "fuzzing/target-registry.json",
                "sha256": sha256_file(registry_path),
            },
            {
                "path": "fuzzing/toolchain-policy.json",
                "sha256": sha256_file(policy_path),
            },
        ],
        "redaction": {
            "secret_canary_values_recorded": False,
            "private_paths_recorded": False,
            "raw_payload_recorded": False,
            "redaction_count": 0,
        },
        "network_used": False,
        "product_fuzz_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }
    return {**value, "record_sha256": sha256_bytes(canonical_json(value))}


def validate_example(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["fuzz result example must be an object"]
    failures: list[str] = []
    if value.get("schema_version") != 1 or value.get("record_type") != "fuzz-result":
        failures.append("fuzz result example identity is invalid")
    run = value.get("run", {})
    failure = value.get("failure")
    if run.get("result_status") == "pass":
        if failure is not None or value.get("minimized_reproducer", {}).get(
            "status"
        ) != "not-required":
            failures.append("passing fuzz result retained a failure")
    elif not isinstance(failure, dict) or failure.get("failure_class") != run.get(
        "result_status"
    ):
        failures.append("non-pass fuzz result lost its typed failure")
    sanitizer = value.get("sanitizer", {})
    if sanitizer.get("finding_count") != len(sanitizer.get("triggered", [])):
        failures.append("fuzz sanitizer findings do not reconcile")
    minimized = value.get("minimized_reproducer", {})
    if minimized.get("status") == "minimized" and (
        minimized.get("failure_signature_preserved") is not True
        or not isinstance(minimized.get("path"), str)
        or not isinstance(minimized.get("bytes"), int)
        or minimized.get("bytes", 4097) > 4096
    ):
        failures.append("fuzz minimized reproducer contract is invalid")
    if minimized.get("raw_private_input_retained") is not False:
        failures.append("fuzz result retained a private reproducer")
    redaction = value.get("redaction", {})
    if any(
        redaction.get(field) is not False
        for field in (
            "secret_canary_values_recorded",
            "private_paths_recorded",
            "raw_payload_recorded",
        )
    ):
        failures.append("fuzz result redaction contract was weakened")
    if value.get("network_used") is not False or value.get("product_fuzz_claim") != "none":
        failures.append("fuzz result made a network or product claim")
    if value.get("macos_execution_status") != "blocked-macos" or value.get(
        "macos_support_claim"
    ) != "none":
        failures.append("fuzz result made an invalid macOS claim")
    core = {key: item for key, item in value.items() if key != "record_sha256"}
    if value.get("record_sha256") != sha256_bytes(canonical_json(core)):
        failures.append("fuzz result self-hash is invalid")
    try:
        expected = build_example(root)
    except (OSError, ValueError, KeyError, TypeError) as error:
        failures.append(f"cannot rebuild fuzz result example: {error}")
    else:
        if value != expected:
            failures.append("fuzz result example is stale or non-deterministic")
    return failures


def build_report(root: Path = ROOT) -> dict[str, Any]:
    example_path = root / EXAMPLE_PATH.relative_to(ROOT)
    example = read_json(example_path)
    failures = validate_example(example, root)
    if failures:
        raise ValueError("; ".join(failures))
    return {
        "schema_version": 1,
        "task_id": "2.2.1.3",
        "status": "pass-result-contract-recorded",
        "schema": {
            "path": SCHEMA_PATH.relative_to(ROOT).as_posix(),
            "sha256": sha256_file(root / SCHEMA_PATH.relative_to(ROOT)),
            "draft": "2020-12",
        },
        "canonical_example": {
            "path": EXAMPLE_PATH.relative_to(ROOT).as_posix(),
            "sha256": sha256_file(example_path),
            "record_sha256": example["record_sha256"],
        },
        "required_contract_fields": list(REQUIRED_CONTRACT_FIELDS),
        "required_contract_field_count": len(REQUIRED_CONTRACT_FIELDS),
        "additional_properties_permitted": False,
        "non_pass_failure_required": True,
        "private_values_retained": False,
        "product_fuzz_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }


def validate_report(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["fuzz result schema report must be an object"]
    failures: list[str] = []
    if (
        value.get("schema_version") != 1
        or value.get("task_id") != "2.2.1.3"
        or value.get("status") != "pass-result-contract-recorded"
    ):
        failures.append("fuzz result schema report identity is invalid")
    if value.get("required_contract_fields") != list(REQUIRED_CONTRACT_FIELDS) or value.get(
        "required_contract_field_count"
    ) != len(REQUIRED_CONTRACT_FIELDS):
        failures.append("fuzz result schema report field closure is invalid")
    if (
        value.get("additional_properties_permitted") is not False
        or value.get("non_pass_failure_required") is not True
        or value.get("private_values_retained") is not False
    ):
        failures.append("fuzz result schema report safety contract is invalid")
    if value.get("product_fuzz_claim") != "none":
        failures.append("fuzz result schema report made a product claim")
    if value.get("macos_execution_status") != "blocked-macos" or value.get(
        "macos_support_claim"
    ) != "none":
        failures.append("fuzz result schema report made an invalid macOS claim")
    try:
        expected = build_report(root)
    except (OSError, ValueError, KeyError, TypeError) as error:
        failures.append(f"cannot rebuild fuzz result schema report: {error}")
    else:
        if value != expected:
            failures.append("fuzz result schema report is stale or non-deterministic")
    return failures


def write_artifacts(root: Path = ROOT) -> None:
    write_atomic(root / EXAMPLE_PATH.relative_to(ROOT), canonical_json(build_example(root)))
    write_atomic(root / REPORT_PATH.relative_to(ROOT), canonical_json(build_report(root)))


def check_artifacts(root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    try:
        example = read_json(root / EXAMPLE_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        failures.append(f"cannot read fuzz result example: {error}")
    else:
        failures.extend(validate_example(example, root))
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        failures.append(f"cannot read fuzz result schema report: {error}")
    else:
        failures.extend(validate_report(report, root))
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            write_artifacts()
        failures = check_artifacts()
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"fuzz result contract failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"fuzz result contract failed: {failure}", file=sys.stderr)
        return 1
    print("Story 2.2 fuzz result contract validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
