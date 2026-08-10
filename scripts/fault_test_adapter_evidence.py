#!/usr/bin/env python3
"""Validate fault adapters and emit deterministic synthetic evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from collections import Counter
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from fixtures.fault_test_adapters import (  # noqa: E402
    RESOURCE_DIMENSIONS,
    AdversarialDisposition,
    AdversarialTestAdapter,
    CancellationToken,
    CrashMode,
    CrashTestAdapter,
    FaultAdapter,
    NetworkMode,
    NetworkTestAdapter,
    ResourceTestAdapter,
    trace_sha256,
)


PROFILE_PATH = ROOT / "fixtures" / "fault-test-adapter-profile.json"
REPORT_PATH = (
    ROOT
    / "artifacts"
    / "sprints"
    / "sprint-2"
    / "story-2.1"
    / "fault-test-adapter-report.json"
)
CORPUS_MANIFEST_PATH = ROOT / "fixtures" / "corpus" / "v1" / "manifest.json"
EXPECTED_BOUNDS = {
    "max_detail_bytes": 128,
    "max_events_per_adapter": 64,
    "max_payload_metadata_bytes": 1048576,
}
EXPECTED_EXECUTION_CONTRACT = {
    "executes_external_commands": False,
    "uses_network": False,
    "writes_filesystem": False,
    "exhausts_real_resources": False,
    "executes_adversarial_payloads": False,
    "reads_ambient_data": False,
    "retains_raw_payloads": False,
}
EXPECTED_CLEANUP_CONTRACT = {
    "close_is_idempotent": True,
    "operations_after_close_rejected": True,
    "resource_counters_zeroed": True,
}
EXPECTED_ADAPTERS = [
    "crash-test-adapter",
    "network-test-adapter",
    "resource-test-adapter",
    "adversarial-test-adapter",
]


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.tmp")
    temporary.write_bytes(content)
    temporary.replace(path)


def duplicate_values(values: list[str]) -> list[str]:
    seen: set[str] = set()
    duplicates: set[str] = set()
    for value in values:
        if value in seen:
            duplicates.add(value)
        seen.add(value)
    return sorted(duplicates)


def validate_profile(profile: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(profile, dict):
        return ["fault adapter profile must be an object"]
    failures: list[str] = []
    expected_fields = {
        "schema_version",
        "profile_id",
        "status",
        "generated_at",
        "bounds",
        "execution_contract",
        "crash_cases",
        "network_allowlist",
        "network_cases",
        "resource_limits",
        "resource_cases",
        "adversarial_cases",
        "cancellation_contract",
        "cleanup_contract",
        "macos_execution_status",
    }
    if set(profile) != expected_fields:
        failures.append("fault adapter profile fields do not match the contract")
        return failures
    if (
        profile["schema_version"] != 1
        or profile["profile_id"] != "agentmage-fault-test-adapters-v1"
        or profile["status"] != "synthetic-fault-adapter-contract"
        or profile["generated_at"] != "2024-01-01T00:04:00Z"
    ):
        failures.append("fault adapter profile identity is invalid")
    if profile["bounds"] != EXPECTED_BOUNDS:
        failures.append("fault adapter bounds drifted")
    if profile["execution_contract"] != EXPECTED_EXECUTION_CONTRACT:
        failures.append("fault adapter side-effect contract was weakened")
    if profile["cleanup_contract"] != EXPECTED_CLEANUP_CONTRACT:
        failures.append("fault adapter cleanup contract was weakened")
    if profile["macos_execution_status"] != "blocked-macos":
        failures.append("fault adapter profile lost blocked macOS status")

    crash_cases = profile["crash_cases"]
    crash_ids = [item.get("case_id", "") for item in crash_cases]
    crash_modes = [item.get("mode", "") for item in crash_cases]
    if duplicate_values(crash_ids) or crash_modes != [mode.value for mode in CrashMode]:
        failures.append("fault adapter crash-mode closure drifted")
    for item in crash_cases:
        try:
            mode = CrashMode(item["mode"])
        except (KeyError, ValueError):
            failures.append("fault adapter crash case is malformed")
            continue
        expected = {
            CrashMode.HEALTHY: ("succeeded", False),
            CrashMode.BEFORE_OPERATION: ("crashed", False),
            CrashMode.BEFORE_COMMIT: ("crashed", False),
            CrashMode.AFTER_COMMIT: ("crashed", True),
            CrashMode.UNCERTAIN_COMMIT: ("uncertain", None),
        }[mode]
        if (item.get("expected_status"), item.get("committed")) != expected:
            failures.append(f"fault adapter crash expectation drifted: {item.get('case_id')}")

    allowlist = profile["network_allowlist"]
    network_cases = profile["network_cases"]
    network_ids = [item.get("case_id", "") for item in network_cases]
    network_modes = [item.get("mode", "") for item in network_cases]
    if duplicate_values(allowlist) or duplicate_values(network_ids):
        failures.append("fault adapter network identities are not unique")
    if network_modes != [mode.value for mode in NetworkMode]:
        failures.append("fault adapter network-mode closure drifted")
    try:
        NetworkTestAdapter(allowlist)
    except ValueError as error:
        failures.append(str(error))
    for item in network_cases:
        if not NetworkTestAdapter._valid_endpoint(item.get("endpoint", "")):
            failures.append(f"fault adapter endpoint is invalid: {item.get('case_id')}")

    limits = profile["resource_limits"]
    if set(limits) != set(RESOURCE_DIMENSIONS):
        failures.append("fault adapter resource-limit closure drifted")
    resource_cases = profile["resource_cases"]
    resource_ids = [item.get("case_id", "") for item in resource_cases]
    if duplicate_values(resource_ids) or len(resource_cases) != len(RESOURCE_DIMENSIONS) + 1:
        failures.append("fault adapter resource-case closure drifted")
    exhausted = [
        item.get("exhausted_dimension")
        for item in resource_cases
        if item.get("expected_status") == "resource-exhausted"
    ]
    if exhausted != list(RESOURCE_DIMENSIONS):
        failures.append("fault adapter resource dimensions are not exhaustively covered")
    for item in resource_cases:
        if not ResourceTestAdapter._valid_demand(item.get("demand", {})):
            failures.append(f"fault adapter resource demand is invalid: {item.get('case_id')}")

    manifest = read_json(root / CORPUS_MANIFEST_PATH.relative_to(ROOT))
    archive_entries = {item["archive_path"]: item for item in manifest["entries"]}
    adversarial_cases = profile["adversarial_cases"]
    adversarial_ids = [item.get("case_id", "") for item in adversarial_cases]
    if duplicate_values(adversarial_ids) or len(adversarial_cases) != 5:
        failures.append("fault adapter adversarial-case closure drifted")
    if {item.get("disposition") for item in adversarial_cases} != {
        item.value for item in AdversarialDisposition
    }:
        failures.append("fault adapter adversarial dispositions are incomplete")
    for item in adversarial_cases:
        if item.get("archive_path") not in archive_entries:
            failures.append(
                f"fault adapter adversarial fixture is missing: {item.get('case_id')}"
            )

    cancellation = profile["cancellation_contract"]
    if cancellation != {
        "pre_cancelled_status": "cancelled",
        "adapters": EXPECTED_ADAPTERS,
    }:
        failures.append("fault adapter cancellation closure drifted")
    return failures


def adapter_bounds(profile: dict[str, Any]) -> dict[str, int]:
    return {
        "max_events": profile["bounds"]["max_events_per_adapter"],
        "max_detail_bytes": profile["bounds"]["max_detail_bytes"],
    }


def run_scenarios(profile: dict[str, Any], root: Path = ROOT) -> dict[str, Any]:
    failures = validate_profile(profile, root)
    if failures:
        raise ValueError("; ".join(failures))
    bounds = adapter_bounds(profile)
    manifest = read_json(root / CORPUS_MANIFEST_PATH.relative_to(ROOT))
    archive_entries = {item["archive_path"]: item for item in manifest["entries"]}

    crash = CrashTestAdapter(**bounds)
    network = NetworkTestAdapter(profile["network_allowlist"], **bounds)
    resource = ResourceTestAdapter(profile["resource_limits"], **bounds)
    adversarial = AdversarialTestAdapter(
        profile["bounds"]["max_payload_metadata_bytes"], **bounds
    )
    adapters: list[FaultAdapter] = [crash, network, resource, adversarial]

    for item in profile["crash_cases"]:
        outcome = crash.inject(item["checkpoint"], CrashMode(item["mode"]))
        if (
            outcome.status.value != item["expected_status"]
            or outcome.event.committed != item["committed"]
        ):
            raise ValueError(f"crash case did not meet expectation: {item['case_id']}")
    for item in profile["network_cases"]:
        outcome = network.request(item["endpoint"], NetworkMode(item["mode"]))
        if outcome.status.value != item["expected_status"]:
            raise ValueError(f"network case did not meet expectation: {item['case_id']}")
    for item in profile["resource_cases"]:
        outcome = resource.consume(item["case_id"], item["demand"])
        if outcome.status.value != item["expected_status"] or outcome.metadata.get(
            "exhausted_dimension"
        ) != item["exhausted_dimension"]:
            raise ValueError(f"resource case did not meet expectation: {item['case_id']}")
    for item in profile["adversarial_cases"]:
        archive_entry = archive_entries[item["archive_path"]]
        outcome = adversarial.inspect(
            case_id=item["case_id"],
            archive_path=item["archive_path"],
            classification=item["classification"],
            disposition=AdversarialDisposition(item["disposition"]),
            payload_sha256=archive_entry["sha256"],
            payload_bytes=archive_entry["bytes"],
        )
        if outcome.status.value != item["expected_status"]:
            raise ValueError(
                f"adversarial case did not meet expectation: {item['case_id']}"
            )

    token = CancellationToken()
    token.cancel()
    first_adversarial = profile["adversarial_cases"][0]
    first_entry = archive_entries[first_adversarial["archive_path"]]
    cancelled_adapters: list[FaultAdapter] = [
        CrashTestAdapter(**bounds),
        NetworkTestAdapter(profile["network_allowlist"], **bounds),
        ResourceTestAdapter(profile["resource_limits"], **bounds),
        AdversarialTestAdapter(
            profile["bounds"]["max_payload_metadata_bytes"], **bounds
        ),
    ]
    cancelled_outcomes = [
        cancelled_adapters[0].inject("cancel-check", CrashMode.HEALTHY, token),
        cancelled_adapters[1].request(
            profile["network_allowlist"][0], NetworkMode.ALLOWLISTED_SUCCESS, token
        ),
        cancelled_adapters[2].consume(
            "cancel-check", {key: 0 for key in RESOURCE_DIMENSIONS}, token
        ),
        cancelled_adapters[3].inspect(
            case_id="cancel-check",
            archive_path=first_adversarial["archive_path"],
            classification=first_adversarial["classification"],
            disposition=AdversarialDisposition(first_adversarial["disposition"]),
            payload_sha256=first_entry["sha256"],
            payload_bytes=first_entry["bytes"],
            token=token,
        ),
    ]
    if any(outcome.status.value != "cancelled" for outcome in cancelled_outcomes):
        raise ValueError("pre-cancelled adapter operation was not cancelled")
    adapters.extend(cancelled_adapters)

    records = [event.as_record() for adapter in adapters for event in adapter.events]
    status_counts = dict(sorted(Counter(item["status"] for item in records).items()))
    event_trace_sha256 = trace_sha256(adapters)
    for adapter in adapters:
        adapter.close()
        adapter.close()
    return {
        "adapters": adapters,
        "records": records,
        "status_counts": status_counts,
        "trace_sha256": event_trace_sha256,
        "resource_counters_zeroed": all(value == 0 for value in resource.used.values()),
    }


def build_report(root: Path = ROOT) -> dict[str, Any]:
    profile_path = root / PROFILE_PATH.relative_to(ROOT)
    profile = read_json(profile_path)
    run = run_scenarios(profile, root)
    records = run["records"]
    return {
        "schema_version": 1,
        "task_id": "2.1.2.4",
        "status": "pass",
        "profile": {
            "id": profile["profile_id"],
            "sha256": hashlib.sha256(profile_path.read_bytes()).hexdigest(),
        },
        "scenario_counts": {
            "crash": len(profile["crash_cases"]),
            "network": len(profile["network_cases"]),
            "resource": len(profile["resource_cases"]),
            "adversarial": len(profile["adversarial_cases"]),
            "cancellation": len(EXPECTED_ADAPTERS),
        },
        "event_count": len(records),
        "status_counts": run["status_counts"],
        "trace_sha256": run["trace_sha256"],
        "all_adapters_closed": all(adapter.closed for adapter in run["adapters"]),
        "resource_counters_zeroed": run["resource_counters_zeroed"],
        "side_effect_count": sum(len(item["side_effects"]) for item in records),
        "network_calls_performed": 0,
        "external_commands_performed": 0,
        "filesystem_writes_performed": 0,
        "real_resource_exhaustions_performed": 0,
        "adversarial_payloads_executed": 0,
        "raw_payloads_retained": False,
        "execution_contract": EXPECTED_EXECUTION_CONTRACT,
        "product_support_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }


def validate_report(report: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(report, dict):
        return ["fault adapter report must be an object"]
    failures: list[str] = []
    if report.get("schema_version") != 1 or report.get("task_id") != "2.1.2.4":
        failures.append("fault adapter report identity is invalid")
    if report.get("status") != "pass":
        failures.append("fault adapter report did not pass")
    zero_effect_fields = (
        "side_effect_count",
        "network_calls_performed",
        "external_commands_performed",
        "filesystem_writes_performed",
        "real_resource_exhaustions_performed",
        "adversarial_payloads_executed",
    )
    if any(report.get(field) != 0 for field in zero_effect_fields):
        failures.append("fault adapter report contains a real side effect")
    if report.get("raw_payloads_retained") is not False:
        failures.append("fault adapter report retained raw payloads")
    if report.get("product_support_claim") != "none":
        failures.append("fault adapter report made a product support claim")
    if report.get("macos_execution_status") != "blocked-macos" or report.get(
        "macos_support_claim"
    ) != "none":
        failures.append("fault adapter report made an invalid macOS claim")
    try:
        expected = build_report(root)
    except (OSError, ValueError, KeyError, TypeError) as error:
        failures.append(f"cannot rebuild fault adapter report: {error}")
    else:
        if report != expected:
            failures.append("fault adapter report is stale or non-deterministic")
    return failures


def write_report(root: Path = ROOT) -> None:
    write_atomic(root / REPORT_PATH.relative_to(ROOT), canonical_json(build_report(root)))


def check_report(root: Path = ROOT) -> list[str]:
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read fault adapter report: {error}"]
    return validate_report(report, root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            write_report()
        failures = check_report()
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"fault adapter validation failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"fault adapter validation failed: {failure}", file=sys.stderr)
        return 1
    print("crash, network, resource, and adversarial test adapters validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
