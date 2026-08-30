#!/usr/bin/env python3
"""Register Story 2.3 parser and workflow boundaries for deferred fuzzing."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import os
import sys
import tempfile
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
REGISTRY_PATH: Final = ROOT / "fuzzing" / "artifact-workflow-target-registry-v1.json"
PARENT_REGISTRY_PATH: Final = ROOT / "fuzzing" / "target-registry.json"
TOOLCHAIN_PATH: Final = ROOT / "fuzzing" / "toolchain-policy.json"
GENERIC_SEED_PATH: Final = ROOT / "fuzzing" / "seeds" / "security-failures-v1.json"
DECISION_PATH: Final = ROOT / "docs" / "decisions" / "0025-final-manual-fuzz-campaign.md"
EVALUATION_DIR: Final = ROOT / "fixtures" / "artifact-evaluation" / "v1"
INPUT_PATHS: Final = (
    PARENT_REGISTRY_PATH,
    TOOLCHAIN_PATH,
    GENERIC_SEED_PATH,
    DECISION_PATH,
    EVALUATION_DIR / "text-reference-manifest.json",
    EVALUATION_DIR / "document-variant-manifest.json",
    EVALUATION_DIR / "lifecycle-scenarios.json",
    EVALUATION_DIR / "workflow-plan-fixtures.json",
    EVALUATION_DIR / "workflow-crash-points.json",
    EVALUATION_DIR / "workflow-terminal-outcomes.json",
    EVALUATION_DIR / "artifact-golden-metrics.json",
    EVALUATION_DIR / "workflow-golden-metrics.json",
    EVALUATION_DIR / "golden-manifest-v1.json",
    EVALUATION_DIR / "reproducibility-report.json",
    ROOT / "artifacts" / "sprints" / "sprint-2" / "story-2.1" / "fixture-security-scan-report.json",
)
ZERO_SHA256: Final = "0" * 64
REGISTRATION_SPECS: Final = (
    (
        "AE-PARSER-TEXT-001",
        "artifact-text-and-reference-parser",
        ("FT-TEXT-001",),
        ("fixtures/artifact-evaluation/v1/text-reference-manifest.json",),
        "text_parser_oracle",
    ),
    (
        "AE-PARSER-DOCUMENT-001",
        "artifact-document-and-package-parser",
        ("FT-ARCHIVE-001",),
        ("fixtures/artifact-evaluation/v1/document-variant-manifest.json",),
        "document_parser_oracle",
    ),
    (
        "AE-PROVENANCE-001",
        "artifact-provenance-and-golden-manifest",
        ("FT-MANIFEST-001",),
        (
            "fixtures/artifact-evaluation/v1/lifecycle-scenarios.json",
            "fixtures/artifact-evaluation/v1/golden-manifest-v1.json",
        ),
        "provenance_oracle",
    ),
    (
        "AE-WORKFLOW-PLAN-001",
        "workflow-plan-decoder",
        ("FT-MODEL-OUTPUT-001",),
        ("fixtures/artifact-evaluation/v1/workflow-plan-fixtures.json",),
        "plan_oracle",
    ),
    (
        "AE-WORKFLOW-APPROVAL-001",
        "workflow-approval-boundary",
        ("FT-GRANT-001",),
        ("fixtures/artifact-evaluation/v1/workflow-plan-fixtures.json",),
        "approval_oracle",
    ),
    (
        "AE-WORKFLOW-RETRY-001",
        "workflow-retry-and-attempt-boundary",
        ("FT-IPC-001",),
        (
            "fixtures/artifact-evaluation/v1/workflow-plan-fixtures.json",
            "fixtures/artifact-evaluation/v1/workflow-crash-points.json",
        ),
        "retry_oracle",
    ),
    (
        "AE-WORKFLOW-CRASH-001",
        "workflow-crash-and-recovery-boundary",
        ("FT-IPC-001",),
        ("fixtures/artifact-evaluation/v1/workflow-crash-points.json",),
        "crash_oracle",
    ),
    (
        "AE-WORKFLOW-TERMINAL-001",
        "workflow-terminal-transition-boundary",
        ("FT-IPC-001",),
        ("fixtures/artifact-evaluation/v1/workflow-terminal-outcomes.json",),
        "terminal_oracle",
    ),
)
MANUAL_CAMPAIGN_CONTRACT: Final = {
    "campaign_id": "RM-024",
    "decision": "0025",
    "required": True,
    "execution_status": "deferred-not-executed",
    "manual_supervision_required": True,
    "source_freeze_required_before_execution": True,
    "real_fuzz_engine_executed": False,
    "sanitizer_result": None,
    "coverage_result": None,
    "duration_seconds": None,
    "crash_disposition": None,
    "synthetic_evidence_substitution_permitted": False,
    "blocking_gates": ["affected-RV-15", "Sprint-166", "G-GA"],
}


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def read_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-artifact-fuzz-registration-", dir=path.parent
    )
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
        temporary.chmod(0o644)
        os.replace(temporary, path)
    except BaseException:
        temporary.unlink(missing_ok=True)
        raise


def sealed(value: dict[str, Any]) -> dict[str, Any]:
    result = copy.deepcopy(value)
    result["registry_sha256"] = ZERO_SHA256
    result["registry_sha256"] = sha256_bytes(canonical_json(result))
    return result


def valid_hash(value: dict[str, Any]) -> bool:
    unhashed = copy.deepcopy(value)
    recorded = unhashed.get("registry_sha256")
    unhashed["registry_sha256"] = ZERO_SHA256
    return recorded == sha256_bytes(canonical_json(unhashed))


def metric_index() -> dict[str, dict[str, Any]]:
    metrics = read_json(EVALUATION_DIR / "workflow-golden-metrics.json")["metrics"]
    return {item["metric_id"]: item["golden"] for item in metrics}


def build_oracles() -> dict[str, dict[str, Any]]:
    text = read_json(EVALUATION_DIR / "text-reference-manifest.json")
    documents = read_json(EVALUATION_DIR / "document-variant-manifest.json")
    golden = read_json(EVALUATION_DIR / "golden-manifest-v1.json")
    metrics = metric_index()
    plan = metrics["plan_completion"]
    approval = metrics["approval_bypass"]
    attempts = metrics["attempt_count"]
    recovery = metrics["recovery_quality"]
    termination = metrics["bounded_termination"]
    diagnosis = metrics["terminal_diagnosis"]
    return {
        "text_parser_oracle": {
            "seed_case_count": text["case_count"],
            "non_success_case_count": sum(
                item["expected_disposition"] != "captured" for item in text["cases"]
            ),
            "oversized_cases_preserved": 1,
            "network_effects": 0,
        },
        "document_parser_oracle": {
            "seed_case_count": documents["case_count"],
            "non_success_case_count": sum(
                item["expected_disposition"] != "captured"
                for item in documents["cases"]
            ),
            "hostile_relationship_cases_preserved": 2,
            "active_content_executions": 0,
        },
        "provenance_oracle": {
            "seed_case_count": golden["outcome_count"],
            "required_input_count": len(golden["required_inputs"]),
            "non_success_outcomes_preserved": golden["non_success_terminal_count"],
            "changed_identity_allowed": False,
        },
        "plan_oracle": {
            "seed_case_count": plan["plan_count"],
            "verified_success_count": plan["verified_success_count"],
            "explicit_non_success_count": plan["explicit_non_success_count"],
            "open_plan_completion_allowed": plan["open_plan_completion_allowed"],
        },
        "approval_oracle": {
            "seed_case_count": approval["approval_required_step_count"],
            "approval_bypass_count": approval["approval_bypass_count"],
            "authority_minted_by_fixture": approval["authority_minted_by_fixture"],
        },
        "retry_oracle": {
            "seed_case_count": attempts["record_count"],
            "total_attempts": attempts["total_attempts"],
            "maximum_attempts": attempts["maximum_attempts"],
            "fresh_identity_required_when_retried": attempts[
                "fresh_identity_required_when_retried"
            ],
        },
        "crash_oracle": {
            "seed_case_count": recovery["crash_oracle_count"],
            "safe_recovery_count": recovery["crash_oracle_count"],
            "replay_allowed": recovery["replay_allowed"],
            "duplicate_effect_count": 0,
        },
        "terminal_oracle": {
            "seed_case_count": termination["terminal_fixture_count"],
            "false_completion_count": termination["false_completion_count"],
            "artificial_continue_count": termination["artificial_continue_count"],
            "non_cancelled_diagnosis_count": diagnosis[
                "non_cancelled_diagnosis_count"
            ],
            "cancelled_diagnosis_count": diagnosis["cancelled_diagnosis_count"],
        },
    }


def input_ledger() -> list[dict[str, Any]]:
    return [
        {
            "path": path.relative_to(ROOT).as_posix(),
            "byte_length": path.stat().st_size,
            "sha256": sha256_bytes(path.read_bytes()),
        }
        for path in INPUT_PATHS
    ]


def build_registrations() -> list[dict[str, Any]]:
    parent = read_json(PARENT_REGISTRY_PATH)
    parent_ids = {item["target_id"] for item in parent["targets"]}
    oracles = build_oracles()
    registrations = []
    for registration_id, boundary, target_ids, seed_paths, oracle_id in REGISTRATION_SPECS:
        if any(target_id not in parent_ids for target_id in target_ids):
            raise ValueError(f"registration names an unknown parent target: {registration_id}")
        oracle = oracles[oracle_id]
        registrations.append(
            {
                "registration_id": registration_id,
                "boundary": boundary,
                "parent_target_ids": list(target_ids),
                "promotion_status": "registered-pending-rm-024",
                "seed_inputs": [
                    {
                        "path": relative,
                        "sha256": sha256_bytes((ROOT / relative).read_bytes()),
                    }
                    for relative in seed_paths
                ],
                "deterministic_fault_oracle": {
                    "oracle_id": oracle_id,
                    "execution_class": "fixed-seed-oracle-not-fuzzing",
                    "status": "pass",
                    **oracle,
                },
                "fault_harness": {
                    "engine_id": "agentmage-bounded-fake-fuzzer",
                    "generic_seed_corpus": GENERIC_SEED_PATH.relative_to(ROOT).as_posix(),
                    "malformed_and_fault_replay_required": True,
                    "regression_replay_required": True,
                    "real_fuzz_equivalence": False,
                },
                "network_calls": 0,
                "product_runtime_executed": False,
                "authority_minted": False,
            }
        )
    return registrations


def build_registry() -> dict[str, Any]:
    parent = read_json(PARENT_REGISTRY_PATH)
    policy = read_json(TOOLCHAIN_PATH)
    registrations = build_registrations()
    value = {
        "schema_version": 1,
        "registry_id": "agentmage-artifact-workflow-fuzz-targets-v1",
        "registry_version": "1.0.0",
        "task_id": "2.3.4.3",
        "status": "registration-complete-manual-fuzz-deferred",
        "extension_contract": {
            "parent_registry_path": PARENT_REGISTRY_PATH.relative_to(ROOT).as_posix(),
            "parent_registry_version": parent["registry_version"],
            "parent_registry_self_sha256": parent["registry_sha256"],
            "toolchain_policy_path": TOOLCHAIN_PATH.relative_to(ROOT).as_posix(),
            "toolchain_policy_version": policy["policy_version"],
            "toolchain_policy_self_sha256": policy["policy_sha256"],
            "baseline_registry_mutated": False,
        },
        "input_count": len(INPUT_PATHS),
        "inputs": input_ledger(),
        "registration_count": len(registrations),
        "registrations": registrations,
        "deterministic_oracle_summary": {
            "oracle_count": len(registrations),
            "passing_oracle_count": sum(
                item["deterministic_fault_oracle"]["status"] == "pass"
                for item in registrations
            ),
            "seed_case_observation_count": sum(
                item["deterministic_fault_oracle"]["seed_case_count"]
                for item in registrations
            ),
            "fixed_seed_oracles_are_real_fuzzing": False,
            "sanitizer_execution_claim": "none",
            "coverage_claim": "none",
        },
        "manual_campaign": MANUAL_CAMPAIGN_CONTRACT,
        "synthetic_only": True,
        "network_calls": 0,
        "external_processes": 0,
        "private_user_data_used": False,
        "real_credentials_used": False,
        "raw_payloads_retained": False,
        "product_runtime_executed": False,
        "product_support_claim": "none",
        "release_claim": "none",
        "generator": {
            "path": "scripts/artifact_evaluation_fuzz_registration.py",
            "sha256": sha256_bytes(Path(__file__).read_bytes()),
        },
        "registry_sha256": ZERO_SHA256,
    }
    return sealed(value)


def validate_registry(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["artifact/workflow fuzz registration must be an object"]
    failures: list[str] = []
    if not valid_hash(value):
        failures.append("artifact/workflow fuzz registration self-hash is invalid")
    if (
        value.get("registry_id") != "agentmage-artifact-workflow-fuzz-targets-v1"
        or value.get("registry_version") != "1.0.0"
        or value.get("task_id") != "2.3.4.3"
        or value.get("status") != "registration-complete-manual-fuzz-deferred"
    ):
        failures.append("artifact/workflow fuzz registration identity is invalid")
    if value.get("input_count") != len(INPUT_PATHS) or value.get("inputs") != input_ledger():
        failures.append("artifact/workflow fuzz registration input ledger is stale or incomplete")
    expected_registrations = build_registrations()
    if (
        value.get("registration_count") != len(REGISTRATION_SPECS)
        or value.get("registrations") != expected_registrations
    ):
        failures.append("artifact/workflow parser or fault boundary registration is incomplete")
    expected_summary = build_registry()["deterministic_oracle_summary"]
    if value.get("deterministic_oracle_summary") != expected_summary:
        failures.append("artifact/workflow deterministic fault-oracle accounting is false")
    if value.get("manual_campaign") != MANUAL_CAMPAIGN_CONTRACT:
        failures.append("Decision 0025 manual fuzz deferral or blocking gate was weakened")
    parent = read_json(PARENT_REGISTRY_PATH)
    policy = read_json(TOOLCHAIN_PATH)
    expected_extension = {
        "parent_registry_path": PARENT_REGISTRY_PATH.relative_to(ROOT).as_posix(),
        "parent_registry_version": parent["registry_version"],
        "parent_registry_self_sha256": parent["registry_sha256"],
        "toolchain_policy_path": TOOLCHAIN_PATH.relative_to(ROOT).as_posix(),
        "toolchain_policy_version": policy["policy_version"],
        "toolchain_policy_self_sha256": policy["policy_sha256"],
        "baseline_registry_mutated": False,
    }
    if value.get("extension_contract") != expected_extension:
        failures.append("artifact/workflow fuzz extension is detached from the baseline")
    prohibited = (
        value.get("synthetic_only") is not True
        or value.get("network_calls") != 0
        or value.get("external_processes") != 0
        or value.get("private_user_data_used") is not False
        or value.get("real_credentials_used") is not False
        or value.get("raw_payloads_retained") is not False
        or value.get("product_runtime_executed") is not False
        or value.get("product_support_claim") != "none"
        or value.get("release_claim") != "none"
    )
    if prohibited:
        failures.append("artifact/workflow registration retained data, effects, or an overclaim")
    try:
        expected = build_registry()
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
        failures.append(f"cannot rebuild artifact/workflow fuzz registration: {error}")
    else:
        if value != expected:
            failures.append("checked artifact/workflow fuzz registration is stale or widened")
    return failures


def check() -> list[str]:
    try:
        actual = read_json(REGISTRY_PATH)
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read artifact/workflow fuzz registration: {error}"]
    return validate_registry(actual)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    if args.write:
        write_atomic(REGISTRY_PATH, canonical_json(build_registry()))
    failures = check()
    if failures:
        for failure in failures:
            print(f"Artifact/workflow fuzz registration failed: {failure}", file=sys.stderr)
        return 1
    print(
        "Registered 8 artifact/workflow boundaries; RM-024 remains deferred and blocking"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
