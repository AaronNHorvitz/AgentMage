#!/usr/bin/env python3
"""Retain already-demonstrated frozen-scope breadth criteria without product promotion."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
REPORT_PATH: Final = ROOT / "artifacts/stabilization/frozen-scope-breadth-report.json"
INPUTS: Final = (
    "artifacts/sprints/sprint-10/story-10.1/state-root-policy.json",
    "artifacts/sprints/sprint-10/story-10.1/storage-detection-fixtures.json",
    "artifacts/sprints/sprint-10/story-10.1/strict-local-classification.json",
    "artifacts/sprints/sprint-11/story-11.1/crash-canary-results.json",
    "artifacts/sprints/sprint-11/story-11.1/derived-json-lines.json",
    "artifacts/sprints/sprint-11/story-11.1/security-evidence-map.json",
    "artifacts/sprints/sprint-11/story-11.2/storage-security-evidence-report.json",
    "artifacts/sprints/sprint-11/story-11.3/durable-resume-report.json",
)


def load(path: str) -> dict[str, Any]:
    return json.loads((ROOT / path).read_text(encoding="utf-8"))


def artifact(path: str) -> dict[str, Any]:
    data = (ROOT / path).read_bytes()
    return {"path": path, "byte_length": len(data), "sha256": hashlib.sha256(data).hexdigest()}


def validate_inputs() -> list[str]:
    failures: list[str] = []
    try:
        values = {path: load(path) for path in INPUTS}
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read breadth evidence input: {error}"]
    state = values[INPUTS[0]]["claims"]
    fixtures = values[INPUTS[1]]["claims"]
    classification = values[INPUTS[2]]["claims"]
    crash = values[INPUTS[3]]["claims"]
    derived = values[INPUTS[4]]["claims"]
    store_security = values[INPUTS[6]]["product_truth"]
    resume = values[INPUTS[7]]["product_truth"]
    if not (
        state.get("configuration_and_authority_pre_io_rejection_tested")
        and fixtures.get("provider_and_sentinel_directories_tested")
        and classification.get("remote_fuse_unknown_and_sync_storage_fixtures_rejected")
    ):
        failures.append("Sprint 10 strict-local state-root evidence is incomplete")
    if not (
        resume.get("journal_process_stop_matrix_complete")
        and resume.get("authority_crash_reconciliation_complete")
        and resume.get("uncertain_or_completed_effect_replayed") is False
    ):
        failures.append("Sprint 11 crash/orphan invariant evidence is incomplete")
    if not store_security.get("encrypted_page_scans_retained"):
        failures.append("Sprint 11 plaintext-fallback evidence is incomplete")
    if not (crash.get("encrypted_and_derived_canary_absent") and store_security.get("raw_canary_retained") is False):
        failures.append("Sprint 11 canary evidence is incomplete")
    if not (derived.get("deletion_or_tamper_changes_no_canonical_state") and derived.get("json_lines_startup_authority_rejected")):
        failures.append("Sprint 11 derived-index authority evidence is incomplete")
    return failures


def expected_report() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "agentmage-frozen-scope-breadth-evidence",
        "generated_on": "2026-09-03",
        "status": "PASS_LOCAL_INVARIANTS_WITH_PRODUCT_GATES_OPEN",
        "criteria": [
            {"criterion_id": "10.AC4", "status": "pass-linux-policy-and-fixture-scope"},
            {"criterion_id": "11.AC2", "status": "pass-local-transaction-and-resume-scope"},
            {"criterion_id": "11.AC3", "status": "pass-local-encrypted-store-scope"},
            {"criterion_id": "11.AC4", "status": "pass-current-retained-canary-surfaces"},
            {"criterion_id": "11.AC5", "status": "pass-one-way-derived-export-scope"},
        ],
        "artifacts": [artifact(path) for path in INPUTS],
        "product_truth": {
            "installed_product_complete": False,
            "native_cross_platform_complete": False,
            "story_10_complete": False,
            "sprint_10_complete": False,
            "story_11_1_complete": False,
            "sprint_11_complete": False,
            "release_claim": "none",
            "external_evidence_substituted": False,
        },
    }


def render(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, sort_keys=True) + "\n"


def validate_report(value: Any) -> list[str]:
    return [] if value == expected_report() else ["frozen-scope breadth report is stale or widened"]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    failures = validate_inputs()
    if args.write and not failures:
        REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
        REPORT_PATH.write_text(render(expected_report()), encoding="utf-8")
    elif not args.write:
        try:
            failures.extend(validate_report(json.loads(REPORT_PATH.read_text(encoding="utf-8"))))
        except (OSError, json.JSONDecodeError) as error:
            failures.append(f"cannot read frozen-scope breadth report: {error}")
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    print("Frozen-scope local breadth criteria validated: 5")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
