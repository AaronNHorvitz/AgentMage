#!/usr/bin/env python3
"""Build and validate the Story 11.2 AC2 invalidation-acceptance record."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-11/story-11.2"
RAW_PATH: Final = EVIDENCE_DIR / "story-ac2-invalidation-acceptance-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "story-ac2-invalidation-acceptance-report.json"
RUNTIME_TEST_PATH: Final = ROOT / "kernel/engine/src/runtime_loop_tests.rs"
SOURCE_SCHEMA_PATH: Final = ROOT / "kernel/engine/migrations/operational-store/0012-source-artifact-materializations.sql"
COMMANDS: Final = (
    (
        "cargo",
        "test",
        "-p",
        "agentmage-kernel-engine",
        "story_11_2_changed_source_parser_policy_model_plan_tool_and_environment_block_resume",
        "--locked",
    ),
    ("python3", "scripts/source_lifecycle_transactions_evidence.py"),
    ("python3", "scripts/workflow_attempt_invariants_evidence.py"),
    (
        "cargo",
        "clippy",
        "-p",
        "agentmage-kernel-engine",
        "--all-targets",
        "--all-features",
        "--locked",
        "--",
        "-D",
        "warnings",
    ),
)
MARKERS: Final = (
    "story_11_2_changed_source_parser_policy_model_plan_tool_and_environment_block_resume ... ok",
    "Sub-task 11.2.1.3 atomic source lifecycle transactions validated",
    "Sub-task 11.2.2.2 workflow attempt invariants validated",
    "Finished `dev` profile",
)
IDENTITIES: Final = ("source", "parser", "policy", "model", "plan", "tool", "environment")
RUNTIME_MARKERS: Final = (
    "fn story_11_2_changed_source_parser_policy_model_plan_tool_and_environment_block_resume()",
    '"source",',
    '"parser",',
    '"policy",',
    '"model",',
    '"plan",',
    '"tool",',
    '"environment",',
    "candidate.request_sha256 = \"0\".repeat(64);",
    "Err(RuntimeLoopError::InvalidBoundaryResult)",
    '"changed {identity} identity cannot replay the completed tool"',
)
SOURCE_SCHEMA_MARKERS: Final = (
    "parser_identity TEXT NOT NULL",
    "parser_version TEXT NOT NULL",
    "parser_configuration_sha256 TEXT NOT NULL",
    "schema_sha256 TEXT NOT NULL",
    "policy_sha256 TEXT NOT NULL",
    "cache_key_sha256 TEXT NOT NULL UNIQUE",
    "FOREIGN KEY(source_artifact_id) REFERENCES source_manifests(source_artifact_id)",
)
RETAINED_PATHS: Final = (
    "kernel/engine/src/runtime_loop.rs",
    "kernel/engine/src/runtime_loop_tests.rs",
    "kernel/engine/src/source_lifecycle.rs",
    "kernel/engine/migrations/operational-store/0012-source-artifact-materializations.sql",
    "kernel/engine/migrations/operational-store/0014-source-lifecycle-transactions.sql",
    "kernel/engine/migrations/operational-store/0015-workflow-materializations.sql",
    "kernel/engine/migrations/operational-store/0016-workflow-attempt-invariants.sql",
    "artifacts/sprints/sprint-11/story-11.2/source-lifecycle-transactions-report.json",
    "artifacts/sprints/sprint-11/story-11.2/source-lifecycle-transactions-results.log",
    "artifacts/sprints/sprint-11/story-11.2/workflow-attempt-invariants-report.json",
    "artifacts/sprints/sprint-11/story-11.2/workflow-attempt-invariants-results.log",
)
TRUTH: Final = {
    "current_durable_runtime_and_store_scope_complete": True,
    "identity_change_count": 7,
    "source_change_blocks_resume": True,
    "parser_change_blocks_resume": True,
    "policy_change_blocks_resume": True,
    "model_change_blocks_resume": True,
    "plan_change_blocks_resume": True,
    "tool_change_blocks_resume": True,
    "environment_change_blocks_resume": True,
    "stale_source_derivatives_excluded": True,
    "stale_attempt_replay_refused": True,
    "post_change_tool_execution_count": 0,
    "synthetic_data_only": True,
    "all_later_active_parser_integrations_complete": False,
    "cross_platform_complete": False,
    "independent_review_complete": False,
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
        "record_type": "agentmage-story-acceptance-evidence",
        "story_id": "11.2",
        "criterion_id": "11.2.AC2",
        "generated_on": "2026-08-30",
        "status": "pass-local-current-durable-runtime-and-store-scope",
        "commands": [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "identity_dimensions": list(IDENTITIES),
        "identity_bindings": [
            {"identity": "source", "binding": "repository snapshot plus source lifecycle closure"},
            {"identity": "parser", "binding": "source-cache tuple plus authoritative evidence"},
            {"identity": "policy", "binding": "policy identity and revision digest"},
            {"identity": "model", "binding": "profile, manifest, runtime, and context tuple"},
            {"identity": "plan", "binding": "work-packet plan identity and revision"},
            {"identity": "tool", "binding": "catalog identity, digest, and ordered definitions"},
            {"identity": "environment", "binding": "workspace identity and sealed snapshot digest"},
        ],
        "retained_evidence": [artifact(path) for path in RETAINED_PATHS],
        "artifacts": [
            artifact("docs/verification/story-11-2-ac2-invalidation-acceptance.md"),
            artifact("scripts/story_11_2_ac2_evidence.py"),
            artifact("tests/test_story_11_2_ac2_evidence.py"),
            artifact(RAW_PATH.relative_to(ROOT).as_posix()),
        ],
        "acceptance_truth": dict(TRUTH),
        "limitations": [
            "later active parser and adapter integrations require their own rerun",
            "native cross-platform and installed-package execution remain separate gates",
            "independent review, Story, Sprint, packaging, and release completion are not claimed",
        ],
    }


def render(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n"


def validate_sources() -> list[str]:
    failures = [f"missing retained evidence: {path}" for path in RETAINED_PATHS if not (ROOT / path).is_file()]
    try:
        runtime = RUNTIME_TEST_PATH.read_text(encoding="utf-8")
        schema = SOURCE_SCHEMA_PATH.read_text(encoding="utf-8")
    except OSError as error:
        return failures + [f"cannot read AC2 source: {error}"]
    failures.extend(f"runtime test missing marker: {marker}" for marker in RUNTIME_MARKERS if marker not in runtime)
    failures.extend(f"source schema missing marker: {marker}" for marker in SOURCE_SCHEMA_MARKERS if marker not in schema)
    return failures


def validate_upstream_reports() -> list[str]:
    failures: list[str] = []
    expectations = (
        (
            "source-lifecycle-transactions-report.json",
            "product_truth",
            {
                "atomic_refresh_complete": True,
                "transitive_invalidation_complete": True,
                "synthetic_data_only": True,
            },
        ),
        (
            "source-lifecycle-transactions-report.json",
            "transaction_contract",
            {
                "refresh_invalidates_dependency_closure": True,
                "stale_derivatives_excluded_from_current_views": True,
                "stale_revisions_roll_back_without_reference_drift": True,
            },
        ),
        (
            "workflow-attempt-invariants-report.json",
            "invariant_contract",
            {
                "unique_attempt_identity": True,
                "unique_idempotency_key_digest": True,
                "exact_attempt_receipt_outcome_binding": True,
                "uncertainty_is_terminal_and_immutable": True,
            },
        ),
    )
    for name, section, required in expectations:
        try:
            value = json.loads((EVIDENCE_DIR / name).read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            failures.append(f"cannot read upstream report {name}: {error}")
            continue
        actual = value.get(section, {})
        for field, expected in required.items():
            if actual.get(field) != expected:
                failures.append(f"upstream report {name} has invalid {section}.{field}")
    return failures


def validate_raw(value: str) -> list[str]:
    failures = [f"raw results missing marker: {marker}" for marker in MARKERS if marker not in value]
    for prohibited in ("validation failed", "Traceback", "FAILED", "error:"):
        if prohibited in value:
            failures.append(f"raw results contain prohibited marker: {prohibited}")
    return failures


def validate_report(value: Any) -> list[str]:
    if value != expected_report():
        return ["Story 11.2 AC2 report is stale, incomplete, reordered, or widened"]
    return []


def capture() -> tuple[str, int]:
    chunks: list[str] = []
    for command in COMMANDS:
        result = subprocess.run(command, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, check=False)
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
        failures = validate_sources() + validate_upstream_reports() + validate_raw(raw)
        if failures:
            for failure in failures:
                print(f"Story 11.2 AC2 evidence build failed: {failure}", file=sys.stderr)
            return 1
        EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
        RAW_PATH.write_text(raw, encoding="utf-8")
        REPORT_PATH.write_text(render(expected_report()), encoding="utf-8")
    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(f"Story 11.2 AC2 evidence validation failed: {error}", file=sys.stderr)
        return 1
    failures = validate_sources() + validate_upstream_reports() + validate_raw(raw) + validate_report(report)
    if failures:
        for failure in failures:
            print(f"Story 11.2 AC2 evidence validation failed: {failure}", file=sys.stderr)
        return 1
    print("Story acceptance criterion 11.2.AC2 invalidation and stale-result refusal validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
