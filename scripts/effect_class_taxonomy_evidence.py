#!/usr/bin/env python3
"""Build and validate retained evidence for Sub-task 5.2.1.1."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts" / "sprints" / "sprint-5" / "story-5.2"
RAW_PATH: Final = EVIDENCE_DIR / "effect-class-taxonomy-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "effect-class-taxonomy-report.json"

COMMANDS: Final = (
    (
        "cargo",
        "test",
        "-p",
        "agentmage-kernel-contracts",
        "engineering_records::step_execution_policy_tests::effect_taxonomy_is_closed_versioned_and_independent_of_authority_and_risk",
        "--locked",
    ),
    (
        "cargo",
        "test",
        "-p",
        "agentmage-kernel-contracts",
        "engineering_records::step_execution_policy_tests::every_effect_class_admits_only_its_own_retry_classes",
        "--locked",
    ),
    (
        "cargo",
        "test",
        "-p",
        "agentmage-kernel-contracts",
        "engineering_records::step_execution_policy_tests::workflow_failure_taxonomy_has_one_exact_conservative_default_per_class",
        "--locked",
    ),
)
MARKERS: Final = (
    "effect_taxonomy_is_closed_versioned_and_independent_of_authority_and_risk ... ok",
    "every_effect_class_admits_only_its_own_retry_classes ... ok",
    "workflow_failure_taxonomy_has_one_exact_conservative_default_per_class ... ok",
    "test result: ok. 1 passed; 0 failed",
)
EFFECT_CLASSES: Final = (
    {
        "class": "read_only",
        "automatic_retry": True,
        "approval_required": False,
        "retry_classes": ["never", "recoverable_read"],
    },
    {
        "class": "idempotent_write",
        "automatic_retry": True,
        "approval_required": False,
        "retry_classes": ["never", "conditional_after_reconciliation"],
    },
    {
        "class": "conditional",
        "automatic_retry": True,
        "approval_required": False,
        "retry_classes": ["never", "conditional_after_reconciliation"],
    },
    {
        "class": "non_idempotent",
        "automatic_retry": False,
        "approval_required": True,
        "retry_classes": ["never", "user_decision_required"],
    },
    {
        "class": "destructive",
        "automatic_retry": False,
        "approval_required": True,
        "retry_classes": ["never", "user_decision_required"],
    },
    {
        "class": "external",
        "automatic_retry": False,
        "approval_required": True,
        "retry_classes": ["never", "user_decision_required"],
    },
    {
        "class": "unknown",
        "automatic_retry": False,
        "approval_required": True,
        "retry_classes": ["never", "user_decision_required"],
    },
)
FAILURE_CLASSES: Final = (
    ("malformed_input", "reject_before_dispatch"),
    ("preflight", "reject_before_dispatch"),
    ("policy", "blocked_by_policy"),
    ("approval", "await_fresh_approval"),
    ("dependency", "await_dependency"),
    ("transient", "eligible_fresh_attempt"),
    ("conflict", "reconcile_then_decide"),
    ("timeout", "reconcile_then_decide"),
    ("cancellation", "terminal_cancelled"),
    ("crash", "reconcile_then_decide"),
    ("uncertain_effect", "reconcile_then_decide"),
    ("verification", "terminal_failure"),
    ("resource", "terminal_resource_exhausted"),
    ("internal", "terminal_failure"),
)
TRUTH: Final = {
    "synthetic_data_only": True,
    "product_runtime_executed": False,
    "tool_effect_executed": False,
    "authority_minted": False,
    "network_calls": 0,
    "native_platform_claim": "none",
    "product_support_claim": "none",
    "release_claim": "none",
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def artifact(path: str) -> dict[str, Any]:
    absolute = ROOT / path
    return {"path": path, "byte_length": absolute.stat().st_size, "sha256": sha256(absolute)}


def command_text(command: tuple[str, ...]) -> str:
    return " ".join(command)


def expected_report() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "agentmage-effect-class-taxonomy-evidence",
        "story_id": "5.2",
        "task_id": "5.2.1.1",
        "coverage_task_ids": ["5.2.1.1", "5.2.1.2"],
        "generated_on": "2026-08-30",
        "status": "pass-local-contract-evidence",
        "taxonomy_version": 1,
        "effect_classes": [dict(item) for item in EFFECT_CLASSES],
        "workflow_failure_taxonomy_version": 1,
        "failure_classes": [
            {"class": failure_class, "default_disposition": disposition}
            for failure_class, disposition in FAILURE_CLASSES
        ],
        "independent_dimensions": {
            "effect_class_count": 7,
            "authority_class_count": 8,
            "tool_risk_level_count": 4,
            "cartesian_cases": 224,
            "authority_or_risk_can_infer_effect": False,
        },
        "rejected_class_shapes": ["custom", "wildcard", "inherited", "model_created", "empty"],
        "commands": [command_text(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "artifacts": [
            artifact("kernel/contracts/src/engineering_records.rs"),
            artifact("scripts/effect_class_taxonomy_evidence.py"),
            artifact("tests/test_effect_class_taxonomy_evidence.py"),
            artifact(RAW_PATH.relative_to(ROOT).as_posix()),
        ],
        "product_truth": dict(TRUTH),
    }


def render(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n"


def validate_raw(text: str) -> list[str]:
    failures = [f"raw results missing marker: {marker}" for marker in MARKERS if marker not in text]
    for prohibited in ("FAILED (", "test result: FAILED", "not ok ", "Traceback (most recent call last)"):
        if prohibited in text:
            failures.append(f"raw results contain failure marker: {prohibited}")
    return failures


def validate_report(value: Any) -> list[str]:
    if value != expected_report():
        return ["effect-class taxonomy report is stale, incomplete, reordered, or widened"]
    if value.get("product_truth") != TRUTH:
        return ["effect-class taxonomy product truth was widened"]
    return []


def capture() -> tuple[str, int]:
    chunks: list[str] = []
    for command in COMMANDS:
        chunks.append(f"$ {command_text(command)}\n")
        result = subprocess.run(
            command,
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
    if args.write:
        raw, returncode = capture()
        if returncode != 0:
            sys.stderr.write(raw)
            print("effect-class taxonomy evidence build failed", file=sys.stderr)
            return 1
        failures = validate_raw(raw)
        if failures:
            for failure in failures:
                print(f"effect-class taxonomy evidence build failed: {failure}", file=sys.stderr)
            return 1
        EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
        RAW_PATH.write_text(raw, encoding="utf-8")
        REPORT_PATH.write_text(render(expected_report()), encoding="utf-8")

    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(f"effect-class taxonomy evidence validation failed: {error}", file=sys.stderr)
        return 1
    failures = validate_raw(raw) + validate_report(report)
    if failures:
        for failure in failures:
            print(f"effect-class taxonomy evidence validation failed: {failure}", file=sys.stderr)
        return 1
    print("Sub-tasks 5.2.1.1-5.2.1.2 effect and failure taxonomy evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
