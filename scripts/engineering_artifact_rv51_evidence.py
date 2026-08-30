#!/usr/bin/env python3
"""Execute and retain the Story 2.4-applicable portions of reviewer protocol RV-51."""

from __future__ import annotations

import argparse
import hashlib
import json
import shlex
import subprocess
import sys
from collections import Counter
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.fixture_security_scan import check_report as check_fixture_scan  # noqa: E402
from scripts.fixture_security_scan import write_report as write_fixture_scan  # noqa: E402


FIXTURE_DIR: Final = ROOT / "fixtures" / "artifact-admission" / "v1"
EVIDENCE_DIR: Final = ROOT / "artifacts" / "sprints" / "sprint-2" / "story-2.4"
RAW_PATH: Final = EVIDENCE_DIR / "rv51-applicable-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "rv51-applicability.json"
SCAN_PATH: Final = ROOT / "artifacts" / "sprints" / "sprint-2" / "story-2.1" / "fixture-security-scan-report.json"
PATHS: Final = {
    "admission": FIXTURE_DIR / "manifest.json",
    "adversarial": FIXTURE_DIR / "adversarial-manifest.json",
    "lineage": FIXTURE_DIR / "lineage-manifest.json",
    "accounting": FIXTURE_DIR / "context-accounting-manifests.json",
    "delivery": FIXTURE_DIR / "context-delivery-receipts.json",
    "resource": FIXTURE_DIR / "resource-gate-observations.json",
}
COMMANDS: Final = (
    "npm run engineering-runtime:artifact-admission-fixtures:check",
    "npm run engineering-runtime:artifact-adversarial-fixtures:check",
    "npm run engineering-runtime:artifact-lineage-fixtures:check",
    "npm run engineering-runtime:context-accounting-fixtures:check",
    "npm run engineering-runtime:context-delivery-fixtures:check",
    "npm run engineering-runtime:artifact-resource-gate:check",
    "python3 scripts/fixture_security_scan.py",
)
MARKERS: Final = (
    "Validated 16 identity-bound artifact admission fixtures",
    "Validated 10 inert hostile artifact fixtures",
    "Validated lineage for 26 sources and 21 ranges",
    "Validated 8 complete context-accounting manifests",
    "Validated 8 reconstructible context-delivery receipts",
    "Validated 10 resource ceilings and 4 cleanup boundaries",
    "Sprint 2 fixtures and generated artifacts passed the security scan",
)


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def artifact(path: Path) -> dict[str, str]:
    return {"path": str(path.relative_to(ROOT)), "sha256": sha(path)}


def source_hashes(lineage: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        {
            "fixture_id": case["fixture_id"],
            "source_artifact_id": case["source_artifact_id"],
            "capture_state": case["capture_state"],
            "byte_length": None if case["source_byte_identity"] is None else case["source_byte_identity"]["byte_length"],
            "sha256": None if case["source_byte_identity"] is None else case["source_byte_identity"]["sha256"],
        }
        for case in lineage["cases"]
    ]


def derivative_hashes(lineage: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        {
            "fixture_id": case["fixture_id"],
            "range_id": item["range_id"],
            "source_sha256": item["source_sha256"],
            "derivative_sha256": item["derivative_sha256"],
            "source_range": item["source_range"],
            "derivative_range": item["derivative_range"],
            "parser_id": item["parser_id"],
        }
        for case in lineage["cases"]
        for item in case["ranges"]
    ]


def parser_identities(lineage: dict[str, Any]) -> list[dict[str, Any]]:
    captured = [case for case in lineage["cases"] if case["parser_link"]["state"] == "fixture_identity_projection"]
    absent = [case for case in lineage["cases"] if case["parser_link"]["state"] == "not_created"]
    reasons = Counter(case["parser_link"]["reason_code"] for case in absent)
    return [
        {
            "parser_id": "fixture.byte_identity_projector",
            "parser_version": "1.0.0",
            "state": "fixture_identity_projection",
            "source_count": len(captured),
            "product_parser_executed": False,
        },
        {
            "parser_id": None,
            "parser_version": None,
            "state": "not_created",
            "source_count": len(absent),
            "reason_counts": dict(sorted(reasons.items())),
            "product_parser_executed": False,
        },
    ]


def context_receipts(delivery: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        {
            "scenario_id": scenario["scenario_id"],
            "context_manifest_id": scenario["context_manifest_id"],
            "context_manifest_sha256": scenario["context_manifest_sha256"],
            "receipt_id": scenario["receipt"]["receipt_id"],
            "receipt_sha256": scenario["receipt"]["receipt_sha256"],
            "model_visible_set_sha256": scenario["model_visible_set_sha256"],
            "outcome": scenario["receipt"]["outcome"],
            "completion_allowed": scenario["completion_allowed"],
            "required_unseen_artifact_ids": scenario["receipt"]["required_unseen_artifact_ids"],
        }
        for scenario in delivery["scenarios"]
    ]


def zero_silent_drop(lineage: dict[str, Any], accounting: dict[str, Any]) -> dict[str, Any]:
    expected_ids = [case["source_artifact_id"] for case in lineage["cases"]]
    scenario_results = []
    for scenario in accounting["scenarios"]:
        items = scenario["context_manifest"]["items"]
        actual_ids = [item["artifact_id"] for item in items]
        scenario_results.append(
            {
                "scenario_id": scenario["scenario_id"],
                "supplied_source_count": len(expected_ids),
                "explicit_disposition_count": len(items),
                "missing_source_artifact_ids": sorted(set(expected_ids) - set(actual_ids)),
                "unexpected_source_artifact_ids": sorted(set(actual_ids) - set(expected_ids)),
                "duplicate_source_artifact_ids": sorted(
                    identity for identity, count in Counter(actual_ids).items() if count != 1
                ),
                "result": "PASS_LOCAL_FIXTURE" if actual_ids == expected_ids else "FAIL",
            }
        )
    return {
        "supplied_source_count": len(expected_ids),
        "lineage_source_count": lineage["source_count"],
        "lineage_missing_source_artifact_ids": [],
        "scenario_count": len(scenario_results),
        "scenario_results": scenario_results,
        "zero_silent_drop_result": "PASS_LOCAL_FIXTURE",
    }


def hostile_results(adversarial: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        {
            "fixture_id": case["fixture_id"],
            "category": case["category"],
            "terminal_state": case["expected_result"]["terminal_state"],
            "error_code": case["expected_result"]["error_code"],
            "active_content_executed": case["expected_result"]["active_content_executed"],
            "external_relationship_fetched": case["expected_result"]["external_relationship_fetched"],
            "residue_retained": case["expected_result"]["residue_retained"],
            "may_claim_complete": case["expected_result"]["may_claim_complete"],
        }
        for case in adversarial["cases"]
    ]


def expected_report() -> dict[str, Any]:
    values = {name: read_json(path) for name, path in PATHS.items()}
    scan = read_json(SCAN_PATH)
    source_records = source_hashes(values["lineage"])
    derivative_records = derivative_hashes(values["lineage"])
    receipts = context_receipts(values["delivery"])
    return {
        "schema_version": 1,
        "record_type": "engineering-artifact-rv51-applicability",
        "decision_id": "ADR-0043",
        "story_id": "2.4",
        "task_id": "2.4.3.2",
        "protocol_id": "RV-51",
        "generated_on": "2026-08-29",
        "status": "PARTIAL_LOCAL_FIXTURE_EVIDENCE",
        "protocol_complete": False,
        "commands": list(COMMANDS),
        "required_markers": list(MARKERS),
        "fixture_manifests": [
            {"kind": name, **artifact(path)} for name, path in PATHS.items()
        ],
        "parser_identities": parser_identities(values["lineage"]),
        "source_hash_count": len(source_records),
        "source_hashes": source_records,
        "derivative_hash_count": len(derivative_records),
        "derivative_hashes": derivative_records,
        "context_receipt_count": len(receipts),
        "context_receipts": receipts,
        "resource_observation_count": values["resource"]["resource_observation_count"],
        "resource_observations": values["resource"]["resource_observations"],
        "lifecycle_observations": values["resource"]["lifecycle_observations"],
        "zero_silent_drop": zero_silent_drop(values["lineage"], values["accounting"]),
        "hostile_fixture_results": hostile_results(values["adversarial"]),
        "fixture_security": {
            **artifact(SCAN_PATH),
            "status": scan["status"],
            "blocking_finding_count": scan["summary"]["blocking_finding_count"],
            "raw_sensitive_values_retained": scan["raw_sensitive_values_retained"],
            "network_calls_performed": scan["network_calls_performed"],
            "product_support_claim": scan["product_support_claim"],
        },
        "applicable_results": [
            {"scenario": "complete-source-accounting", "status": "PASS_LOCAL_FIXTURE"},
            {"scenario": "exact-source-and-derivative-lineage", "status": "PASS_LOCAL_FIXTURE"},
            {"scenario": "hostile-input-inert-dispositions", "status": "PASS_LOCAL_FIXTURE"},
            {"scenario": "context-pressure-retrieval-and-required-unseen", "status": "PASS_LOCAL_FIXTURE"},
            {"scenario": "bounded-resource-and-cleanup-campaign", "status": "PASS_LOCAL_SYNTHETIC_WORKER"},
            {"scenario": "fixture-secret-and-network-scan", "status": "PASS_LOCAL_FIXTURE_SCAN"},
        ],
        "acceptance_tests": [
            {"id": "AT-CTX-001", "status": "PARTIAL_LOCAL_FIXTURE", "owner": "2.4"},
            {"id": "AT-CTX-002", "status": "PARTIAL_LOCAL_FIXTURE", "owner": "2.4"},
            {"id": "AT-CTX-003", "status": "PARTIAL_LOCAL_FIXTURE", "owner": "2.4"},
            {"id": "AT-CTX-004", "status": "PARTIAL_LOCAL_FIXTURE", "owner": "2.4"},
        ],
        "remaining_protocol_scenarios": [
            {
                "scenario": "product-parser-execution-for-every-admitted-format",
                "status": "BLOCKED_LATER_STORY",
                "owners": ["16.4", "22.3", "60.2", "62.2"],
            },
            {
                "scenario": "parser-crash-resume-reparse-reorder-and-sentinel-fidelity",
                "status": "BLOCKED_LATER_STORY",
                "owners": ["22.3", "22.5"],
            },
            {
                "scenario": "installed-participant-and-client-delivery",
                "status": "BLOCKED_LATER_STORY",
                "owners": ["23.5", "50.4"],
            },
            {
                "scenario": "complete-at-ctx-001-through-at-ctx-004-product-execution",
                "status": "BLOCKED_LATER_STORY",
                "owners": ["22.5", "50.4", "126.2"],
            },
        ],
        "artifacts": [
            artifact(ROOT / "SECURITY-REVIEW.md"),
            artifact(ROOT / "scripts" / "engineering_artifact_rv51_evidence.py"),
            artifact(ROOT / "tests" / "test_engineering_artifact_rv51_evidence.py"),
            artifact(RAW_PATH),
        ],
        "limitations": [
            "Identity projection is a fixture-only byte-preserving transform; no product parser executed.",
            "Synthetic worker cleanup does not substitute for integrated parser crash and resume evidence.",
            "No installed client, native platform, qualified model, product acceptance, external review, release, or full RV-51 pass is claimed.",
        ],
    }


def render(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, ensure_ascii=True) + "\n"


def validate_raw(text: str) -> list[str]:
    failures = [f"RV-51 raw results missing marker: {marker}" for marker in MARKERS if marker not in text]
    for prohibited in ("FAILED (", "test result: FAILED", "not ok ", "Traceback (most recent call last)"):
        if prohibited in text:
            failures.append(f"RV-51 raw results contain failure marker: {prohibited}")
    return failures


def validate_report(value: Any) -> list[str]:
    if value != expected_report():
        return ["RV-51 applicability record is stale, incomplete, reordered, or widened"]
    failures: list[str] = []
    if value.get("protocol_complete") is not False or value.get("status") == "PASS":
        failures.append("RV-51 was falsely represented as complete")
    if any(item.get("status") != "PARTIAL_LOCAL_FIXTURE" for item in value.get("acceptance_tests", [])):
        failures.append("an AT-CTX test was falsely represented as complete")
    zero_drop = value.get("zero_silent_drop", {})
    if zero_drop.get("zero_silent_drop_result") != "PASS_LOCAL_FIXTURE" or any(
        item.get("result") != "PASS_LOCAL_FIXTURE" for item in zero_drop.get("scenario_results", [])
    ):
        failures.append("zero-silent-drop fixture reconciliation did not pass")
    if any(item.get("product_parser_executed") is not False for item in value.get("parser_identities", [])):
        failures.append("fixture parser identity was widened into a product parser claim")
    if not value.get("remaining_protocol_scenarios"):
        failures.append("remaining RV-51 protocol scenarios were hidden")
    return failures


def capture() -> tuple[str, int]:
    chunks: list[str] = []
    for command in COMMANDS:
        chunks.append(f"$ {command}\n")
        result = subprocess.run(
            shlex.split(command),
            cwd=ROOT,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            check=False,
        )
        chunks.append(result.stdout)
        if not result.stdout.endswith("\n"):
            chunks.append("\n")
        if result.returncode != 0:
            return "".join(chunks), result.returncode
    return "".join(chunks), 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    if args.write:
        # Establish the final evidence file closure before the recursive fixture scan so
        # adding the RV-51 envelope cannot make its own captured scan command stale.
        RAW_PATH.write_text("", encoding="utf-8")
        REPORT_PATH.write_text("{}\n", encoding="utf-8")
        write_fixture_scan()
        raw, returncode = capture()
        if returncode != 0:
            sys.stderr.write(raw)
            print("RV-51 applicable evidence build failed", file=sys.stderr)
            return 1
        RAW_PATH.write_text(raw, encoding="utf-8")
        failures = validate_raw(raw)
        if failures:
            for failure in failures:
                print(f"RV-51 applicable evidence build failed: {failure}", file=sys.stderr)
            return 1
        # Re-scan the now-populated raw result, then bind that current scan in the
        # final report. The scanner retains counts and findings, not report hashes,
        # so replacing the fixed report envelope does not create a hash cycle.
        write_fixture_scan()
        REPORT_PATH.write_text(render(expected_report()), encoding="utf-8")

    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = read_json(REPORT_PATH)
    except (OSError, json.JSONDecodeError) as error:
        print(f"RV-51 applicability validation failed: {error}", file=sys.stderr)
        return 1
    failures = check_fixture_scan() + validate_raw(raw) + validate_report(report)
    if failures:
        for failure in failures:
            print(f"RV-51 applicability validation failed: {failure}", file=sys.stderr)
        return 1
    print("Task 2.4.3.2 applicable RV-51 evidence validated with full protocol blockers open")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
