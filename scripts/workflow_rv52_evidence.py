#!/usr/bin/env python3
"""Run and retain the Story 5.3-applicable slice of reviewer protocol RV-52."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-5/story-5.3"
RAW_PATH: Final = EVIDENCE_DIR / "rv52-applicable-results.log"
LINEAGE_PATH: Final = EVIDENCE_DIR / "rv52-workflow-lineage.json"
REPORT_PATH: Final = EVIDENCE_DIR / "rv52-applicability.json"
COMMANDS: Final = (
    ("cargo", "test", "-p", "agentmage-kernel-engine", "--test", "workflow_definition", "--locked"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "--test", "workflow_identity", "--locked"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "--test", "retry_admission", "--locked"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "--test", "workflow_verifier", "--locked"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "workflow_progress::tests", "--locked"),
    (
        "cargo",
        "clippy",
        "-p",
        "agentmage-kernel-engine",
        "-p",
        "agentmage-kernel-contracts",
        "--all-targets",
        "--all-features",
        "--locked",
        "--",
        "-D",
        "warnings",
    ),
)
LINEAGE_MARKERS: Final = {
    "graph": "admits_an_exact_closed_graph_and_exposes_no_execution_authority ... ok",
    "event": "fingerprint_is_deterministic_complete_and_boundary_preserving ... ok",
    "attempt": "issues_the_complete_identity_chain_without_execution_authority ... ok",
    "grant": "fresh_attempt_requires_current_preflight_remaining_budget_and_single_use_grant ... ok",
    "effect": "effect_classes_require_conservative_approval_and_idempotency_policy ... ok",
    "receipt": "approvals_receipts_and_verifications_are_fresh_single_issue_identities ... ok",
    "verification": "exact_current_deterministic_evidence_is_admitted_without_authority ... ok",
    "terminal_state": "changed_and_unchanged_verified_evidence_resolve_to_distinct_success_states ... ok",
    "no_replay": "complete_prior_use_ledger_denies_every_replayed_identity_and_any_receipt ... ok",
}
TRUTH: Final = {
    "synthetic_data_only": True,
    "model_inference_executed": False,
    "runtime_effect_executed": False,
    "native_platform_campaign": False,
    "protocol_complete": False,
    "later_story_acceptance_claim": False,
    "story_completion_claim": False,
    "sprint_completion_claim": False,
    "release_claim": "none",
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def artifact(path: str) -> dict[str, Any]:
    absolute = ROOT / path
    return {"path": path, "byte_length": absolute.stat().st_size, "sha256": sha256(absolute)}


def expected_lineage() -> dict[str, Any]:
    nodes = [
        {
            "kind": kind,
            "ordinal": ordinal,
            "synthetic_identity": f"rv52-synthetic-{kind.replace('_', '-')}-01",
            "evidence_marker": marker,
            "status": "PASS_LOCAL_CONTRACT",
        }
        for ordinal, (kind, marker) in enumerate(LINEAGE_MARKERS.items(), start=1)
    ]
    return {
        "schema_version": 1,
        "record_type": "agentmage-rv52-workflow-lineage",
        "protocol_id": "RV-52",
        "story_id": "5.3",
        "task_id": "5.3.3.2",
        "fixture_class": "synthetic-authority-free-contract",
        "nodes": nodes,
        "edges": [
            {
                "from": nodes[index]["synthetic_identity"],
                "to": nodes[index + 1]["synthetic_identity"],
                "relation": "evidence-precedes",
            }
            for index in range(len(nodes) - 1)
        ],
        "identity_contract": {
            "all_identities_unique": True,
            "attempt_predecessor_chain_is_ordered": True,
            "grant_is_single_use": True,
            "receipt_is_single_issue": True,
            "verification_is_current_and_receipt_bound": True,
            "terminal_state_requires_verification": True,
            "replayed_identity_is_admitted": False,
        },
        "execution_truth": {
            "runtime_effect_executed": False,
            "lineage_is_runtime_receipt": False,
            "lineage_is_native_evidence": False,
        },
        "sources": [
            artifact("kernel/engine/src/workflow_definition.rs"),
            artifact("kernel/engine/src/workflow_progress.rs"),
            artifact("kernel/engine/src/workflow_identity.rs"),
            artifact("kernel/engine/src/retry_admission.rs"),
            artifact("kernel/engine/src/workflow_verifier.rs"),
            artifact("kernel/engine/src/workflow_terminal.rs"),
        ],
    }


def expected_report() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "agentmage-rv52-applicability",
        "decision_id": "ADR-0043",
        "protocol_id": "RV-52",
        "story_id": "5.3",
        "task_id": "5.3.3.2",
        "generated_on": "2026-08-30",
        "status": "PARTIAL_LOCAL_CONTRACT_EVIDENCE",
        "protocol_complete": False,
        "commands": [" ".join(command) for command in COMMANDS],
        "lineage_kinds": list(LINEAGE_MARKERS),
        "applicable_results": [
            {
                "scenario": "graph-event-and-terminal-lineage",
                "status": "PASS_LOCAL_CONTRACT",
                "evidence": "ordered graph, state fingerprint, current verification, and distinct terminal outcomes",
            },
            {
                "scenario": "attempt-grant-effect-receipt-lineage",
                "status": "PASS_LOCAL_CONTRACT",
                "evidence": "fresh attempt identities, current single-use grant admission, conservative effect class, and single-issue receipt identity",
            },
            {
                "scenario": "drop-duplicate-reorder-corrupt-and-replay",
                "status": "PASS_LOCAL_CONTRACT",
                "evidence": "missing, duplicated, reordered, corrupt, stale, and replayed contract inputs fail closed before authority or success",
            },
            {
                "scenario": "uncertain-effect-reconciliation",
                "status": "PASS_LOCAL_CONTRACT",
                "evidence": "uncertain effects remain sticky and cannot open an automatic successor or verified success",
            },
        ],
        "acceptance_tests": [
            {"id": "AT-TIO-001", "status": "NOT_EXECUTED_OWNER_OPEN", "owner": "16.4"},
            {"id": "AT-TIO-002", "status": "NOT_EXECUTED_OWNER_OPEN", "owner": "16.4"},
            {"id": "AT-RESUME-002", "status": "NOT_EXECUTED_OWNER_OPEN", "owner": "11.3"},
            {"id": "AT-OBS-002", "status": "NOT_EXECUTED_OWNER_OPEN", "owner": "21.4"},
        ],
        "remaining_protocol_scenarios": [
            {
                "scenario": "client-loss-host-restart-and-current-reconstruction",
                "status": "BLOCKED_LATER_STORY",
                "owner": "11.3",
            },
            {
                "scenario": "large-artifact-backed-output-and-resource-pressure",
                "status": "BLOCKED_LATER_STORY",
                "owner": "16.4",
            },
            {
                "scenario": "exactly-one-terminal-observation-per-live-call",
                "status": "BLOCKED_LATER_STORY",
                "owner": "16.4",
            },
            {
                "scenario": "retention-inspector-and-observability-non-authority",
                "status": "BLOCKED_LATER_STORY",
                "owner": "21.4",
            },
            {
                "scenario": "complete-installed-source-to-terminal-workflow",
                "status": "BLOCKED_LATER_STORY",
                "owner": "22.5",
            },
        ],
        "artifacts": [
            artifact("SECURITY-REVIEW.md"),
            artifact("docs/verification/story-5-3-rv52-evidence.md"),
            artifact("scripts/workflow_rv52_evidence.py"),
            artifact("tests/test_workflow_rv52_evidence.py"),
            artifact(LINEAGE_PATH.relative_to(ROOT).as_posix()),
            artifact(RAW_PATH.relative_to(ROOT).as_posix()),
        ],
        "product_truth": dict(TRUTH),
    }


def render(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n"


def validate_raw(value: str) -> list[str]:
    failures = [
        f"RV-52 raw results missing {kind} marker: {marker}"
        for kind, marker in LINEAGE_MARKERS.items()
        if marker not in value
    ]
    for prohibited in ("test result: FAILED", "error: could not compile", "warning:"):
        if prohibited in value:
            failures.append(f"RV-52 raw results contain prohibited marker: {prohibited}")
    return failures


def validate_lineage(value: Any) -> list[str]:
    if value != expected_lineage():
        return ["RV-52 lineage is stale, incomplete, reordered, or widened"]
    identities = [node["synthetic_identity"] for node in value["nodes"]]
    if len(identities) != len(set(identities)) or len(identities) != len(LINEAGE_MARKERS):
        return ["RV-52 lineage identities are incomplete or reused"]
    return []


def validate_report(value: Any) -> list[str]:
    if value != expected_report():
        return ["RV-52 applicability record is stale, incomplete, reordered, or widened"]
    if value.get("protocol_complete") is not False or value.get("status") == "PASS":
        return ["RV-52 was falsely represented as complete"]
    if value.get("product_truth") != TRUTH:
        return ["RV-52 product truth was widened"]
    return []


def capture() -> tuple[str, int]:
    chunks: list[str] = []
    for command in COMMANDS:
        result = subprocess.run(
            command,
            cwd=ROOT,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            check=False,
        )
        chunks.append(f"$ {' '.join(command)}\n{result.stdout.rstrip(chr(10))}\n")
        if result.returncode != 0:
            return "".join(chunks), result.returncode
    return "".join(chunks), 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    if args.write:
        raw, returncode = capture()
        if returncode != 0:
            sys.stderr.write(raw)
            return 1
        failures = validate_raw(raw)
        if failures:
            for failure in failures:
                print(f"RV-52 evidence build failed: {failure}", file=sys.stderr)
            return 1
        EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
        RAW_PATH.write_text(raw, encoding="utf-8")
        LINEAGE_PATH.write_text(render(expected_lineage()), encoding="utf-8")
        REPORT_PATH.write_text(render(expected_report()), encoding="utf-8")

    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        lineage = json.loads(LINEAGE_PATH.read_text(encoding="utf-8"))
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(f"RV-52 applicability validation failed: {error}", file=sys.stderr)
        return 1
    failures = validate_raw(raw) + validate_lineage(lineage) + validate_report(report)
    if failures:
        for failure in failures:
            print(f"RV-52 applicability validation failed: {failure}", file=sys.stderr)
        return 1
    print("Sub-task 5.3.3.2 applicable RV-52 lineage validated with later-story blockers open")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
