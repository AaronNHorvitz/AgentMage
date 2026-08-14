#!/usr/bin/env python3
"""Build and verify the attributable Sprint 15 candidate-role matrix."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
INVENTORY = ROOT / "model-profiles/catalogs/2026-08-14/candidate-inventory.json"
ROLE_MATRIX = ROOT / "model-profiles/catalogs/2026-08-14/candidate-role-suite-matrix.json"
CORPUS = ROOT / "model-profiles/evaluation/role-suite-corpus-v1.json"
SCHEMA = ROOT / "model-profiles/evaluation/benchmark-record-schema-v1.json"
MUSE_RESULT = ROOT / "artifacts/sprints/sprint-13/story-13.3/muse-profile-evaluation.json"
OUTPUT = ROOT / "artifacts/sprints/sprint-15/story-15.3/candidate-role-evaluation-matrix.json"

ALLOWED_DISPOSITIONS = {
    "PASS",
    "FAILED",
    "BLOCKED",
    "BLOCKED-HARDWARE",
    "REJECTED",
    "STALE",
    "INCOMPARABLE",
    "UNKNOWN",
    "NOT-APPLICABLE",
    "INELIGIBLE",
}


def load(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def canonical_sha256(value: Any) -> str:
    data = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True)
    return hashlib.sha256(data.encode("ascii")).hexdigest()


def file_sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def build() -> dict[str, Any]:
    inventory = load(INVENTORY)
    matrix = load(ROLE_MATRIX)
    corpus = load(CORPUS)
    muse = load(MUSE_RESULT)
    corpus_suites = {entry["suite_id"] for entry in corpus["suites"]}

    inventory_by_id = {entry["entry_id"]: entry for entry in inventory["entries"]}
    results = []
    for entry in matrix["entries"]:
        source = inventory_by_id.get(entry["entry_id"])
        if source is None:
            raise ValueError(f"matrix entry missing from inventory: {entry['entry_id']}")
        if source["repository"] != entry["repository"]:
            raise ValueError(f"repository mismatch: {entry['entry_id']}")
        missing_suites = sorted(set(entry["applicable_suites"]) - corpus_suites)
        if missing_suites:
            raise ValueError(f"unfrozen suites for {entry['entry_id']}: {missing_suites}")
        disposition = entry["disposition"]
        if disposition not in ALLOWED_DISPOSITIONS:
            raise ValueError(f"unsupported disposition: {disposition}")
        results.append(
            {
                "entry_id": entry["entry_id"],
                "repository": entry["repository"],
                "roles": entry["roles"],
                "applicable_suites": entry["applicable_suites"],
                "disposition": disposition,
                "hardware_status": entry["hardware_status"],
                "exact_profile_admitted": False,
                "selectable": False,
                "result_reason": (
                    "source-ineligible"
                    if disposition == "INELIGIBLE"
                    else "exact-artifact-profile-and-role-trials-not-admitted"
                ),
                "borrowed_result": False,
            }
        )

    exact_muse = {
        "profile_id": muse["tuple"]["quality_profile_id"],
        "source_family": "meta-models/Muse-Glimmer-30B-GGUF",
        "disposition": muse["disposition"]["status"],
        "selectable": muse["disposition"]["product_profile_enabled"],
        "quality_trial_count": muse["quality"]["trial_count"],
        "pass_at_one": muse["quality"]["pass_at_one"],
        "pass_at_k": muse["quality"]["case_pass_at_k"],
        "pass_to_the_k": muse["quality"]["case_pass_to_the_k"],
        "confidence_interval": muse["quality"]["wilson_95_interval"],
        "invalid_proposal_rate": muse["quality"]["invalid_proposal_rate"],
        "false_completion_rate": (
            muse["quality"]["false_completion_count"] / muse["quality"]["trial_count"]
        ),
        "diagnostic_repeatability_trial_count": muse["diagnostic_repeatability"]["trial_count"],
        "observed_exact_repeat_rate": muse["diagnostic_repeatability"][
            "observed_exact_repeat_rate"
        ],
        "universal_determinism_claim": False,
        "source_inventory_result_reused": False,
        "evidence_sha256": file_sha256(MUSE_RESULT),
    }
    if exact_muse["disposition"] != "REJECTED" or exact_muse["selectable"]:
        raise ValueError("Muse evidence no longer has the retained rejected disposition")

    counts: dict[str, int] = {}
    for result in results:
        counts[result["disposition"]] = counts.get(result["disposition"], 0) + 1
    frozen_inputs = {
        str(path.relative_to(ROOT)): file_sha256(path)
        for path in [INVENTORY, ROLE_MATRIX, CORPUS, SCHEMA, MUSE_RESULT]
    }
    report: dict[str, Any] = {
        "schema_version": 1,
        "record_type": "agentmage_candidate_role_evaluation_matrix",
        "source_revision": f"content-{canonical_sha256(frozen_inputs)}",
        "frozen_inputs": frozen_inputs,
        "candidate_count": len(results),
        "candidate_dispositions": counts,
        "enabled_model_count": 0,
        "automatic_fallback": False,
        "inference_slot_default": 1,
        "source_candidate_results": results,
        "exact_profile_results": [exact_muse],
        "comparability_policy": {
            "matching_tuple_required": True,
            "differing_tuples_are_merged": False,
            "family_results_are_borrowed": False,
            "quality_and_repeatability_are_separate": True,
            "automatic_selection": False,
        },
        "admission_thresholds": {
            "minimum_trials": 5,
            "schema_valid_rate": 1.0,
            "tool_valid_rate": 1.0,
            "unsupported_action_rate": 0.0,
            "false_completion_rate": 0.0,
            "role_specific_threshold_required": True,
            "negative_controls_required": True,
            "evidence_expiry_requires_retest": True,
            "tuple_change_requires_retest": True,
        },
        "claims": {
            "family_winner": False,
            "family_wide_support": False,
            "universal_determinism": False,
            "hardware_fit_beyond_recorded_tuple": False,
        },
        "blockers": [
            "415 source candidates lack independently admitted exact artifact profiles",
            "one source candidate is ineligible and remains unselectable",
            "the only deeply measured Muse text tuple is rejected",
            "native macOS comparable evidence is unavailable",
            "no exact profile currently satisfies every product admission gate",
        ],
    }
    report["matrix_sha256"] = canonical_sha256(report)
    return report


def validate(report: dict[str, Any]) -> None:
    rebuilt = build()
    if report != rebuilt:
        raise ValueError("candidate role evaluation matrix is stale or malformed")
    if report["candidate_count"] != 416:
        raise ValueError("candidate population is incomplete")
    if any(item["selectable"] or item["borrowed_result"] for item in report["source_candidate_results"]):
        raise ValueError("blocked source candidates cannot be selectable or borrow evidence")
    if report["enabled_model_count"] != 0 or report["automatic_fallback"]:
        raise ValueError("evaluation evidence changed product routing")
    if any(report["claims"].values()):
        raise ValueError("unsupported broad model claim is present")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    report = build()
    if args.check:
        validate(load(OUTPUT))
    else:
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        validate(load(OUTPUT))
    print(
        json.dumps(
            {
                "candidate_count": report["candidate_count"],
                "dispositions": report["candidate_dispositions"],
                "enabled_model_count": report["enabled_model_count"],
                "matrix_sha256": report["matrix_sha256"],
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
