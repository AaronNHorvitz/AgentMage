#!/usr/bin/env python3
"""Exact benchmark comparability, statistics, and claim-lint contracts."""

from __future__ import annotations

import math
from typing import Any, Iterable, NamedTuple

TUPLE_FIELDS = (
    "artifact_sha256",
    "tokenizer_sha256",
    "template_sha256",
    "codec_sha256",
    "runtime_sha256",
    "sampler_sha256",
    "context_tokens",
    "inference_slots",
    "platform",
    "hardware_sha256",
    "driver_sha256",
    "tool_catalog_sha256",
    "grader_sha256",
    "corpus_sha256",
)

MEASURED_DISPOSITIONS = {"PASS", "FAILED", "REJECTED"}
NON_MEASURED_DISPOSITIONS = {
    "BLOCKED",
    "BLOCKED-HARDWARE",
    "STALE",
    "INCOMPARABLE",
    "UNKNOWN",
    "NOT-APPLICABLE",
}
PROHIBITED_CLAIMS = {
    "family_winner",
    "family_wide_support",
    "universal_determinism",
    "unrecorded_hardware_fit",
    "hidden_failure_omission",
}


class Comparison(NamedTuple):
    comparable: bool
    differing_fields: tuple[str, ...]
    label: str


def compare_tuples(left: dict[str, Any], right: dict[str, Any]) -> Comparison:
    validate_tuple(left)
    validate_tuple(right)
    differences = tuple(field for field in TUPLE_FIELDS if left[field] != right[field])
    return Comparison(
        comparable=not differences,
        differing_fields=differences,
        label="COMPARABLE" if not differences else "INCOMPARABLE-TUPLE-DRIFT",
    )


def validate_tuple(value: dict[str, Any]) -> None:
    if set(value) != set(TUPLE_FIELDS):
        raise ValueError("benchmark tuple has missing or extra fields")


def repeated_trial_statistics(outcomes: Iterable[bool], k: int) -> dict[str, Any]:
    values = tuple(bool(value) for value in outcomes)
    n = len(values)
    if n == 0 or k <= 0 or k > n:
        raise ValueError("invalid repeated-trial population or k")
    successes = sum(values)
    failures = n - successes
    p = successes / n
    pass_at_k = 1.0 if failures < k else 1.0 - math.comb(failures, k) / math.comb(n, k)
    pass_to_the_k = 0.0 if successes < k else math.comb(successes, k) / math.comb(n, k)
    variance = sum((int(value) - p) ** 2 for value in values) / n
    z = 1.959963984540054
    denominator = 1.0 + z * z / n
    center = (p + z * z / (2 * n)) / denominator
    margin = z * math.sqrt((p * (1 - p) + z * z / (4 * n)) / n) / denominator
    return {
        "trial_count": n,
        "pass_at_one": p,
        "pass_at_k": pass_at_k,
        "pass_to_the_k": pass_to_the_k,
        "confidence_interval": [max(0.0, center - margin), min(1.0, center + margin)],
        "variance": variance,
    }


def validate_benchmark_record(record: dict[str, Any]) -> None:
    disposition = record.get("disposition")
    if disposition not in MEASURED_DISPOSITIONS | NON_MEASURED_DISPOSITIONS:
        raise ValueError("unsupported benchmark disposition")
    validate_tuple(record.get("tuple", {}))
    statistics = record.get("statistics")
    if not isinstance(statistics, dict):
        raise ValueError("benchmark statistics are required")
    values = tuple(statistics.values())
    if disposition in MEASURED_DISPOSITIONS:
        if any(value is None for value in record["tuple"].values()):
            raise ValueError("measured result has an incomplete tuple")
        if statistics.get("trial_count") is None or statistics["trial_count"] < 1:
            raise ValueError("measured result has no repeated trials")
    elif any(value is not None for value in values):
        raise ValueError("non-measured disposition cannot carry invented statistics")
    if not record.get("limitations"):
        raise ValueError("benchmark limitations are required")


def lint_claims(claims: Iterable[dict[str, Any]]) -> list[str]:
    failures = []
    for index, claim in enumerate(claims):
        kind = claim.get("kind")
        supported = claim.get("supported_by_exact_result") is True
        scope = claim.get("scope")
        if kind in PROHIBITED_CLAIMS or not supported or scope != "exact_recorded_tuple":
            failures.append(f"claim-{index}-unsupported")
    return failures
