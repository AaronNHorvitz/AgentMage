#!/usr/bin/env python3
"""Build and validate fail-closed fuzz requirements for future story gates."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
import tempfile
from pathlib import Path, PurePosixPath
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
POLICY_PATH = ROOT / "fuzzing/story-gate-policy.json"
TARGET_REGISTRY_PATH = ROOT / "fuzzing/target-registry.json"
TOOLCHAIN_POLICY_PATH = ROOT / "fuzzing/toolchain-policy.json"
RESULT_SCHEMA_PATH = ROOT / "schemas/testing/fuzz-result.schema.json"
REPORT_PATH = ROOT / "artifacts/sprints/sprint-2/story-2.2/gate-policy-report.json"
EVIDENCE_FIELDS = (
    "target_id",
    "boundary_implemented",
    "target_registered",
    "target_registry_sha256",
    "corpus_path",
    "corpus_sha256",
    "resource_policy_sha256",
    "result_path",
    "result_sha256",
    "result_schema_valid",
    "result_status",
    "regression_path",
    "regression_replay_passed",
)
BLOCKING_STATUS = (
    "cancelled",
    "crash",
    "error",
    "hang",
    "path-escape",
    "resource-exhaustion",
    "secret-leak",
    "authorization-bypass",
    "timeout",
)


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-fuzz-story-gate-", dir=path.parent
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


def safe_relative_path(value: Any) -> bool:
    if not isinstance(value, str) or not value:
        return False
    path = PurePosixPath(value)
    return not path.is_absolute() and ".." not in path.parts and str(path) == value


def owner_gates(registry: dict[str, Any]) -> list[dict[str, Any]]:
    assignments: dict[int, list[str]] = {}
    for target in registry["targets"]:
        for sprint in target["first_execution_owner_sprints"]:
            assignments.setdefault(sprint, []).append(target["target_id"])
    return [
        {
            "sprint": sprint,
            "target_ids": sorted(target_ids),
            "target_count": len(target_ids),
            "gate_requirement": "all-targets-pass",
        }
        for sprint, target_ids in sorted(assignments.items())
    ]


def build_policy(root: Path = ROOT) -> dict[str, Any]:
    registry_path = root / TARGET_REGISTRY_PATH.relative_to(ROOT)
    toolchain_path = root / TOOLCHAIN_POLICY_PATH.relative_to(ROOT)
    schema_path = root / RESULT_SCHEMA_PATH.relative_to(ROOT)
    registry = read_json(registry_path)
    gates = owner_gates(registry)
    current_readiness = [
        {
            "sprint": gate["sprint"],
            "target_ids": gate["target_ids"],
            "status": "blocked-boundary-not-implemented",
            "blocking_target_ids": gate["target_ids"],
        }
        for gate in gates
    ]
    value = {
        "schema_version": 1,
        "policy_id": "agentmage-fuzz-story-gate-policy-v1",
        "policy_version": "1.0.0",
        "status": "enforced-registration-and-execution-policy",
        "inputs": {
            "target_registry": {
                "path": TARGET_REGISTRY_PATH.relative_to(ROOT).as_posix(),
                "sha256": sha256_file(registry_path),
            },
            "toolchain_policy": {
                "path": TOOLCHAIN_POLICY_PATH.relative_to(ROOT).as_posix(),
                "sha256": sha256_file(toolchain_path),
            },
            "result_schema": {
                "path": RESULT_SCHEMA_PATH.relative_to(ROOT).as_posix(),
                "sha256": sha256_file(schema_path),
            },
        },
        "required_evidence_fields": list(EVIDENCE_FIELDS),
        "owner_gates": gates,
        "evaluation_contract": {
            "boundary_implementation_required": True,
            "registration_required": True,
            "versioned_corpus_required": True,
            "resource_policy_required": True,
            "schema_valid_result_required": True,
            "result_status_required": "pass",
            "regression_path_required": True,
            "regression_replay_required": True,
            "missing_or_stale_evidence_disposition": "block",
            "failed_timed_out_cancelled_or_skipped_disposition": "block",
            "normal_development_waiver_permitted": False,
            "external_risk_decision_validator": "future-release-governance-boundary",
        },
        "current_readiness": current_readiness,
        "current_ready_gate_count": 0,
        "current_blocked_gate_count": len(gates),
        "product_boundary_execution_claim": "none",
        "network_used": False,
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }
    return {**value, "policy_sha256": sha256_bytes(canonical_json(value))}


def evaluate_target(
    target: dict[str, Any], evidence: dict[str, Any], policy: dict[str, Any]
) -> list[str]:
    target_id = target["target_id"]
    failures = []
    if set(evidence) != set(EVIDENCE_FIELDS):
        failures.append("evidence-field-closure")
    if evidence.get("target_id") != target_id:
        failures.append("target-identity")
    if evidence.get("boundary_implemented") is not True:
        failures.append("boundary-not-implemented")
    if evidence.get("target_registered") is not True:
        failures.append("target-not-registered")
    expected_registry = policy["inputs"]["target_registry"]["sha256"]
    if evidence.get("target_registry_sha256") != expected_registry:
        failures.append("target-registry-stale")
    if not safe_relative_path(evidence.get("corpus_path")) or not isinstance(
        evidence.get("corpus_sha256"), str
    ) or len(evidence.get("corpus_sha256", "")) != 64:
        failures.append("corpus-missing-or-invalid")
    expected_policy = policy["inputs"]["toolchain_policy"]["sha256"]
    if evidence.get("resource_policy_sha256") != expected_policy:
        failures.append("resource-policy-stale")
    if not safe_relative_path(evidence.get("result_path")) or not isinstance(
        evidence.get("result_sha256"), str
    ) or len(evidence.get("result_sha256", "")) != 64:
        failures.append("result-missing-or-invalid")
    if evidence.get("result_schema_valid") is not True:
        failures.append("result-schema-invalid")
    if evidence.get("result_status") != "pass":
        failures.append("result-non-pass")
    if not safe_relative_path(evidence.get("regression_path")):
        failures.append("regression-path-missing")
    if evidence.get("regression_replay_passed") is not True:
        failures.append("regression-replay-non-pass")
    return sorted(set(failures))


def evaluate_sprint_gate(
    sprint: int,
    evidence_records: list[dict[str, Any]],
    *,
    root: Path = ROOT,
) -> dict[str, Any]:
    policy = build_policy(root)
    registry = read_json(root / TARGET_REGISTRY_PATH.relative_to(ROOT))
    gate = next(
        (item for item in policy["owner_gates"] if item["sprint"] == sprint), None
    )
    if gate is None:
        return {
            "sprint": sprint,
            "status": "not-applicable-no-registered-target",
            "target_results": [],
            "blocking_target_count": 0,
        }
    by_target = {item.get("target_id"): item for item in evidence_records}
    if len(by_target) != len(evidence_records):
        raise ValueError("fuzz gate received duplicate evidence target identities")
    target_contracts = {item["target_id"]: item for item in registry["targets"]}
    results = []
    for target_id in gate["target_ids"]:
        evidence = by_target.get(target_id, {})
        failures = evaluate_target(target_contracts[target_id], evidence, policy)
        results.append(
            {
                "target_id": target_id,
                "status": "pass" if not failures else "block",
                "failures": failures,
            }
        )
    extras = sorted(set(by_target) - set(gate["target_ids"]))
    if extras:
        raise ValueError(f"fuzz gate received out-of-scope evidence: {extras}")
    blocking = sum(item["status"] == "block" for item in results)
    return {
        "sprint": sprint,
        "status": "pass" if blocking == 0 else "block",
        "target_results": results,
        "blocking_target_count": blocking,
    }


def validate_policy(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["fuzz story-gate policy must be an object"]
    failures: list[str] = []
    if (
        value.get("schema_version") != 1
        or value.get("policy_id") != "agentmage-fuzz-story-gate-policy-v1"
        or value.get("policy_version") != "1.0.0"
    ):
        failures.append("fuzz story-gate policy identity is invalid")
    if value.get("required_evidence_fields") != list(EVIDENCE_FIELDS):
        failures.append("fuzz story-gate evidence closure is invalid")
    gates = value.get("owner_gates", [])
    if len(gates) != 13 or sum(item.get("target_count", 0) for item in gates) != 14:
        failures.append("fuzz story-gate owner coverage is invalid")
    readiness = value.get("current_readiness", [])
    if len(readiness) != len(gates) or any(
        item.get("status") != "blocked-boundary-not-implemented" for item in readiness
    ):
        failures.append("fuzz story-gate current readiness overclaimed completion")
    contract = value.get("evaluation_contract", {})
    if (
        contract.get("normal_development_waiver_permitted") is not False
        or contract.get("missing_or_stale_evidence_disposition") != "block"
        or contract.get("failed_timed_out_cancelled_or_skipped_disposition") != "block"
    ):
        failures.append("fuzz story-gate evaluation does not fail closed")
    if value.get("current_ready_gate_count") != 0 or value.get(
        "current_blocked_gate_count"
    ) != len(gates):
        failures.append("fuzz story-gate readiness counts are invalid")
    if value.get("product_boundary_execution_claim") != "none" or value.get(
        "network_used"
    ) is not False:
        failures.append("fuzz story-gate policy made an execution claim")
    if value.get("macos_execution_status") != "blocked-macos" or value.get(
        "macos_support_claim"
    ) != "none":
        failures.append("fuzz story-gate policy made an invalid macOS claim")
    try:
        expected = build_policy(root)
    except (OSError, ValueError, KeyError, TypeError) as error:
        failures.append(f"cannot rebuild fuzz story-gate policy: {error}")
    else:
        if value != expected:
            failures.append("fuzz story-gate policy is stale or non-deterministic")
    return failures


def build_report(root: Path = ROOT) -> dict[str, Any]:
    policy_path = root / POLICY_PATH.relative_to(ROOT)
    policy = read_json(policy_path)
    failures = validate_policy(policy, root)
    if failures:
        raise ValueError("; ".join(failures))
    return {
        "schema_version": 1,
        "task_id": "2.2.1.4",
        "status": "pass-fail-closed-gate-policy",
        "policy": {
            "path": POLICY_PATH.relative_to(ROOT).as_posix(),
            "sha256": sha256_file(policy_path),
            "self_sha256": policy["policy_sha256"],
            "version": policy["policy_version"],
        },
        "registered_target_count": 12,
        "owner_gate_count": len(policy["owner_gates"]),
        "target_gate_assignment_count": sum(
            item["target_count"] for item in policy["owner_gates"]
        ),
        "required_evidence_field_count": len(EVIDENCE_FIELDS),
        "current_ready_gate_count": 0,
        "current_blocked_gate_count": len(policy["owner_gates"]),
        "normal_development_waiver_permitted": False,
        "seeded_gate_mutations_required": True,
        "product_boundary_execution_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }


def validate_report(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["fuzz story-gate policy report must be an object"]
    failures: list[str] = []
    if (
        value.get("schema_version") != 1
        or value.get("task_id") != "2.2.1.4"
        or value.get("status") != "pass-fail-closed-gate-policy"
    ):
        failures.append("fuzz story-gate policy report identity is invalid")
    if (
        value.get("registered_target_count") != 12
        or value.get("owner_gate_count") != 13
        or value.get("target_gate_assignment_count") != 14
        or value.get("required_evidence_field_count") != len(EVIDENCE_FIELDS)
        or value.get("current_ready_gate_count") != 0
        or value.get("current_blocked_gate_count") != 13
        or value.get("normal_development_waiver_permitted") is not False
    ):
        failures.append("fuzz story-gate policy report closure is invalid")
    if value.get("product_boundary_execution_claim") != "none":
        failures.append("fuzz story-gate policy report made a product claim")
    if value.get("macos_execution_status") != "blocked-macos" or value.get(
        "macos_support_claim"
    ) != "none":
        failures.append("fuzz story-gate policy report made an invalid macOS claim")
    try:
        expected = build_report(root)
    except (OSError, ValueError, KeyError, TypeError) as error:
        failures.append(f"cannot rebuild fuzz story-gate report: {error}")
    else:
        if value != expected:
            failures.append("fuzz story-gate report is stale or non-deterministic")
    return failures


def write_artifacts(root: Path = ROOT) -> None:
    policy_path = root / POLICY_PATH.relative_to(ROOT)
    write_atomic(policy_path, canonical_json(build_policy(root)))
    write_atomic(root / REPORT_PATH.relative_to(ROOT), canonical_json(build_report(root)))


def check_artifacts(root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    try:
        policy = read_json(root / POLICY_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        failures.append(f"cannot read fuzz story-gate policy: {error}")
    else:
        failures.extend(validate_policy(policy, root))
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        failures.append(f"cannot read fuzz story-gate policy report: {error}")
    else:
        failures.extend(validate_report(report, root))
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            write_artifacts()
        failures = check_artifacts()
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"fuzz story gate failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"fuzz story gate failed: {failure}", file=sys.stderr)
        return 1
    print("Story 2.2 future fuzz story-gate policy validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
