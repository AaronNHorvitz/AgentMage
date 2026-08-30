#!/usr/bin/env python3
"""Build and verify reconstructible context-delivery receipts for Story 2.4.2.3."""

from __future__ import annotations

import argparse
import copy
import io
import json
import sys
import zipfile
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.engineering_artifact_admission_fixtures import (  # noqa: E402
    ZERO_SHA256,
    build_archive,
    canonical_json,
    sealed,
    sha256_bytes,
    validate_archive,
    write_atomic,
)


FIXTURE_DIR: Final = ROOT / "fixtures" / "artifact-admission" / "v1"
ACCOUNTING_PATH: Final = FIXTURE_DIR / "context-accounting-manifests.json"
ADMISSION_MANIFEST_PATH: Final = FIXTURE_DIR / "manifest.json"
ADMISSION_ARCHIVE_PATH: Final = FIXTURE_DIR / "artifact-admission-corpus-v1.zip"
PAYLOAD_ARCHIVE_PATH: Final = FIXTURE_DIR / "context-delivery-payloads-v1.zip"
OUTPUT_PATH: Final = FIXTURE_DIR / "context-delivery-receipts.json"
BLOCKED_SCENARIOS: Final = {
    "token_budget_overflow",
    "stale",
    "restricted",
    "model_profile_change",
}


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def admission_bytes() -> dict[str, bytes]:
    manifest = read_json(ADMISSION_MANIFEST_PATH)
    with zipfile.ZipFile(ADMISSION_ARCHIVE_PATH) as archive:
        return {
            case["fixture_id"]: archive.read(case["archive_entry"])
            for case in manifest["cases"]
            if case["archive_entry"] is not None
        }


def delivery_content(scenario: dict[str, Any], item: dict[str, Any], sources: dict[str, bytes]) -> bytes:
    scenario_id = scenario["scenario_id"]
    fixture_id = scenario["target_fixture_id"]
    if item["disposition"] == "summarized":
        return f"Synthetic bounded summary for {item['artifact_id']}.\n".encode("utf-8")
    if len(item["ranges"]) != 1:
        raise ValueError(f"admitting delivery item does not have one exact range: {scenario_id}")
    byte_range = item["ranges"][0]
    content = sources[fixture_id]
    return content[byte_range["start_byte"] : byte_range["end_byte_exclusive"]]


def payload_entries(accounting: dict[str, Any]) -> dict[str, bytes]:
    sources = admission_bytes()
    entries: dict[str, bytes] = {}
    for scenario in accounting["scenarios"]:
        for item in scenario["context_manifest"]["items"]:
            if item["disposition"] not in {"included", "summarized", "truncated"}:
                continue
            path = f"delivered/{scenario['scenario_id']}/{item['artifact_id']}.bin"
            entries[path] = delivery_content(scenario, item, sources)
    return dict(sorted(entries.items()))


def model_visible_set_sha256(items: list[dict[str, Any]], entries: dict[str, bytes]) -> str:
    framed = bytearray()
    for item in items:
        path_bytes = item["archive_entry"].encode("utf-8")
        content = entries[item["archive_entry"]]
        framed.extend(len(path_bytes).to_bytes(8, "big"))
        framed.extend(path_bytes)
        framed.extend(len(content).to_bytes(8, "big"))
        framed.extend(content)
    return sha256_bytes(bytes(framed))


def scenario_receipt(scenario: dict[str, Any], entries: dict[str, bytes]) -> dict[str, Any]:
    manifest = scenario["context_manifest"]
    delivery_map: list[dict[str, Any]] = []
    delivered: list[dict[str, Any]] = []
    for item in manifest["items"]:
        if item["disposition"] not in {"included", "summarized", "truncated"}:
            continue
        path = f"delivered/{scenario['scenario_id']}/{item['artifact_id']}.bin"
        content = entries[path]
        delivered_range = {"start_byte": 0, "end_byte_exclusive": len(content)}
        record = {
            "artifact_id": item["artifact_id"],
            "sha256": sha256_bytes(content),
            "range": delivered_range,
            "token_count": item["token_count"],
        }
        delivered.append(record)
        delivery_map.append(
            {
                **record,
                "archive_entry": path,
                "manifest_disposition": item["disposition"],
                "source_ranges": item["ranges"],
            }
        )
    blocked = scenario["scenario_id"] in BLOCKED_SCENARIOS
    required_unseen = [scenario["target_artifact_id"]] if blocked else []
    receipt = sealed(
        {
            "schema_version": 1,
            "receipt_id": f"receipt-{scenario['scenario_id']}-v1",
            "context_manifest_id": manifest["context_manifest_id"],
            "context_manifest_sha256": manifest["manifest_sha256"],
            "model_request_id": f"model-request-{scenario['scenario_id']}-v1",
            "route_decision_id": "route-context-delivery-fixture-v1",
            "delivered": delivered,
            "required_unseen_artifact_ids": required_unseen,
            "outcome": "blocked" if blocked else "delivered",
            "receipt_sha256": ZERO_SHA256,
        },
        "receipt_sha256",
    )
    return {
        "scenario_id": scenario["scenario_id"],
        "context_manifest_id": manifest["context_manifest_id"],
        "context_manifest_sha256": manifest["manifest_sha256"],
        "delivery_map": delivery_map,
        "model_visible_item_count": len(delivery_map),
        "model_visible_set_sha256": model_visible_set_sha256(delivery_map, entries),
        "required_authoritative_artifact_ids": [scenario["target_artifact_id"]] if blocked else [],
        "completion_allowed": not blocked,
        "completion_block_reason": (
            "required_authoritative_content_not_delivered" if blocked else None
        ),
        "receipt": receipt,
        "execution_claim": "synthetic_delivery_reconstruction_only",
    }


def build_suite() -> tuple[bytes, dict[str, bytes], dict[str, Any]]:
    accounting = read_json(ACCOUNTING_PATH)
    entries = payload_entries(accounting)
    archive = build_archive(entries)
    value = {
        "schema_version": 1,
        "suite_id": "engineering-context-delivery-v1",
        "task_id": "2.4.2.3",
        "generated_on": "2026-08-29",
        "status": "synthetic-reconstructible-delivery-contract",
        "synthetic_only": True,
        "model_request_executed": False,
        "product_delivery_claim": "none",
        "accounting_suite": {
            "path": str(ACCOUNTING_PATH.relative_to(ROOT)),
            "sha256": sha256_bytes(ACCOUNTING_PATH.read_bytes()),
        },
        "payload_archive": {
            "path": str(PAYLOAD_ARCHIVE_PATH.relative_to(ROOT)),
            "format": "zip-stored",
            "entry_count": len(entries),
            "byte_length": len(archive),
            "sha256": sha256_bytes(archive),
        },
        "generator": {
            "path": "scripts/engineering_context_delivery_fixtures.py",
            "sha256": sha256_bytes(Path(__file__).read_bytes()),
        },
        "scenario_count": len(accounting["scenarios"]),
        "blocked_scenarios": sorted(BLOCKED_SCENARIOS),
        "scenarios": [scenario_receipt(scenario, entries) for scenario in accounting["scenarios"]],
    }
    value["suite_sha256"] = sha256_bytes(canonical_json(value))
    return archive, entries, value


def validate_suite(value: Any, archive: bytes, entries: dict[str, bytes]) -> list[str]:
    if not isinstance(value, dict):
        return ["context delivery suite must be an object"]
    failures: list[str] = []
    unhashed = copy.deepcopy(value)
    recorded = unhashed.pop("suite_sha256", None)
    if recorded != sha256_bytes(canonical_json(unhashed)):
        failures.append("context delivery suite self-hash is invalid")
    if value != build_suite()[2]:
        failures.append("context delivery suite is stale, incomplete, reordered, or widened")
    if value.get("payload_archive", {}).get("sha256") != sha256_bytes(archive):
        failures.append("context delivery payload archive identity is invalid")
    scenarios = value.get("scenarios", [])
    if value.get("scenario_count") != len(scenarios):
        failures.append("context delivery scenario count is incomplete")
    for scenario in scenarios if isinstance(scenarios, list) else []:
        receipt = scenario.get("receipt", {})
        delivery_map = scenario.get("delivery_map", [])
        if len(delivery_map) != scenario.get("model_visible_item_count") or len(delivery_map) != len(receipt.get("delivered", [])):
            failures.append(f"context delivery item count drifted: {scenario.get('scenario_id')}")
        if scenario.get("model_visible_set_sha256") != model_visible_set_sha256(delivery_map, entries):
            failures.append(f"context delivery set cannot be reconstructed: {scenario.get('scenario_id')}")
        for mapped, delivered in zip(delivery_map, receipt.get("delivered", []), strict=False):
            content = entries.get(mapped.get("archive_entry", ""))
            if content is None or delivered.get("sha256") != sha256_bytes(content) or mapped.get("sha256") != delivered.get("sha256"):
                failures.append(f"context delivery bytes do not match receipt: {scenario.get('scenario_id')}")
        blocked = bool(scenario.get("required_authoritative_artifact_ids"))
        if blocked != (receipt.get("outcome") == "blocked") or blocked == scenario.get("completion_allowed"):
            failures.append(f"context completion blocking drifted: {scenario.get('scenario_id')}")
        if blocked and set(receipt.get("required_unseen_artifact_ids", [])) != set(scenario.get("required_authoritative_artifact_ids", [])):
            failures.append(f"required unseen authority is incomplete: {scenario.get('scenario_id')}")
    if value.get("synthetic_only") is not True or value.get("model_request_executed") is not False or value.get("product_delivery_claim") != "none":
        failures.append("context delivery suite makes a product execution or delivery overclaim")
    return failures


def check() -> list[str]:
    try:
        expected_archive, entries, expected_suite = build_suite()
        actual_archive = PAYLOAD_ARCHIVE_PATH.read_bytes()
        actual_suite = read_json(OUTPUT_PATH)
    except (OSError, ValueError, json.JSONDecodeError, zipfile.BadZipFile) as error:
        return [f"cannot validate context delivery suite: {error}"]
    failures: list[str] = []
    if actual_archive != expected_archive:
        failures.append("checked context delivery payload archive is stale or corrupt")
    failures.extend(validate_archive(actual_archive, entries))
    failures.extend(validate_suite(actual_suite, actual_archive, entries))
    if actual_suite != expected_suite:
        failures.append("checked context delivery suite is stale")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    if args.write:
        archive, _entries, suite = build_suite()
        write_atomic(PAYLOAD_ARCHIVE_PATH, archive)
        write_atomic(OUTPUT_PATH, canonical_json(suite))
    failures = check()
    if failures:
        for failure in failures:
            print(f"Context delivery fixture validation failed: {failure}", file=sys.stderr)
        return 1
    suite = read_json(OUTPUT_PATH)
    print(f"Validated {suite['scenario_count']} reconstructible context-delivery receipts")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
