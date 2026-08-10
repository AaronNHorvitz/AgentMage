#!/usr/bin/env python3
"""Build and validate pinned fuzz tools, budgets, and retention policy."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
import tempfile
from pathlib import Path, PurePosixPath
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
POLICY_PATH = ROOT / "fuzzing/toolchain-policy.json"
TARGET_REGISTRY_PATH = ROOT / "fuzzing/target-registry.json"
REPORT_PATH = ROOT / "artifacts/sprints/sprint-2/story-2.2/toolchain-policy-report.json"
DICTIONARY_PATHS = (
    "fuzzing/dictionaries/common.dict",
    "fuzzing/dictionaries/structured.dict",
    "fuzzing/dictionaries/paths.dict",
    "fuzzing/dictionaries/git.dict",
    "fuzzing/dictionaries/sqlite.dict",
    "fuzzing/dictionaries/archives.dict",
)
CARGO_FUZZ_VERSION = "0.13.2"
CARGO_FUZZ_CRATE_SHA256 = "5acfd01930e49823e58c30dd8012d3338a620377d7c7d4cc140ca4b2169400e2"
RUST_NIGHTLY = "nightly-2026-08-01"
RUST_NIGHTLY_MANIFEST_SHA256 = (
    "f4b63db200deb36a120b27ae085140637ee04af1254891308799225e5afbe8bf"
)
JAZZER_VERSION = "4.0.0"
JAZZER_INTEGRITY = (
    "sha512-w90xGs5qMOE9KA5AV7OrosTWIOvMaeH5YSSnMr14FNAg2n0kalkurZMwlboz9G3+"
    "SccMp4n6QkKExq3b7r1VAQ=="
)


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-fuzz-toolchain-", dir=path.parent
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


def artifact(root: Path, relative: str) -> dict[str, str]:
    path = root / relative
    if not path.is_file():
        raise ValueError(f"fuzz toolchain artifact is missing: {relative}")
    return {"path": relative, "sha256": sha256_bytes(path.read_bytes())}


def build_policy(root: Path = ROOT) -> dict[str, Any]:
    registry = read_json(root / TARGET_REGISTRY_PATH.relative_to(ROOT))
    dictionaries = [artifact(root, path) for path in DICTIONARY_PATHS]
    value = {
        "schema_version": 1,
        "policy_id": "agentmage-fuzz-toolchain-policy-v1",
        "policy_version": "1.0.0",
        "status": "pinned-tools-recorded-not-installed",
        "target_registry": {
            "path": TARGET_REGISTRY_PATH.relative_to(ROOT).as_posix(),
            "sha256": sha256_bytes(
                (root / TARGET_REGISTRY_PATH.relative_to(ROOT)).read_bytes()
            ),
            "registry_version": registry["registry_version"],
        },
        "engines": [
            {
                "engine_id": "rust-libfuzzer",
                "applies_to": ["rust", "native-ffi"],
                "runner": {
                    "name": "cargo-fuzz",
                    "version": CARGO_FUZZ_VERSION,
                    "crate_sha256": CARGO_FUZZ_CRATE_SHA256,
                    "license": "MIT OR Apache-2.0",
                    "install_command": "cargo install cargo-fuzz --version 0.13.2 --locked",
                },
                "toolchain": {
                    "channel": RUST_NIGHTLY,
                    "channel_manifest_sha256": RUST_NIGHTLY_MANIFEST_SHA256,
                    "target": "x86_64-unknown-linux-gnu",
                    "components": ["cargo", "rustc", "rust-std"],
                },
                "sanitizers": [
                    {
                        "name": "address",
                        "required": True,
                        "scope": "every-rust-fuzz-run",
                    },
                    {
                        "name": "leak",
                        "required": True,
                        "scope": "linux-address-sanitizer-runs",
                    },
                    {
                        "name": "undefined",
                        "required": True,
                        "scope": "each-added-c-or-cpp-ffi-target",
                    },
                ],
                "installation_status": "recorded-not-installed",
            },
            {
                "engine_id": "typescript-jazzer-js",
                "applies_to": ["typescript", "javascript"],
                "runner": {
                    "name": "@jazzer.js/core",
                    "version": JAZZER_VERSION,
                    "npm_integrity": JAZZER_INTEGRITY,
                    "license": "Apache-2.0",
                    "minimum_node": "14.0.0",
                    "minimum_npm": "7.0.0",
                },
                "sanitizers": [
                    {"name": "command-injection", "required": True},
                    {"name": "path-traversal", "required": True},
                    {"name": "server-side-request-forgery", "required": True},
                ],
                "installation_status": "recorded-not-installed",
            },
            {
                "engine_id": "agentmage-bounded-fake-fuzzer",
                "applies_to": ["empty-boundary-baseline", "fake-boundary-baseline"],
                "runner": {
                    "name": "agentmage-standard-library-baseline",
                    "version": "1.0.0",
                    "runtime": "python>=3.11",
                    "third_party_dependencies": [],
                },
                "sanitizers": [
                    {"name": "deterministic-security-oracle", "required": True}
                ],
                "installation_status": "repository-provided",
                "product_coverage_claim": "none",
            },
        ],
        "dictionaries": dictionaries,
        "seed_corpora": [
            artifact(root, "fixtures/story-2.1/later-input-class-fixtures-v1.json"),
            artifact(root, "fixtures/corpus/v1/agentmage-synthetic-corpus-v1.zip"),
            artifact(root, "fixtures/corpus/v1/manifest.json"),
        ],
        "resource_limits": {
            "maximum_input_bytes": 4096,
            "per_input_timeout_seconds": 5,
            "maximum_resident_memory_mib": 1024,
            "maximum_artifact_bytes": 1048576,
            "maximum_corpus_bytes_per_target": 67108864,
            "maximum_parallel_jobs_per_target": 1,
            "host_resource_exhaustion_permitted": False,
        },
        "minimum_durations_seconds": {
            "local_changed_target": 60,
            "continuous_integration_changed_target": 300,
            "scheduled_full_target": 3600,
            "regression_replay": 1,
        },
        "crash_deduplication": {
            "signature_fields": [
                "target_id",
                "failure_class",
                "sanitizer",
                "normalized_top_frames",
            ],
            "signature_algorithm": "sha256-canonical-json",
            "volatile_values_removed": [
                "absolute-path",
                "address",
                "process-id",
                "thread-id",
                "timestamp",
            ],
            "all_distinct_input_hashes_retained": True,
            "cross_target_deduplication_permitted": False,
        },
        "minimization": {
            "required_for_every_non_pass_input": True,
            "maximum_reproducer_bytes": 4096,
            "must_preserve_failure_signature": True,
            "timeout_during_minimization_disposition": "non-pass-unminimized",
        },
        "regression_retention": {
            "path_template": "fuzzing/regressions/<target-id>/<sha256>.bin",
            "raw_private_input_permitted": False,
            "synthetic_minimized_reproducer_permitted": True,
            "minimum_fixed_release_count": 2,
            "deletion_requires_reviewed_disposition": True,
            "replay_on_every_target_change": True,
            "failed_replay_disposition": "block",
        },
        "execution_network_policy": "offline",
        "tool_acquisition_separate_from_execution": True,
        "product_fuzz_execution_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }
    return {**value, "policy_sha256": sha256_bytes(canonical_json(value))}


def safe_relative_path(value: Any) -> bool:
    if not isinstance(value, str) or not value:
        return False
    path = PurePosixPath(value)
    return not path.is_absolute() and ".." not in path.parts and str(path) == value


def validate_policy(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["fuzz toolchain policy must be an object"]
    failures: list[str] = []
    if (
        value.get("schema_version") != 1
        or value.get("policy_id") != "agentmage-fuzz-toolchain-policy-v1"
        or value.get("policy_version") != "1.0.0"
        or value.get("status") != "pinned-tools-recorded-not-installed"
    ):
        failures.append("fuzz toolchain policy identity is invalid")
    engines = value.get("engines", [])
    if [item.get("engine_id") for item in engines] != [
        "rust-libfuzzer",
        "typescript-jazzer-js",
        "agentmage-bounded-fake-fuzzer",
    ]:
        failures.append("fuzz engine closure is invalid")
    if engines:
        rust_runner = engines[0].get("runner", {})
        rust_toolchain = engines[0].get("toolchain", {})
        if (
            rust_runner.get("version") != CARGO_FUZZ_VERSION
            or rust_runner.get("crate_sha256") != CARGO_FUZZ_CRATE_SHA256
            or rust_toolchain.get("channel") != RUST_NIGHTLY
            or rust_toolchain.get("channel_manifest_sha256")
            != RUST_NIGHTLY_MANIFEST_SHA256
        ):
            failures.append("Rust fuzz toolchain is not pinned exactly")
    if len(engines) > 1:
        jazzer = engines[1].get("runner", {})
        if jazzer.get("version") != JAZZER_VERSION or jazzer.get(
            "npm_integrity"
        ) != JAZZER_INTEGRITY:
            failures.append("Jazzer.js toolchain is not pinned exactly")
    dictionary_records = value.get("dictionaries", [])
    if [item.get("path") for item in dictionary_records] != list(DICTIONARY_PATHS):
        failures.append("fuzz dictionary closure is invalid")
    for record in dictionary_records:
        path = record.get("path")
        if not safe_relative_path(path) or not (root / path).is_file():
            failures.append(f"fuzz dictionary path is invalid: {path}")
        elif record.get("sha256") != sha256_bytes((root / path).read_bytes()):
            failures.append(f"fuzz dictionary hash is invalid: {path}")
    limits = value.get("resource_limits", {})
    if (
        limits.get("maximum_input_bytes") != 4096
        or limits.get("per_input_timeout_seconds") != 5
        or limits.get("maximum_resident_memory_mib") != 1024
        or limits.get("host_resource_exhaustion_permitted") is not False
    ):
        failures.append("fuzz resource limits are invalid")
    durations = value.get("minimum_durations_seconds", {})
    if durations != {
        "local_changed_target": 60,
        "continuous_integration_changed_target": 300,
        "scheduled_full_target": 3600,
        "regression_replay": 1,
    }:
        failures.append("fuzz minimum durations are invalid")
    deduplication = value.get("crash_deduplication", {})
    if (
        deduplication.get("signature_algorithm") != "sha256-canonical-json"
        or deduplication.get("all_distinct_input_hashes_retained") is not True
        or deduplication.get("cross_target_deduplication_permitted") is not False
    ):
        failures.append("fuzz crash deduplication is invalid")
    retention = value.get("regression_retention", {})
    if (
        retention.get("raw_private_input_permitted") is not False
        or retention.get("deletion_requires_reviewed_disposition") is not True
        or retention.get("failed_replay_disposition") != "block"
    ):
        failures.append("fuzz regression retention is invalid")
    if value.get("execution_network_policy") != "offline" or value.get(
        "tool_acquisition_separate_from_execution"
    ) is not True:
        failures.append("fuzz acquisition and execution are not separated")
    if value.get("product_fuzz_execution_claim") != "none":
        failures.append("fuzz toolchain policy made a product execution claim")
    if value.get("macos_execution_status") != "blocked-macos" or value.get(
        "macos_support_claim"
    ) != "none":
        failures.append("fuzz toolchain policy made an invalid macOS claim")
    try:
        expected = build_policy(root)
    except (OSError, ValueError, KeyError, TypeError) as error:
        failures.append(f"cannot rebuild fuzz toolchain policy: {error}")
    else:
        if value != expected:
            failures.append("fuzz toolchain policy is stale or non-deterministic")
    return failures


def build_report(root: Path = ROOT) -> dict[str, Any]:
    policy_path = root / POLICY_PATH.relative_to(ROOT)
    policy = read_json(policy_path)
    failures = validate_policy(policy, root)
    if failures:
        raise ValueError("; ".join(failures))
    return {
        "schema_version": 1,
        "task_id": "2.2.1.2",
        "status": "pass-pinned-policy-recorded",
        "policy": {
            "path": POLICY_PATH.relative_to(ROOT).as_posix(),
            "sha256": sha256_bytes(policy_path.read_bytes()),
            "self_sha256": policy["policy_sha256"],
            "version": policy["policy_version"],
        },
        "engine_count": len(policy["engines"]),
        "dictionary_count": len(policy["dictionaries"]),
        "seed_corpus_count": len(policy["seed_corpora"]),
        "budgets_recorded": True,
        "crash_deduplication_recorded": True,
        "regression_retention_recorded": True,
        "tools_installed_by_task": False,
        "network_calls_during_fuzz_execution": 0,
        "product_fuzz_execution_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }


def validate_report(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["fuzz toolchain policy report must be an object"]
    failures: list[str] = []
    if (
        value.get("schema_version") != 1
        or value.get("task_id") != "2.2.1.2"
        or value.get("status") != "pass-pinned-policy-recorded"
    ):
        failures.append("fuzz toolchain policy report identity is invalid")
    if (
        value.get("engine_count") != 3
        or value.get("dictionary_count") != 6
        or value.get("seed_corpus_count") != 3
        or value.get("budgets_recorded") is not True
        or value.get("crash_deduplication_recorded") is not True
        or value.get("regression_retention_recorded") is not True
        or value.get("tools_installed_by_task") is not False
    ):
        failures.append("fuzz toolchain policy report closure is invalid")
    if value.get("network_calls_during_fuzz_execution") != 0 or value.get(
        "product_fuzz_execution_claim"
    ) != "none":
        failures.append("fuzz toolchain policy report made an execution claim")
    if value.get("macos_execution_status") != "blocked-macos" or value.get(
        "macos_support_claim"
    ) != "none":
        failures.append("fuzz toolchain policy report made an invalid macOS claim")
    try:
        expected = build_report(root)
    except (OSError, ValueError, KeyError, TypeError) as error:
        failures.append(f"cannot rebuild fuzz toolchain policy report: {error}")
    else:
        if value != expected:
            failures.append("fuzz toolchain policy report is stale or non-deterministic")
    return failures


def write_artifacts(root: Path = ROOT) -> None:
    policy_path = root / POLICY_PATH.relative_to(ROOT)
    write_atomic(policy_path, canonical_json(build_policy(root)))
    write_atomic(root / REPORT_PATH.relative_to(ROOT), canonical_json(build_report(root)))


def check_artifacts(root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    try:
        policy = read_json(root / POLICY_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        failures.append(f"cannot read fuzz toolchain policy: {error}")
    else:
        failures.extend(validate_policy(policy, root))
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        failures.append(f"cannot read fuzz toolchain policy report: {error}")
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
        print(f"fuzz toolchain policy failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"fuzz toolchain policy failed: {failure}", file=sys.stderr)
        return 1
    print("Story 2.2 fuzz toolchain policy validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
