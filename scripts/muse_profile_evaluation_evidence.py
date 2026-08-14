#!/usr/bin/env python3
"""Retain bounded, content-free Muse quality and repeatability evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
import subprocess
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-13/story-13.3/muse-profile-evaluation.json"
QUALITY_PROFILE: Final = "muse-glimmer-30b-q4-k-m-text-8k-fedora-first-party-quality"
REPEAT_PROFILE: Final = (
    "muse-glimmer-30b-q4-k-m-text-8k-fedora-diagnostic-repeatability"
)
QUALITY_MANIFEST: Final = "ee8353ddf9abb244db9ecf871962a2b3039e920ee247028f14e127299395522d"
REPEAT_MANIFEST: Final = "ec9537e5e7bd48001837da25d91e1ae6829c5fa7b76a1db9243613bc930aaeb9"
MODEL_SHA256: Final = "4cc57c0f51040a226e5a72cc47b7613f7772950e460a665f7083de89f183f60e"
RUNTIME_SHA256: Final = "3b1194ef38f4b02b6329d698e29532435a5a7c3567c84b8bb822459ca0893286"
EXPECTED_CASES: Final = {
    "FACT-001": "factual",
    "CODE-001": "coding",
    "TOOL-001": "tool",
    "INJECT-001": "prompt_injection",
}
SOURCE_PATHS: Final = (
    "kernel/contracts/src/model.rs",
    "kernel/engine/src/model_runtime.rs",
    "platforms/linux-inference/src/llama_server_driver.rs",
    "platforms/linux-inference/src/muse_atem_codec.rs",
    "platforms/linux-inference/tests/muse_live_evaluation.rs",
    "model-profiles/exact-profile-catalog.json",
    "scripts/muse_profile_evaluation_evidence.py",
    "tests/test_muse_profile_evaluation_evidence.py",
)
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def git_file(revision: str, relative: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{relative}"],
        cwd=ROOT,
        check=False,
        capture_output=True,
        timeout=10,
    )
    if result.returncode != 0:
        raise ValueError(f"committed evaluation source is absent: {relative}")
    return result.stdout


def wilson_interval(successes: int, trials: int) -> list[float]:
    if trials <= 0:
        raise ValueError("trial count must be positive")
    z = 1.959963984540054
    observed = successes / trials
    denominator = 1 + z * z / trials
    center = (observed + z * z / (2 * trials)) / denominator
    spread = z * math.sqrt(
        observed * (1 - observed) / trials + z * z / (4 * trials * trials)
    ) / denominator
    return [round(max(0.0, center - spread), 6), round(min(1.0, center + spread), 6)]


def validate_summary(summary: Any) -> list[str]:
    if not isinstance(summary, dict):
        return ["evaluation summary must be an object"]
    failures: list[str] = []
    if set(summary) != {
        "schema_version",
        "record_type",
        "quality_profile_id",
        "quality_manifest_sha256",
        "repeatability_profile_id",
        "repeatability_manifest_sha256",
        "quality_trial_count",
        "quality_closed_proposal_count",
        "quality_pass_count",
        "false_completion_count",
        "case_summaries",
        "repeatability_trial_count",
        "repeatability_unique_response_count",
        "repeatability_response_sha256",
        "maximum_resident_memory_bytes",
        "maximum_accelerator_memory_bytes",
        "socket_residue_count",
        "raw_output_retained",
    }:
        return ["evaluation summary fields are not closed"]
    if (
        summary.get("schema_version") != 1
        or summary.get("record_type") != "muse_live_profile_evaluation_summary"
        or summary.get("quality_profile_id") != QUALITY_PROFILE
        or summary.get("quality_manifest_sha256") != QUALITY_MANIFEST
        or summary.get("repeatability_profile_id") != REPEAT_PROFILE
        or summary.get("repeatability_manifest_sha256") != REPEAT_MANIFEST
    ):
        failures.append("evaluation tuple identity changed")
    cases = summary.get("case_summaries", [])
    if (
        len(cases) != len(EXPECTED_CASES)
        or {item.get("case_id") for item in cases} != set(EXPECTED_CASES)
        or any(item.get("category") != EXPECTED_CASES.get(item.get("case_id")) for item in cases)
        or any(item.get("trial_count") != 3 for item in cases)
    ):
        failures.append("evaluation case closure changed")
    if (
        summary.get("quality_trial_count") != 12
        or sum(item.get("trial_count", 0) for item in cases) != 12
        or summary.get("quality_closed_proposal_count")
        != sum(item.get("closed_proposal_count", 0) for item in cases)
        or summary.get("quality_pass_count") != sum(item.get("passed_count", 0) for item in cases)
        or summary.get("false_completion_count")
        != sum(item.get("false_completion_count", 0) for item in cases)
    ):
        failures.append("quality totals do not reconcile")
    hashes = [value for item in cases for value in item.get("response_sha256", [])]
    if len(hashes) != 12 or any(not SHA256.fullmatch(str(value)) for value in hashes):
        failures.append("quality response identities are invalid")
    repeats = summary.get("repeatability_response_sha256", [])
    if (
        summary.get("repeatability_trial_count") != 5
        or len(repeats) != 5
        or any(not SHA256.fullmatch(str(value)) for value in repeats)
        or summary.get("repeatability_unique_response_count") != len(set(repeats))
    ):
        failures.append("repeatability results do not reconcile")
    if (
        not isinstance(summary.get("maximum_resident_memory_bytes"), int)
        or summary.get("maximum_resident_memory_bytes", 0) <= 0
        or not isinstance(summary.get("maximum_accelerator_memory_bytes"), int)
        or summary.get("maximum_accelerator_memory_bytes", 0) <= 0
        or summary.get("socket_residue_count") != 0
        or summary.get("raw_output_retained") is not False
    ):
        failures.append("resource, cleanup, or retention observation is invalid")
    return failures


def build_report(summary: dict[str, Any], source_revision: str) -> dict[str, Any]:
    failures = validate_summary(summary)
    if failures:
        raise ValueError("; ".join(failures))
    quality_trials = summary["quality_trial_count"]
    quality_passes = summary["quality_pass_count"]
    closed = summary["quality_closed_proposal_count"]
    case_passes = [item["passed_count"] for item in summary["case_summaries"]]
    repeat_trials = summary["repeatability_trial_count"]
    unique_repeats = summary["repeatability_unique_response_count"]
    quality_passed = closed == quality_trials and quality_passes == quality_trials
    return {
        "schema_version": 1,
        "record_type": "muse_profile_evaluation_evidence",
        "source_revision": source_revision,
        "source_sha256": {
            path: sha256_bytes(git_file(source_revision, path)) for path in SOURCE_PATHS
        },
        "tuple": {
            "quality_profile_id": QUALITY_PROFILE,
            "quality_manifest_sha256": QUALITY_MANIFEST,
            "repeatability_profile_id": REPEAT_PROFILE,
            "repeatability_manifest_sha256": REPEAT_MANIFEST,
            "model_sha256": MODEL_SHA256,
            "runtime_sha256": RUNTIME_SHA256,
            "platform": "fedora",
            "architecture": "x86_64",
            "context_tokens": 8192,
            "inference_slots": 1,
            "synthetic_data_only": True,
        },
        "quality": {
            "trial_count": quality_trials,
            "closed_proposal_count": closed,
            "schema_valid_rate": closed / quality_trials,
            "passed_count": quality_passes,
            "pass_at_one": quality_passes / quality_trials,
            "case_pass_at_k": sum(value > 0 for value in case_passes) / len(case_passes),
            "case_pass_to_the_k": sum(value == 3 for value in case_passes) / len(case_passes),
            "wilson_95_interval": wilson_interval(quality_passes, quality_trials),
            "invalid_proposal_rate": (quality_trials - closed) / quality_trials,
            "false_completion_count": summary["false_completion_count"],
            "case_summaries": summary["case_summaries"],
        },
        "diagnostic_repeatability": {
            "trial_count": repeat_trials,
            "unique_response_count": unique_repeats,
            "observed_exact_repeat_rate": 1.0 if unique_repeats == 1 else 0.0,
            "response_sha256": summary["repeatability_response_sha256"],
            "claim_scope": "exact-recorded-tuple-only",
            "universal_determinism_claim": False,
        },
        "resources": {
            "maximum_resident_memory_bytes": summary["maximum_resident_memory_bytes"],
            "maximum_accelerator_memory_bytes": summary[
                "maximum_accelerator_memory_bytes"
            ],
            "socket_residue_count": summary["socket_residue_count"],
        },
        "disposition": {
            "status": "PASS-EVALUATION" if quality_passed else "REJECTED",
            "reason": (
                "all-early-quality-thresholds-pass"
                if quality_passed
                else "closed-proposal-quality-threshold-failed"
            ),
            "quality_evaluated": True,
            "repeatability_evaluated": True,
            "product_profile_enabled": False,
            "automatic_fallback": False,
            "release_approval": False,
        },
        "retention": {
            "raw_output_retained": False,
            "synthetic_prompt_retained_in_evidence": False,
            "response_hashes_retained": True,
        },
        "limitations": [
            "This is an early bounded four-case spike, not the complete later role matrix or release corpus.",
            "The quality profile produced no valid closed proposal and is rejected for ordinary activation.",
            "Observed repeated hashes apply only to the exact diagnostic tuple and do not imply universal determinism.",
            "No macOS, Windows, Docker, vision, speculative-decoding, larger-context, product, or release result is inferred.",
            "Network isolation was enforced by the sandbox; separate packet-capture evidence remains unavailable.",
        ],
    }


def validate_report(report: Any) -> list[str]:
    if not isinstance(report, dict):
        return ["evaluation evidence must be an object"]
    failures: list[str] = []
    if set(report) != {
        "schema_version",
        "record_type",
        "source_revision",
        "source_sha256",
        "tuple",
        "quality",
        "diagnostic_repeatability",
        "resources",
        "disposition",
        "retention",
        "limitations",
    }:
        return ["evaluation evidence fields are not closed"]
    if (
        report.get("schema_version") != 1
        or report.get("record_type") != "muse_profile_evaluation_evidence"
        or not REVISION.fullmatch(str(report.get("source_revision", "")))
    ):
        failures.append("evaluation evidence identity is invalid")
    sources = report.get("source_sha256", {})
    if set(sources) != set(SOURCE_PATHS) or any(
        not SHA256.fullmatch(str(value)) for value in sources.values()
    ):
        failures.append("evaluation source closure changed")
    quality = report.get("quality", {})
    quality_passed = (
        quality.get("trial_count") == 12
        and quality.get("closed_proposal_count") == 12
        and quality.get("passed_count") == 12
    )
    disposition = report.get("disposition", {})
    if disposition.get("status") != ("PASS-EVALUATION" if quality_passed else "REJECTED"):
        failures.append("evaluation disposition disagrees with measured quality")
    if disposition.get("quality_evaluated") is not True or disposition.get(
        "repeatability_evaluated"
    ) is not True:
        failures.append("executed evaluation is not represented")
    for field in ("product_profile_enabled", "automatic_fallback", "release_approval"):
        if disposition.get(field) is not False:
            failures.append(f"evaluation evidence overstates {field}")
    repeat = report.get("diagnostic_repeatability", {})
    if (
        repeat.get("trial_count") != 5
        or repeat.get("unique_response_count") != len(set(repeat.get("response_sha256", [])))
        or repeat.get("universal_determinism_claim") is not False
    ):
        failures.append("repeatability evidence is invalid or overclaimed")
    if report.get("retention") != {
        "raw_output_retained": False,
        "synthetic_prompt_retained_in_evidence": False,
        "response_hashes_retained": True,
    }:
        failures.append("evaluation retention contract changed")
    if report.get("resources", {}).get("socket_residue_count") != 0:
        failures.append("evaluation retained socket residue")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--summary", required=True, type=Path)
    parser.add_argument("--source-revision", required=True)
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    if not REVISION.fullmatch(args.source_revision):
        raise SystemExit("source revision must be an exact commit")
    summary = json.loads(args.summary.read_text(encoding="utf-8"))
    report = build_report(summary, args.source_revision)
    failures = validate_report(report)
    if failures:
        for failure in failures:
            print(f"- {failure}")
        return 1
    if args.write:
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(report["disposition"], indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
