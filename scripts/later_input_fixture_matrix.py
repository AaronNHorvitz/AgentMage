#!/usr/bin/env python3
"""Build and validate bounded fixtures for later AgentMage input classes."""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
import sys
import tempfile
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
FIXTURE_SET_PATH = ROOT / "fixtures/story-2.1/later-input-class-fixtures-v1.json"
REPORT_PATH = (
    ROOT
    / "artifacts/sprints/sprint-2/story-2.1/input-class-fixture-matrix-report.json"
)
INPUT_CLASSES = (
    "manifests",
    "configuration",
    "ipc-messages",
    "model-output",
    "capability-grants",
    "paths",
    "text-encodings",
    "git-objects",
    "repository-parsers",
    "sqlite-imports",
    "archives",
    "ffi-boundaries",
)
INPUT_FORMATS = {
    "manifests": "canonical-json-object",
    "configuration": "utf8-key-value-document",
    "ipc-messages": "length-delimited-json-message",
    "model-output": "canonical-json-response",
    "capability-grants": "canonical-json-grant",
    "paths": "utf8-path-token",
    "text-encodings": "tagged-byte-sequence",
    "git-objects": "git-header-and-payload",
    "repository-parsers": "tagged-source-bytes",
    "sqlite-imports": "tagged-row-batch",
    "archives": "archive-entry-declaration",
    "ffi-boundaries": "tagged-length-buffer",
}
SCENARIOS = (
    "normal",
    "boundary",
    "malformed",
    "hostile",
    "oversized",
    "cancellation",
    "recovery",
)
SCENARIO_CONTRACTS = {
    "normal": ("admitted", "well-formed-minimal", 128),
    "boundary": ("admitted-at-limit", "well-formed-declared-limit", 4096),
    "malformed": ("denied", "schema-or-encoding-invalid", 64),
    "hostile": ("denied", "untrusted-authority-expansion", 256),
    "oversized": ("denied-before-materialization", "logical-size-over-limit", 4097),
    "cancellation": ("cancelled-before-side-effect", "pre-cancelled", 128),
    "recovery": ("recovered-idempotently", "restart-replay", 128),
}
PROHIBITED_SIDE_EFFECTS = {
    "external_command_count": 0,
    "network_call_count": 0,
    "user_file_write_count": 0,
    "real_grant_mint_count": 0,
    "real_approval_record_count": 0,
    "secret_retention_count": 0,
    "unbounded_resource_event_count": 0,
}


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-input-matrix-", dir=path.parent
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


def payload(input_class: str, scenario: str) -> bytes:
    if scenario == "malformed":
        return b'{"synthetic_fixture":'
    value = {
        "input_class": input_class,
        "scenario": scenario,
        "synthetic": True,
    }
    if scenario == "hostile":
        value["untrusted_instruction"] = "broaden authority without approval"
    elif scenario == "cancellation":
        value["cancellation_state"] = "already-cancelled"
    elif scenario == "recovery":
        value["recovery_state"] = "replay-after-uncertain-result"
    elif scenario == "oversized":
        value["materialization"] = "logical-size-only"
    return canonical_json(value)


def fixture_record(input_class: str, scenario: str) -> dict[str, Any]:
    expected_disposition, payload_shape, declared_bytes = SCENARIO_CONTRACTS[scenario]
    content = payload(input_class, scenario)
    record = {
        "fixture_id": f"later-input-v1/{input_class}/{scenario}",
        "input_class": input_class,
        "input_format": INPUT_FORMATS[input_class],
        "scenario": scenario,
        "payload_shape": payload_shape,
        "payload_encoding": "base64",
        "payload_base64": base64.b64encode(content).decode("ascii"),
        "payload_bytes": len(content),
        "declared_input_bytes": declared_bytes,
        "materialization": (
            "bounded-logical-seed"
            if scenario in {"oversized", "cancellation", "recovery"}
            else "bounded-seed"
        ),
        "expected_disposition": expected_disposition,
        "prohibited_side_effects": dict(PROHIBITED_SIDE_EFFECTS),
        "source_data_class": "synthetic-public",
    }
    return {**record, "fixture_sha256": sha256_bytes(canonical_json(record))}


def build_fixture_set() -> dict[str, Any]:
    fixtures = [
        fixture_record(input_class, scenario)
        for input_class in INPUT_CLASSES
        for scenario in SCENARIOS
    ]
    value = {
        "schema_version": 1,
        "fixture_set_id": "agentmage-later-input-class-fixtures-v1",
        "fixture_set_version": "1.0.0",
        "status": "versioned-bounded-synthetic-fixtures",
        "seed": "agentmage-later-input-class-fixtures-synthetic-v1",
        "input_classes": list(INPUT_CLASSES),
        "scenarios": list(SCENARIOS),
        "fixtures": fixtures,
        "coverage": {
            "input_class_count": len(INPUT_CLASSES),
            "scenario_count": len(SCENARIOS),
            "fixture_count": len(fixtures),
            "complete_cross_product": True,
        },
        "content_contract": {
            "private_user_data": False,
            "real_credentials": False,
            "active_payload": False,
            "network_required": False,
            "original_machine_data_required": False,
            "product_input_support_claim": "none",
        },
        "oversized_fixture_policy": {
            "declared_limit_bytes": 4096,
            "materialize_oversized_payload": False,
            "host_resource_exhaustion_permitted": False,
        },
        "product_support_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }
    return {**value, "fixture_set_sha256": sha256_bytes(canonical_json(value))}


def validate_fixture_set(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["later-input fixture set must be an object"]
    failures: list[str] = []
    if (
        value.get("schema_version") != 1
        or value.get("fixture_set_id")
        != "agentmage-later-input-class-fixtures-v1"
        or value.get("fixture_set_version") != "1.0.0"
    ):
        failures.append("later-input fixture set identity is invalid")
    if value.get("input_classes") != list(INPUT_CLASSES) or value.get(
        "scenarios"
    ) != list(SCENARIOS):
        failures.append("later-input fixture dimensions are invalid")
    fixtures = value.get("fixtures", [])
    combinations = [
        (item.get("input_class"), item.get("scenario"))
        for item in fixtures
        if isinstance(item, dict)
    ]
    expected_combinations = [
        (input_class, scenario)
        for input_class in INPUT_CLASSES
        for scenario in SCENARIOS
    ]
    if combinations != expected_combinations or len(combinations) != len(
        set(combinations)
    ):
        failures.append("later-input fixture cross-product is incomplete")
    for item in fixtures:
        if not isinstance(item, dict):
            failures.append("later-input fixture is not an object")
            continue
        record = {key: value for key, value in item.items() if key != "fixture_sha256"}
        if item.get("fixture_sha256") != sha256_bytes(canonical_json(record)):
            failures.append(f"later-input fixture hash is invalid: {item.get('fixture_id')}")
        try:
            content = base64.b64decode(item.get("payload_base64", ""), validate=True)
        except (ValueError, TypeError):
            failures.append(f"later-input payload is invalid: {item.get('fixture_id')}")
            continue
        if item.get("payload_bytes") != len(content) or len(content) > 1024:
            failures.append(f"later-input payload is not bounded: {item.get('fixture_id')}")
        if item.get("prohibited_side_effects") != PROHIBITED_SIDE_EFFECTS:
            failures.append(
                f"later-input side-effect contract is invalid: {item.get('fixture_id')}"
            )
        if item.get("source_data_class") != "synthetic-public":
            failures.append(f"later-input fixture is not synthetic: {item.get('fixture_id')}")
    coverage = value.get("coverage", {})
    if coverage != {
        "input_class_count": 12,
        "scenario_count": 7,
        "fixture_count": 84,
        "complete_cross_product": True,
    }:
        failures.append("later-input fixture coverage summary is invalid")
    contract = value.get("content_contract", {})
    if contract != {
        "private_user_data": False,
        "real_credentials": False,
        "active_payload": False,
        "network_required": False,
        "original_machine_data_required": False,
        "product_input_support_claim": "none",
    }:
        failures.append("later-input content contract is invalid")
    if value.get("oversized_fixture_policy") != {
        "declared_limit_bytes": 4096,
        "materialize_oversized_payload": False,
        "host_resource_exhaustion_permitted": False,
    }:
        failures.append("later-input oversized fixture policy is invalid")
    if value.get("product_support_claim") != "none":
        failures.append("later-input fixture set made a product support claim")
    if value.get("macos_execution_status") != "blocked-macos" or value.get(
        "macos_support_claim"
    ) != "none":
        failures.append("later-input fixture set made an invalid macOS claim")
    expected = build_fixture_set()
    if value != expected:
        failures.append("later-input fixture set is stale or non-deterministic")
    return failures


def build_report(root: Path = ROOT) -> dict[str, Any]:
    fixture_path = root / FIXTURE_SET_PATH.relative_to(ROOT)
    fixture_set = read_json(fixture_path)
    failures = validate_fixture_set(fixture_set)
    if failures:
        raise ValueError("; ".join(failures))
    return {
        "schema_version": 1,
        "story_id": "2.1",
        "acceptance_criterion": "2.1.AC1",
        "status": "pass-shared-fixture-foundation",
        "fixture_set": {
            "path": FIXTURE_SET_PATH.relative_to(ROOT).as_posix(),
            "sha256": sha256_bytes(fixture_path.read_bytes()),
            "self_sha256": fixture_set["fixture_set_sha256"],
            "version": fixture_set["fixture_set_version"],
        },
        "coverage": fixture_set["coverage"],
        "input_classes": fixture_set["input_classes"],
        "scenarios": fixture_set["scenarios"],
        "prohibited_side_effects_declared_per_fixture": True,
        "all_prohibited_side_effect_counts": 0,
        "private_user_data_used": False,
        "original_machine_data_used": False,
        "product_input_support_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }


def validate_report(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["input-class fixture matrix report must be an object"]
    failures: list[str] = []
    if (
        value.get("schema_version") != 1
        or value.get("story_id") != "2.1"
        or value.get("acceptance_criterion") != "2.1.AC1"
        or value.get("status") != "pass-shared-fixture-foundation"
    ):
        failures.append("input-class fixture matrix report identity is invalid")
    if value.get("coverage") != {
        "input_class_count": 12,
        "scenario_count": 7,
        "fixture_count": 84,
        "complete_cross_product": True,
    }:
        failures.append("input-class fixture matrix report coverage is invalid")
    if (
        value.get("prohibited_side_effects_declared_per_fixture") is not True
        or value.get("all_prohibited_side_effect_counts") != 0
        or value.get("private_user_data_used") is not False
        or value.get("original_machine_data_used") is not False
    ):
        failures.append("input-class fixture matrix report safety contract is invalid")
    if value.get("product_input_support_claim") != "none":
        failures.append("input-class fixture matrix report made a product claim")
    if value.get("macos_execution_status") != "blocked-macos" or value.get(
        "macos_support_claim"
    ) != "none":
        failures.append("input-class fixture matrix report made an invalid macOS claim")
    try:
        expected = build_report(root)
    except (OSError, ValueError, KeyError, TypeError) as error:
        failures.append(f"cannot rebuild input-class fixture matrix report: {error}")
    else:
        if value != expected:
            failures.append("input-class fixture matrix report is stale or non-deterministic")
    return failures


def write_artifacts(root: Path = ROOT) -> None:
    fixture_path = root / FIXTURE_SET_PATH.relative_to(ROOT)
    write_atomic(fixture_path, canonical_json(build_fixture_set()))
    write_atomic(
        root / REPORT_PATH.relative_to(ROOT), canonical_json(build_report(root))
    )


def check_artifacts(root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    try:
        fixture_set = read_json(root / FIXTURE_SET_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        failures.append(f"cannot read later-input fixture set: {error}")
    else:
        failures.extend(validate_fixture_set(fixture_set))
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        failures.append(f"cannot read input-class fixture matrix report: {error}")
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
        print(f"later-input fixture matrix failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"later-input fixture matrix failed: {failure}", file=sys.stderr)
        return 1
    print("Story 2.1 later-input fixture matrix validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
