#!/usr/bin/env python3
"""Build and validate Story 5.2 AC1 deterministic-disposition evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-5/story-5.2"
RAW_PATH: Final = EVIDENCE_DIR / "story-ac1-deterministic-disposition-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "story-ac1-deterministic-disposition-report.json"
COMMANDS: Final = (
    ("cargo", "test", "-p", "agentmage-kernel-contracts", "story_5_2_every_registered_operation_failure_pair_has_one_model_independent_disposition"),
    ("python3", "scripts/effect_class_taxonomy_evidence.py"),
    ("python3", "scripts/retry_repair_policy_evidence.py"),
    ("python3", "scripts/story_5_2_policy_evidence.py"),
)
MARKERS: Final = (
    "test engineering_records::step_execution_policy_tests::story_5_2_every_registered_operation_failure_pair_has_one_model_independent_disposition ... ok",
    "Task 5.2.1 effect, failure, and registration taxonomy evidence validated",
    "Retry, repair, mutation, and race evidence validated through Sub-task 5.2.4.2",
    "Story 5.2 policy evidence validated through Sub-task 5.2.4.3",
)
RETAINED_PATHS: Final = (
    "kernel/contracts/src/engineering_records.rs",
    "kernel/engine/src/tooling.rs",
    "kernel/engine/src/retry_admission.rs",
    "artifacts/sprints/sprint-5/story-5.2/effect-class-taxonomy-report.json",
    "artifacts/sprints/sprint-5/story-5.2/effect-class-taxonomy-results.log",
    "artifacts/sprints/sprint-5/story-5.2/retry-repair-policy-report.json",
    "artifacts/sprints/sprint-5/story-5.2/retry-repair-policy-results.log",
    "artifacts/sprints/sprint-5/story-5.2/policy-verification-report.json",
)
TRUTH: Final = {
    "current_contract_and_registry_scope_complete": True,
    "registered_operation_count": 22,
    "effect_class_count": 7,
    "failure_class_count": 14,
    "operation_failure_pair_count": 308,
    "closed_failure_disposition_count": 9,
    "one_effect_class_per_registered_operation": True,
    "one_default_disposition_per_failure": True,
    "every_operation_failure_pair_deterministic": True,
    "model_created_classification_admitted": False,
    "caller_effect_override_admitted": False,
    "dispatches_after_policy_mutation": 0,
    "tool_effect_executed": False,
    "model_inference_executed": False,
    "synthetic_data_only": True,
    "native_platform_complete": False,
    "independent_review_complete": False,
    "story_completion_claim": False,
    "sprint_completion_claim": False,
    "release_claim": "none",
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def artifact(path: str) -> dict[str, Any]:
    value = ROOT / path
    return {"path": path, "byte_length": value.stat().st_size, "sha256": sha256(value)}


def load(path: str) -> dict[str, Any]:
    return json.loads((ROOT / path).read_text(encoding="utf-8"))


def expected_report() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "agentmage-story-acceptance-evidence",
        "story_id": "5.2",
        "criterion_id": "5.2.AC1",
        "generated_on": "2026-08-30",
        "status": "pass-local-current-contract-and-registry-scope",
        "commands": [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "retained_evidence": [artifact(path) for path in RETAINED_PATHS],
        "artifacts": [
            artifact("docs/verification/story-5-2-ac1-deterministic-disposition.md"),
            artifact("scripts/story_5_2_ac1_evidence.py"),
            artifact("tests/test_story_5_2_ac1_evidence.py"),
            artifact(RAW_PATH.relative_to(ROOT).as_posix()),
        ],
        "acceptance_truth": dict(TRUTH),
        "limitations": [
            "the criterion closes the current pure contract, registry, and admission-policy scope",
            "native provider execution and cross-platform installed-product evidence remain later gates",
            "independent review, Story, Sprint, packaging, and release completion are not claimed",
        ],
    }


def validate_upstream() -> list[str]:
    failures: list[str] = []
    taxonomy = load("artifacts/sprints/sprint-5/story-5.2/effect-class-taxonomy-report.json")
    retry = load("artifacts/sprints/sprint-5/story-5.2/retry-repair-policy-report.json")
    policy = load("artifacts/sprints/sprint-5/story-5.2/policy-verification-report.json")
    mappings = taxonomy.get("operation_effect_mappings", [])
    failures.extend([] if len(mappings) == 22 and len({item["operation"] for item in mappings}) == 22 else ["registered-operation mapping is incomplete"])
    failures.extend([] if len(taxonomy.get("effect_classes", [])) == 7 else ["effect taxonomy is incomplete"])
    failures.extend([] if len(taxonomy.get("failure_classes", [])) == 14 else ["failure taxonomy is incomplete"])
    dispositions = {item["default_disposition"] for item in taxonomy.get("failure_classes", [])}
    failures.extend([] if len(dispositions) == 9 else ["closed failure dispositions are incomplete"])
    mutation = retry.get("mutation_contract", {})
    if mutation.get("total_mutations") != 79 or mutation.get("effect_and_retry_field_mutations") != 9 or mutation.get("failure_and_uncertainty_field_mutations") != 14 or mutation.get("dispatches_after_mutation") != 0:
        failures.append("retry-policy mutation evidence is incomplete")
    if policy.get("decision_table", {}).get("registered_operation_count") != 22:
        failures.append("consolidated policy coverage is incomplete")
    if taxonomy.get("product_truth", {}).get("tool_effect_executed") is not False or retry.get("product_truth", {}).get("model_inference_executed") is not False:
        failures.append("criterion evidence executed an effect or model")
    return failures


def validate_raw(value: str) -> list[str]:
    failures = [f"raw results missing marker: {marker}" for marker in MARKERS if marker not in value]
    for prohibited in ("Traceback", "FAILED", "validation failed"):
        if prohibited in value:
            failures.append(f"raw results contain prohibited marker: {prohibited}")
    return failures


def validate_report(value: Any) -> list[str]:
    return [] if value == expected_report() else ["Story 5.2 AC1 report is stale, incomplete, reordered, or widened"]


def capture() -> tuple[str, int]:
    chunks = []
    for command in COMMANDS:
        result = subprocess.run(command, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, check=False)
        chunks.append(f"$ {' '.join(command)}\n{result.stdout.rstrip()}\n")
        if result.returncode != 0:
            return "".join(chunks), result.returncode
    return "".join(chunks), 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            raw, returncode = capture()
            if returncode != 0:
                sys.stderr.write(raw)
                return 1
            failures = validate_upstream() + validate_raw(raw)
            if failures:
                raise ValueError("; ".join(failures))
            EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
            RAW_PATH.write_text(raw, encoding="utf-8")
            REPORT_PATH.write_text(json.dumps(expected_report(), indent=2, sort_keys=True) + "\n", encoding="utf-8")
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        failures = validate_upstream() + validate_raw(raw) + validate_report(report)
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as error:
        print(f"Story 5.2 AC1 evidence failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Story 5.2 AC1 evidence failed: {failure}", file=sys.stderr)
        return 1
    print("Story acceptance criterion 5.2.AC1 deterministic retry disposition validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
