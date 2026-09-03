#!/usr/bin/env python3
"""Build the truthful local Story 13.4 context-profile evidence slice."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

try:
    from scripts.story_13_4_profile_campaign import OUTPUT_PATH as CAMPAIGN_PATH
    from scripts.story_13_4_profile_campaign import check as check_campaign
    from scripts.story_13_4_profile_campaign import write as write_campaign
except ModuleNotFoundError:  # Direct execution places scripts/ rather than the repository on sys.path.
    from story_13_4_profile_campaign import OUTPUT_PATH as CAMPAIGN_PATH
    from story_13_4_profile_campaign import check as check_campaign
    from story_13_4_profile_campaign import write as write_campaign


ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-13/story-13.4"
RAW_PATH: Final = EVIDENCE_DIR / "context-profile-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "context-profile-report.json"
UPSTREAM_PATH: Final = ROOT / "artifacts/sprints/sprint-13/local-evidence-report.json"
COMMANDS: Final = (
    ("cargo", "test", "-p", "agentmage-kernel-engine", "story_13_4", "--locked"),
    ("node", "--test", "tests/test_model_orchestration_profile_schema.mjs"),
    ("cargo", "clippy", "-p", "agentmage-kernel-engine", "--all-targets", "--all-features", "--locked", "--", "-D", "warnings"),
)
MARKERS: Final = (
    "story_13_4_exact_window_reconciles_without_overcommit ... ok",
    "story_13_4_degradation_is_deterministic_visible_and_source_first ... ok",
    "story_13_4_missing_counter_or_authoritative_minimum_fails_closed ... ok",
    "story_13_4_orchestration_changes_shape_but_not_safety_controls ... ok",
    "story_13_4_quality_evidence_and_model_admission_are_both_required ... ok",
    "story_13_4_every_profile_and_allocation_mutation_changes_identity_or_refuses ... ok",
    "compiled orchestration profile schema accepts the closed record",
    "schema rejects authority widening and stale qualification shapes",
    "Finished `dev` profile",
)
SOURCE_PATHS: Final = (
    "kernel/engine/src/model_orchestration_profile.rs",
    "schemas/model/orchestration-profile.schema.json",
    "docs/architecture/model-context-orchestration-profiles.md",
    "scripts/story_13_4_context_profile_evidence.py",
    "scripts/story_13_4_profile_campaign.py",
    "tests/test_story_13_4_context_profile_evidence.py",
    "tests/test_model_orchestration_profile_schema.mjs",
    "artifacts/sprints/sprint-13/local-evidence-report.json",
    "fixtures/artifact-admission/v1/context-accounting-manifests.json",
    "fixtures/artifact-evaluation/v1/workflow-plan-fixtures.json",
    "artifacts/sprints/sprint-13/story-13.3/muse-profile-evaluation.json",
    "model-profiles/candidates/gemma-4-e4b/feasibility-disposition.json",
    "model-profiles/candidates/gemma-4-12b-unified/feasibility-disposition.json",
)
TRUTH: Final = {
    "checked_context_allocation_contract_complete": True,
    "exact_tokenizer_and_counter_required": True,
    "deterministic_visible_degradation_complete": True,
    "orchestration_shape_contract_complete": True,
    "model_invariant_safety_controls_complete": True,
    "qualification_gate_complete": True,
    "automatic_fallback_enabled": False,
    "currently_admitted_profile_count": 0,
    "fake_profile_contract_campaign_complete": True,
    "muse_profile_disposition": "REJECTED",
    "gemma_profile_disposition": "REJECTED",
    "live_admitted_profile_corpus_complete": False,
    "exact_tuple_campaign_ledger_complete": True,
    "cross_platform_profile_campaign_complete": False,
    "story_completion_claim": False,
    "sprint_completion_claim": False,
    "release_claim": "none",
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def artifact(path: str) -> dict[str, Any]:
    absolute = ROOT / path
    return {"path": path, "byte_length": absolute.stat().st_size, "sha256": sha256(absolute)}


def expected_report() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "agentmage-story-13-4-context-profile-evidence",
        "story_id": "13.4",
        "generated_on": "2026-08-31",
        "status": "PASS_LOCAL_CONTRACT_BLOCKED_PROFILE_CAMPAIGNS",
        "commands": [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "allocation_order": [
            "fixed-system-tool-user-recovery-output-safety",
            "authoritative-source-complete-or-visible-truncation",
            "retrieval-complete-visible-summary-or-visible-omission",
            "record-unallocated-remainder",
        ],
        "model_invariant_controls": ["policy", "grant", "approval", "side-effect", "retry", "verifier", "budget", "completion"],
        "artifacts": [artifact(path) for path in SOURCE_PATHS]
        + [artifact(CAMPAIGN_PATH.relative_to(ROOT).as_posix()), artifact(RAW_PATH.relative_to(ROOT).as_posix())],
        "product_truth": dict(TRUTH),
        "remaining_work": [
            "run the retained corpus against the first independently admitted live profile",
            "retain matched native cross-platform profile evidence without borrowing rejected Muse or Gemma results",
            "complete applicable RV-13, RV-14, RV-16, and RV-41 installed-profile slices",
        ],
    }


def render(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, sort_keys=True) + "\n"


def validate_upstream() -> list[str]:
    try:
        value = json.loads(UPSTREAM_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read upstream model evidence: {error}"]
    truth = value.get("evidence_state", {})
    failures = []
    if truth.get("muse_disposition") != "REJECTED":
        failures.append("upstream Muse disposition is not the retained rejection")
    if truth.get("gemma_e4b_disposition") != "REJECTED" or truth.get("gemma_12b_disposition") != "REJECTED":
        failures.append("upstream Gemma dispositions are not the retained rejections")
    return failures


def validate_sources() -> list[str]:
    return [f"missing retained source: {path}" for path in SOURCE_PATHS if not (ROOT / path).is_file()]


def validate_raw(value: str) -> list[str]:
    failures = [f"raw results missing marker: {marker}" for marker in MARKERS if marker not in value]
    for prohibited in ("test result: FAILED", "not ok", "error: could not compile", "warning:"):
        if prohibited in value:
            failures.append(f"raw results contain prohibited marker: {prohibited}")
    return failures


def validate_report(value: Any) -> list[str]:
    return [] if value == expected_report() else ["Story 13.4 context-profile report is stale or widened"]


def capture() -> tuple[str, int]:
    chunks = []
    for command in COMMANDS:
        result = subprocess.run(command, cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=False)
        chunks.append(f"$ {' '.join(command)}\n{result.stdout}")
        if result.returncode != 0:
            return "\n".join(chunks), result.returncode
    return "\n".join(chunks), 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    failures = validate_sources() + validate_upstream()
    if args.write:
        try:
            write_campaign()
        except ValueError as error:
            failures.append(str(error))
        raw, returncode = capture()
        EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
        RAW_PATH.write_text(raw, encoding="utf-8")
        failures += validate_raw(raw)
        if returncode == 0 and not failures:
            REPORT_PATH.write_text(render(expected_report()), encoding="utf-8")
    else:
        try:
            raw = RAW_PATH.read_text(encoding="utf-8")
            report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            failures.append(f"cannot read retained evidence: {error}")
        else:
            failures += check_campaign() + validate_raw(raw) + validate_report(report)
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    print("Story 13.4 local context and orchestration profile contract validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
