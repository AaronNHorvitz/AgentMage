#!/usr/bin/env python3
"""Build deterministic sharded result bundles from synthetic attempt records."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import os
import re
import sys
import tempfile
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.platform_result_recorder import (  # noqa: E402
    record_platform_result,
    validate_profile as validate_platform_profile,
    validate_record as validate_platform_record,
)


PROFILE_PATH = ROOT / "fixtures" / "test-result-bundle-profile.json"
REPORT_PATH = (
    ROOT
    / "artifacts"
    / "sprints"
    / "sprint-2"
    / "story-2.1"
    / "test-result-bundle-report.json"
)
EXPECTED_CASE_IDS = (
    "parser.normal",
    "parser.persistent-failure",
    "model.flaky",
    "runtime.timeout-flaky",
    "documents.skipped",
    "known.quarantined-failure",
    "worker.timeout-exhausted",
    "output.oversized",
    "worker.cancelled",
)
RETRYABLE_STATUSES = ("fail", "error", "timeout")
TERMINAL_STATUSES = ("pass", "skipped", "cancelled")
ALL_STATUSES = RETRYABLE_STATUSES + TERMINAL_STATUSES
CLASSIFICATIONS = (
    "pass",
    "persistent-failure",
    "flaky",
    "skipped",
    "quarantined-failure",
    "retry-exhausted",
    "cancelled",
)
EXPECTED_SIDE_EFFECTS = {
    "executes_tests": False,
    "uses_network": False,
    "changes_test_status": False,
    "retains_raw_output": False,
    "suppresses_non_pass_results": False,
}
IDENTIFIER = re.compile(r"^[a-z0-9][a-z0-9._-]{0,127}$")
HASH = re.compile(r"^[0-9a-f]{64}$")
TIMESTAMP = re.compile(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$")
SYNTHETIC_CANARY = re.compile(r"AM_SYNTHETIC_(?:SECRET|CANARY)_[A-Z0-9_]+")
MAX_RAW_FIXTURE_OUTPUT_BYTES = 4096


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def validate_attempts(case: dict[str, Any], profile: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    attempts = case.get("attempts")
    if not isinstance(attempts, list) or not attempts:
        return [f"test case has no attempts: {case.get('test_id')}"]
    max_attempts = profile["retry_policy"]["max_attempts"]
    if len(attempts) > max_attempts:
        failures.append(f"test case exceeds maximum attempts: {case.get('test_id')}")
    for index, attempt in enumerate(attempts):
        if not isinstance(attempt, dict) or set(attempt) != {"duration_ms", "output", "status"}:
            failures.append(f"test attempt is malformed: {case.get('test_id')}:{index + 1}")
            continue
        if attempt["status"] not in ALL_STATUSES:
            failures.append(f"test attempt status is invalid: {case.get('test_id')}:{index + 1}")
        if (
            not isinstance(attempt["duration_ms"], int)
            or isinstance(attempt["duration_ms"], bool)
            or attempt["duration_ms"] < 0
            or attempt["duration_ms"] > 60_000
        ):
            failures.append(f"test attempt duration is invalid: {case.get('test_id')}:{index + 1}")
        output = attempt["output"]
        if not isinstance(output, str) or len(output.encode("utf-8")) > MAX_RAW_FIXTURE_OUTPUT_BYTES:
            failures.append(f"test attempt output is invalid: {case.get('test_id')}:{index + 1}")
        if index < len(attempts) - 1 and attempt["status"] not in RETRYABLE_STATUSES:
            failures.append(f"terminal test attempt was retried: {case.get('test_id')}:{index + 1}")
    if case.get("quarantined"):
        if len(attempts) != 1 or profile["retry_policy"]["quarantined_retries"]:
            failures.append(f"quarantined test retry policy drifted: {case.get('test_id')}")
    elif attempts[-1].get("status") in RETRYABLE_STATUSES and len(attempts) != max_attempts:
        failures.append(f"retryable test stopped before retry exhaustion: {case.get('test_id')}")
    return failures


def validate_profile(profile: Any) -> list[str]:
    if not isinstance(profile, dict):
        return ["test result bundle profile must be an object"]
    failures: list[str] = []
    if (
        profile.get("schema_version") != 1
        or profile.get("profile_id") != "agentmage-test-result-bundle-v1"
        or profile.get("status") != "synthetic-result-bundle-contract"
    ):
        failures.append("test result bundle profile identity is invalid")
    if not isinstance(profile.get("generated_at"), str) or not TIMESTAMP.fullmatch(profile["generated_at"]):
        failures.append("test result bundle timestamp is invalid")
    if profile.get("platform_profile") != {
        "path": "fixtures/platform-result-profile.json",
        "synthetic_result_id": "synthetic-fedora-result",
    }:
        failures.append("test result bundle platform profile drifted")
    if profile.get("sharding") != {
        "algorithm": "sha256-test-id-modulo-v1",
        "shard_count": 3,
    }:
        failures.append("test result bundle sharding contract drifted")
    if profile.get("retry_policy") != {
        "max_attempts": 3,
        "quarantined_retries": False,
        "retryable_statuses": list(RETRYABLE_STATUSES),
        "terminal_statuses": list(TERMINAL_STATUSES),
    }:
        failures.append("test result bundle retry policy drifted")
    if profile.get("output_policy") != {
        "encoding": "utf-8",
        "max_retained_bytes_per_attempt": 64,
        "raw_output_retained": False,
        "synthetic_canaries_redacted": True,
    }:
        failures.append("test result bundle output policy drifted")
    if profile.get("side_effect_contract") != EXPECTED_SIDE_EFFECTS:
        failures.append("test result bundle side-effect contract was weakened")
    if profile.get("macos_execution_status") != "blocked-macos":
        failures.append("test result bundle lost blocked macOS status")
    cases = profile.get("cases")
    if not isinstance(cases, list):
        return failures + ["test result bundle cases must be a list"]
    case_ids = tuple(case.get("test_id") for case in cases if isinstance(case, dict))
    if case_ids != EXPECTED_CASE_IDS:
        failures.append("test result bundle case closure drifted")
    if len(case_ids) != len(set(case_ids)):
        failures.append("test result bundle test identities are duplicated")
    for case in cases:
        if not isinstance(case, dict) or set(case) != {
            "attempts",
            "fixture_id",
            "quarantined",
            "test_id",
        }:
            failures.append("test result bundle case is malformed")
            continue
        for field in ("test_id", "fixture_id"):
            if not isinstance(case[field], str) or not IDENTIFIER.fullmatch(case[field]):
                failures.append(f"test result bundle identifier is invalid: {field}")
        if not isinstance(case["quarantined"], bool):
            failures.append(f"test result bundle quarantine flag is invalid: {case['test_id']}")
        failures.extend(validate_attempts(case, profile))
    return failures


def shard_for(test_id: str, shard_count: int) -> int:
    digest = hashlib.sha256(test_id.encode("utf-8")).digest()
    return int.from_bytes(digest[:8], "big") % shard_count


def redact_and_bound(output: str, limit: int) -> dict[str, Any]:
    raw = output.encode("utf-8")
    redacted_text, replacement_count = SYNTHETIC_CANARY.subn(
        "<SYNTHETIC_CANARY_REDACTED>", output
    )
    redacted = redacted_text.encode("utf-8")
    retained = redacted[:limit]
    while True:
        try:
            retained_text = retained.decode("utf-8")
            break
        except UnicodeDecodeError as error:
            retained = retained[: error.start]
    return {
        "output_bytes": len(raw),
        "output_sha256": sha256_bytes(raw),
        "redaction_count": replacement_count,
        "retained_bytes": len(retained),
        "retained_output": retained_text,
        "retained_sha256": sha256_bytes(retained),
        "truncated": len(redacted) > len(retained),
    }


def classify(statuses: list[str], quarantined: bool) -> str:
    final = statuses[-1]
    if quarantined and final != "pass":
        return "quarantined-failure"
    if final == "pass":
        return "flaky" if any(status != "pass" for status in statuses[:-1]) else "pass"
    if final == "skipped":
        return "skipped"
    if final == "cancelled":
        return "cancelled"
    if final == "timeout":
        return "retry-exhausted"
    return "persistent-failure"


def normalize_case(case: dict[str, Any], profile: dict[str, Any]) -> dict[str, Any]:
    attempts = []
    limit = profile["output_policy"]["max_retained_bytes_per_attempt"]
    for number, attempt in enumerate(case["attempts"], start=1):
        attempts.append(
            {
                "attempt": number,
                "duration_ms": attempt["duration_ms"],
                "status": attempt["status"],
                "output": redact_and_bound(attempt["output"], limit),
            }
        )
    statuses = [attempt["status"] for attempt in attempts]
    result = {
        "test_id": case["test_id"],
        "fixture_id": case["fixture_id"],
        "shard": shard_for(case["test_id"], profile["sharding"]["shard_count"]),
        "attempts": attempts,
        "retry_count": len(attempts) - 1,
        "final_status": statuses[-1],
        "classification": classify(statuses, case["quarantined"]),
        "flaky": statuses[-1] == "pass" and any(status != "pass" for status in statuses[:-1]),
        "quarantined": case["quarantined"],
        "total_duration_ms": sum(attempt["duration_ms"] for attempt in attempts),
    }
    result["result_sha256"] = sha256_bytes(canonical_json(result))
    return result


def build_platform_context(profile: dict[str, Any], root: Path = ROOT) -> dict[str, Any]:
    platform_profile_path = root / profile["platform_profile"]["path"]
    platform_profile = read_json(platform_profile_path)
    failures = validate_platform_profile(platform_profile)
    if failures:
        raise ValueError("; ".join(failures))
    selected = [
        run
        for run in platform_profile["synthetic_runs"]
        if run["result"]["result_id"] == profile["platform_profile"]["synthetic_result_id"]
    ]
    if len(selected) != 1:
        raise ValueError("test result bundle platform record selection is ambiguous")
    run = selected[0]
    record = record_platform_result(
        run["environment"],
        run["result"],
        run["captured_at"],
        platform_profile["allowlist_version"],
    )
    failures = validate_platform_record(record)
    if failures:
        raise ValueError("; ".join(failures))
    return {
        "platform_profile_id": platform_profile["profile_id"],
        "platform_profile_sha256": sha256_bytes(platform_profile_path.read_bytes()),
        "platform_record_id": record["record_id"],
        "platform_record_sha256": record["record_sha256"],
    }


def build_shards(results: list[dict[str, Any]], shard_count: int) -> list[dict[str, Any]]:
    shards = []
    for index in range(shard_count):
        shard = {
            "index": index,
            "results": sorted(
                (result for result in results if result["shard"] == index),
                key=lambda item: item["test_id"],
            ),
        }
        shard["result_count"] = len(shard["results"])
        shard["shard_sha256"] = sha256_bytes(canonical_json(shard))
        shards.append(shard)
    return shards


def summarize(results: list[dict[str, Any]]) -> dict[str, Any]:
    final_status_counts = {
        status: sum(result["final_status"] == status for result in results)
        for status in ALL_STATUSES
    }
    classification_counts = {
        classification: sum(result["classification"] == classification for result in results)
        for classification in CLASSIFICATIONS
    }
    non_pass = sum(result["final_status"] != "pass" for result in results)
    return {
        "classification_counts": classification_counts,
        "final_status_counts": final_status_counts,
        "flaky_test_count": sum(result["flaky"] for result in results),
        "non_pass_test_count": non_pass,
        "overall_status": "pass" if non_pass == 0 else "fail",
        "quarantined_test_count": sum(result["quarantined"] for result in results),
        "redacted_attempt_count": sum(
            attempt["output"]["redaction_count"] > 0
            for result in results
            for attempt in result["attempts"]
        ),
        "retried_test_count": sum(result["retry_count"] > 0 for result in results),
        "retry_attempt_count": sum(result["retry_count"] for result in results),
        "skipped_test_count": sum(result["final_status"] == "skipped" for result in results),
        "test_count": len(results),
        "truncated_attempt_count": sum(
            attempt["output"]["truncated"]
            for result in results
            for attempt in result["attempts"]
        ),
    }


def build_bundle(profile: dict[str, Any], root: Path = ROOT) -> dict[str, Any]:
    failures = validate_profile(profile)
    if failures:
        raise ValueError("; ".join(failures))
    profile_path = root / PROFILE_PATH.relative_to(ROOT)
    results = [normalize_case(case, profile) for case in profile["cases"]]
    bundle = {
        "schema_version": 1,
        "bundle_type": "synthetic-sharded-test-results",
        "generated_at": profile["generated_at"],
        "profile": {
            "id": profile["profile_id"],
            "sha256": sha256_bytes(profile_path.read_bytes()),
        },
        "context": build_platform_context(profile, root),
        "sharding": profile["sharding"],
        "shards": build_shards(results, profile["sharding"]["shard_count"]),
        "summary": summarize(results),
        "content_addressing": {
            "algorithm": "sha256",
            "canonicalization": "utf8-sorted-key-indented-json-v1",
            "filesystem_immutability_claim": "none",
        },
        "raw_output_retained": False,
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }
    bundle["bundle_sha256"] = sha256_bytes(canonical_json(bundle))
    return bundle


def flattened_results(bundle: dict[str, Any]) -> list[dict[str, Any]]:
    return [result for shard in bundle["shards"] for result in shard["results"]]


def validate_bundle(bundle: Any, profile: dict[str, Any], root: Path = ROOT) -> list[str]:
    if not isinstance(bundle, dict):
        return ["test result bundle must be an object"]
    failures: list[str] = []
    expected_fields = {
        "schema_version",
        "bundle_type",
        "generated_at",
        "profile",
        "context",
        "sharding",
        "shards",
        "summary",
        "content_addressing",
        "raw_output_retained",
        "macos_execution_status",
        "macos_support_claim",
        "bundle_sha256",
    }
    if set(bundle) != expected_fields:
        return ["test result bundle fields do not match the contract"]
    unhashed = copy.deepcopy(bundle)
    recorded_bundle_hash = unhashed.pop("bundle_sha256")
    if not isinstance(recorded_bundle_hash, str) or sha256_bytes(canonical_json(unhashed)) != recorded_bundle_hash:
        failures.append("test result bundle hash is invalid")
    if bundle["raw_output_retained"] is not False:
        failures.append("test result bundle retained raw output")
    if bundle["macos_execution_status"] != "blocked-macos" or bundle["macos_support_claim"] != "none":
        failures.append("test result bundle made an invalid macOS claim")
    results = flattened_results(bundle)
    observed_ids = [result.get("test_id") for result in results]
    if sorted(observed_ids) != sorted(EXPECTED_CASE_IDS) or len(observed_ids) != len(set(observed_ids)):
        failures.append("test result bundle omitted or duplicated a test result")
    limit = profile["output_policy"]["max_retained_bytes_per_attempt"]
    for shard in bundle["shards"]:
        unhashed_shard = copy.deepcopy(shard)
        recorded_shard_hash = unhashed_shard.pop("shard_sha256", None)
        if sha256_bytes(canonical_json(unhashed_shard)) != recorded_shard_hash:
            failures.append(f"test result shard hash is invalid: {shard.get('index')}")
        if shard.get("result_count") != len(shard.get("results", [])):
            failures.append(f"test result shard count is invalid: {shard.get('index')}")
        for result in shard.get("results", []):
            unhashed_result = copy.deepcopy(result)
            recorded_result_hash = unhashed_result.pop("result_sha256", None)
            if sha256_bytes(canonical_json(unhashed_result)) != recorded_result_hash:
                failures.append(f"test result hash is invalid: {result.get('test_id')}")
            if result.get("shard") != shard.get("index") or result.get("shard") != shard_for(
                result.get("test_id", ""), profile["sharding"]["shard_count"]
            ):
                failures.append(f"test result shard assignment is invalid: {result.get('test_id')}")
            if result.get("final_status") != "pass" and result.get("classification") == "pass":
                failures.append(f"non-pass result was converted to pass: {result.get('test_id')}")
            for attempt in result.get("attempts", []):
                output = attempt.get("output", {})
                retained = output.get("retained_output", "")
                if len(retained.encode("utf-8")) > limit:
                    failures.append(f"retained output exceeds limit: {result.get('test_id')}")
                if SYNTHETIC_CANARY.search(retained):
                    failures.append(f"synthetic canary was not redacted: {result.get('test_id')}")
                if "raw_output" in output:
                    failures.append(f"raw output was retained: {result.get('test_id')}")
    if bundle["summary"] != summarize(results):
        failures.append("test result bundle summary does not reconcile with raw results")
    if bundle != build_bundle(profile, root):
        failures.append("test result bundle is stale or non-deterministic")
    return failures


def write_bundle(profile: dict[str, Any], output: Path, root: Path = ROOT) -> None:
    if output.exists():
        raise FileExistsError("test result bundle output already exists")
    output.parent.mkdir(parents=True, exist_ok=True)
    bundle = build_bundle(profile, root)
    descriptor, temporary_name = tempfile.mkstemp(prefix=".agentmage-result-", dir=output.parent)
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(canonical_json(bundle))
            handle.flush()
            os.fsync(handle.fileno())
        temporary.chmod(0o644)
        os.replace(temporary, output)
    except BaseException:
        temporary.unlink(missing_ok=True)
        raise


def build_report(root: Path = ROOT) -> dict[str, Any]:
    profile_path = root / PROFILE_PATH.relative_to(ROOT)
    profile = read_json(profile_path)
    bundle = build_bundle(profile, root)
    failures = validate_bundle(bundle, profile, root)
    if failures:
        raise ValueError("; ".join(failures))
    return {
        "schema_version": 1,
        "task_id": "2.1.1.8",
        "status": "pass",
        "profile": bundle["profile"],
        "bundle_preview": {
            "persisted": False,
            "bundle_sha256": bundle["bundle_sha256"],
            "context": bundle["context"],
            "shards": [
                {
                    "index": shard["index"],
                    "result_count": shard["result_count"],
                    "sha256": shard["shard_sha256"],
                }
                for shard in bundle["shards"]
            ],
            "summary": bundle["summary"],
        },
        "classification_closure": list(CLASSIFICATIONS),
        "max_retained_bytes_per_attempt": profile["output_policy"][
            "max_retained_bytes_per_attempt"
        ],
        "raw_output_retained": False,
        "synthetic_canary_values_retained": False,
        "content_addressing_claim": "tamper-evident-by-hash",
        "filesystem_immutability_claim": "none",
        "side_effect_contract": EXPECTED_SIDE_EFFECTS,
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }


def validate_report(report: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(report, dict):
        return ["test result bundle report must be an object"]
    failures: list[str] = []
    if report.get("schema_version") != 1 or report.get("task_id") != "2.1.1.8":
        failures.append("test result bundle report identity is invalid")
    if report.get("status") != "pass":
        failures.append("test result bundle generation did not pass")
    if report.get("raw_output_retained") is not False:
        failures.append("test result bundle report retained raw output")
    if report.get("synthetic_canary_values_retained") is not False:
        failures.append("test result bundle report retained synthetic canary values")
    if report.get("filesystem_immutability_claim") != "none":
        failures.append("test result bundle report overstated filesystem immutability")
    if report.get("macos_execution_status") != "blocked-macos" or report.get(
        "macos_support_claim"
    ) != "none":
        failures.append("test result bundle report made an invalid macOS claim")
    if report != build_report(root):
        failures.append("test result bundle report is stale or non-deterministic")
    return failures


def write_report(root: Path = ROOT) -> None:
    output = root / REPORT_PATH.relative_to(ROOT)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(canonical_json(build_report(root)))


def check_report(root: Path = ROOT) -> list[str]:
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read test result bundle report: {error}"]
    return validate_report(report, root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path)
    parser.add_argument("--write-report", action="store_true")
    args = parser.parse_args()
    try:
        profile = read_json(PROFILE_PATH)
        if args.output is not None:
            write_bundle(profile, args.output)
        if args.write_report:
            write_report()
        failures = check_report()
    except (OSError, ValueError) as error:
        print(f"test result bundle generation failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"test result bundle generation failed: {failure}", file=sys.stderr)
        return 1
    if args.output is None:
        print("deterministic sharded test result bundles validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
