#!/usr/bin/env python3
"""Validate the fixed Gemma 4 cross-adapter feasibility corpus."""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path
from typing import Final


ROOT: Final = Path(__file__).resolve().parents[1]
DEFAULT_CORPUS: Final = ROOT / "model-profiles" / "evaluation" / "corpus-v1.json"
REQUIRED_ADAPTERS: Final = {
    "macos-native-metal",
    "linux-native-vulkan",
    "linux-docker-model-runner-cuda",
}
REQUIRED_CATEGORIES: Final = {
    "ordinary_chat",
    "repository_task",
    "citations",
    "tool_schema",
    "malformed_output_recovery",
    "cancellation",
    "context_limit",
    "latency",
    "memory",
    "zero_egress",
}
EXPECTED_THRESHOLDS: Final = {
    "schema_valid_rate": 1.0,
    "tool_call_valid_rate": 1.0,
    "citation_precision": 1.0,
    "citation_recall": 1.0,
    "repository_fact_accuracy": 0.95,
    "unsupported_action_rate": 0.0,
    "malformed_output_accepted_rate": 0.0,
    "cancellation_terminal_receipts": 1,
    "cancellation_max_seconds": 2.0,
    "minimum_generation_tokens_per_second": 10.0,
    "maximum_time_to_first_token_seconds": 10.0,
    "maximum_gpu_or_unified_memory_fraction": 0.9,
    "maximum_system_memory_fraction": 0.8,
    "maximum_swap_growth_bytes": 0,
    "post_install_egress_bytes": 0,
}


def load_corpus(path: Path = DEFAULT_CORPUS) -> dict[str, object]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError("model evaluation corpus must be an object")
    return value


def generate_context_fixture(seed: str, line_count: int) -> bytes:
    lines = [
        f"FACT-{index:04d}: value-{hashlib.sha256(f'{seed}:{index}'.encode()).hexdigest()[:16]}"
        for index in range(line_count)
    ]
    return ("\n".join(lines) + "\n").encode("utf-8")


def validate_corpus(corpus: dict[str, object]) -> list[str]:
    failures: list[str] = []
    required = {
        "schema_version",
        "record_type",
        "corpus_id",
        "version",
        "status",
        "created_at",
        "profile_id",
        "data_classification",
        "contains_user_data",
        "threshold_change_policy",
        "adapters",
        "decoder",
        "fixture_generation",
        "global_thresholds",
        "cases",
    }
    if set(corpus) != required:
        failures.append("corpus top-level fields do not match the schema")
        return failures
    if corpus["schema_version"] != 1 or corpus["version"] != "1.0.0":
        failures.append("unsupported corpus schema or version")
    if corpus["profile_id"] != "gemma-4-e4b-it":
        failures.append("corpus profile identity is incorrect")
    if corpus["data_classification"] != "public_synthetic_only" or corpus["contains_user_data"] is not False:
        failures.append("corpus must contain public synthetic data only")

    adapters = corpus["adapters"]
    if not isinstance(adapters, list):
        failures.append("adapters must be an array")
    else:
        adapter_ids = [item.get("id") for item in adapters if isinstance(item, dict)]
        if len(adapter_ids) != len(set(adapter_ids)):
            failures.append("duplicate adapter identity")
        if set(adapter_ids) != REQUIRED_ADAPTERS:
            failures.append("cross-adapter matrix is incomplete")
        for adapter in adapters:
            if not isinstance(adapter, dict):
                continue
            runtime_identity = adapter.get("runtime_commit") or adapter.get("runtime_digest")
            if not runtime_identity:
                failures.append(f"adapter {adapter.get('id')} lacks immutable runtime identity")

    decoder = corpus["decoder"]
    if not isinstance(decoder, dict):
        failures.append("decoder contract must be an object")
    else:
        expected_decoder = {
            "seed": 4242,
            "temperature": 0.0,
            "top_p": 1.0,
            "top_k": 1,
            "max_output_tokens": 512,
            "operational_context_tokens": 8192,
        }
        for field, expected in expected_decoder.items():
            if decoder.get(field) != expected:
                failures.append(f"decoder setting changed: {field}")

    fixture = corpus["fixture_generation"]
    if not isinstance(fixture, dict):
        failures.append("fixture-generation contract must be an object")
    else:
        generated = generate_context_fixture(str(fixture.get("seed")), int(fixture.get("line_count", 0)))
        if len(generated) != fixture.get("byte_count"):
            failures.append("generated context fixture byte count changed")
        if hashlib.sha256(generated).hexdigest() != fixture.get("sha256"):
            failures.append("generated context fixture hash changed")

    if corpus["global_thresholds"] != EXPECTED_THRESHOLDS:
        failures.append("fixed feasibility thresholds changed")

    cases = corpus["cases"]
    if not isinstance(cases, list):
        failures.append("corpus cases must be an array")
    else:
        case_ids = [item.get("id") for item in cases if isinstance(item, dict)]
        if len(case_ids) != len(set(case_ids)):
            failures.append("duplicate corpus case identity")
        categories = {item.get("category") for item in cases if isinstance(item, dict)}
        if categories != REQUIRED_CATEGORIES:
            failures.append("required corpus category coverage is incomplete")
        for case in cases:
            if not isinstance(case, dict):
                failures.append("corpus case must be an object")
                continue
            required_case = {
                "id",
                "category",
                "title",
                "adapters",
                "trials",
                "input",
                "expected",
                "measurements",
            }
            if set(case) != required_case:
                failures.append(f"corpus case {case.get('id')} fields do not match schema")
            if case.get("adapters") != "all":
                failures.append(f"corpus case {case.get('id')} does not run on all adapters")
            if not isinstance(case.get("trials"), int) or case["trials"] <= 0:
                failures.append(f"corpus case {case.get('id')} has invalid trial count")
            if not case.get("measurements"):
                failures.append(f"corpus case {case.get('id')} lacks measurements")
    return failures


def main() -> int:
    try:
        corpus = load_corpus()
        failures = validate_corpus(corpus)
    except (OSError, json.JSONDecodeError, ValueError, TypeError) as error:
        print(f"Model corpus validation failed: {error}", file=sys.stderr)
        return 1
    if failures:
        print("Model corpus validation failed:", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1
    print(f"Validated {len(corpus['cases'])} fixed cross-adapter corpus cases.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
