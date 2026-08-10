#!/usr/bin/env python3
"""Build and validate Story 2.2 fuzz-foundation security evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
import tempfile
from pathlib import Path, PurePosixPath
from typing import Any, Callable


ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.fuzz_baseline_reconciliation import (  # noqa: E402
    check_report as check_baseline,
)
from scripts.fuzz_result_contract import check_artifacts as check_result_contract  # noqa: E402
from scripts.fuzz_story_gate import check_artifacts as check_gate_policy  # noqa: E402
from scripts.fuzz_target_registry import check_artifacts as check_registry  # noqa: E402
from scripts.fuzz_toolchain_policy import check_artifacts as check_toolchain  # noqa: E402
from scripts.seeded_fuzz_failures import check_artifacts as check_seeded  # noqa: E402


EVIDENCE_ROOT = ROOT / "artifacts/sprints/sprint-2/story-2.2"
PROTOCOL_PATH = EVIDENCE_ROOT / "rv-15-control-map.json"
DISPOSITION_PATH = EVIDENCE_ROOT / "reviewer-disposition.json"
MAP_PATH = EVIDENCE_ROOT / "security-evidence-map.json"
EXPECTED_REQUIREMENTS = (
    "SR-SUP-009",
    "SR-TST-002",
    "SR-TST-004",
    "SR-TST-006",
    "SR-TST-012",
)
REGRESSION_PATHS = (
    "fuzzing/regressions/FT-GRANT-001/7f84c8c9be1e625621626ae0cef24fb24783041fd38856ec4d255d2a2ee3541a.bin",
    "fuzzing/regressions/FT-IPC-001/b1432642907bf9ece49a1ea6cb8c54bebd9f1e5ca2df66bfd995cf6a7a6f73c9.bin",
    "fuzzing/regressions/FT-MANIFEST-001/6f93481fe2c111b1c44176bfa27e6461bf26668e0c6c31fab4a32fa8a4f663e5.bin",
    "fuzzing/regressions/FT-MODEL-OUTPUT-001/1e88bb017cece80e46f2c16182e132f91161d3481388f7ab9259f783fc269ccc.bin",
    "fuzzing/regressions/FT-PATH-001/fa08499e14d0113ba6794623f1badedcc8e9ae51cb5bafc7e14a5af1454bcfe7.bin",
    "fuzzing/regressions/FT-TEXT-001/e1a480b8acb07821ae78acd569b61b07eb629ab423ace44b6d4e7179fe1e9c43.bin",
)
BASE_EVIDENCE_PATHS = (
    "fuzzing/target-registry.json",
    "fuzzing/toolchain-policy.json",
    "fuzzing/story-gate-policy.json",
    "fuzzing/seeds/security-failures-v1.json",
    "schemas/testing/fuzz-result.schema.json",
    "artifacts/sprints/sprint-2/story-2.2/target-registry-report.json",
    "artifacts/sprints/sprint-2/story-2.2/toolchain-policy-report.json",
    "artifacts/sprints/sprint-2/story-2.2/fuzz-result-schema-report.json",
    "artifacts/sprints/sprint-2/story-2.2/gate-policy-report.json",
    "artifacts/sprints/sprint-2/story-2.2/seeded-failure-report.json",
    "artifacts/sprints/sprint-2/story-2.2/baseline-reconciliation-report.json",
    *REGRESSION_PATHS,
)
MAP_EVIDENCE_PATHS = (
    *BASE_EVIDENCE_PATHS,
    "artifacts/sprints/sprint-2/story-2.2/rv-15-control-map.json",
    "artifacts/sprints/sprint-2/story-2.2/reviewer-disposition.json",
)
MAPPINGS = {
    "SR-SUP-009": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "fuzzing/target-registry.json",
            "fuzzing/toolchain-policy.json",
            "artifacts/sprints/sprint-2/story-2.2/target-registry-report.json",
        ],
        "demonstrated": "The registry inventories the FFI boundary class, currently discovers zero active FFI boundaries, and requires sanitizers plus focused fuzz ownership when one appears.",
        "remaining": "Every future unsafe block, FFI edge, native library, privilege, and entitlement still requires concrete inventory, justification, review, and product-boundary fuzz evidence.",
    },
    "SR-TST-002": {
        "story_contribution": "foundation-established",
        "evidence": [
            "fuzzing/target-registry.json",
            "fuzzing/toolchain-policy.json",
            "fuzzing/seeds/security-failures-v1.json",
            "artifacts/sprints/sprint-2/story-2.2/baseline-reconciliation-report.json",
        ],
        "demonstrated": "Twelve required trust-boundary classes have versioned target contracts, pinned fuzz policy, synthetic seeds, normalized results, minimization, and deterministic clean-run reconciliation.",
        "remaining": "Each concrete product parser and protocol must execute its registered target for the required durations with real coverage and sanitizer evidence before its owner gate can pass.",
    },
    "SR-TST-004": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "fuzzing/seeds/security-failures-v1.json",
            "artifacts/sprints/sprint-2/story-2.2/seeded-failure-report.json",
            *REGRESSION_PATHS,
        ],
        "demonstrated": "Six bounded hostile failure classes are deterministically detected, minimized, retained, owned, and prevented from becoming passes without exposing the synthetic secret canary.",
        "remaining": "Every later product input class must exercise malformed, hostile, oversized, partial, stale, and conflicting cases against its implemented boundary and prove bounded side effects.",
    },
    "SR-TST-006": {
        "story_contribution": "partial-story-evidence",
        "evidence": [
            "fuzzing/toolchain-policy.json",
            "artifacts/sprints/sprint-2/story-2.2/seeded-failure-report.json",
            "artifacts/sprints/sprint-2/story-2.2/baseline-reconciliation-report.json",
        ],
        "demonstrated": "Input, duration, memory, output, corpus, concurrency, minimization, and retention budgets are pinned; logical hang and memory-limit crossings remain blocking without exhausting the host.",
        "remaining": "Integrated product processes must enforce CPU, GPU, memory, disk, context, file, output, and concurrency limits while proving cancellation, cleanup, audit, and responsiveness.",
    },
    "SR-TST-012": {
        "story_contribution": "foundation-established",
        "evidence": [
            "fuzzing/story-gate-policy.json",
            "artifacts/sprints/sprint-2/story-2.2/gate-policy-report.json",
            "artifacts/sprints/sprint-2/story-2.2/seeded-failure-report.json",
        ],
        "demonstrated": "Every seeded non-pass result forces its target gate to block, missing or stale evidence fails closed, and ordinary development credentials have no waiver path.",
        "remaining": "The integrated release process must bind these target gates to package production and validate any exceptional signed, dated risk decision outside ordinary development credentials.",
    },
}


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def safe_relative_path(value: Any) -> bool:
    if not isinstance(value, str) or not value:
        return False
    path = PurePosixPath(value)
    return not path.is_absolute() and ".." not in path.parts and str(path) == value


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-story-2-2-security-", dir=path.parent
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


def validate_foundation_inputs(root: Path = ROOT) -> list[str]:
    validators: tuple[tuple[str, Callable[[Path], list[str]]], ...] = (
        ("target-registry", check_registry),
        ("toolchain-policy", check_toolchain),
        ("result-contract", check_result_contract),
        ("story-gate-policy", check_gate_policy),
        ("seeded-failures", check_seeded),
        ("baseline-reconciliation", check_baseline),
    )
    failures = []
    for name, validator in validators:
        failures.extend(f"{name}: {failure}" for failure in validator(root))
    return failures


def evidence_records(root: Path, paths: tuple[str, ...]) -> list[dict[str, Any]]:
    return [
        {
            "path": path,
            "sha256": sha256_file(root / path),
            "bytes": (root / path).stat().st_size,
        }
        for path in paths
    ]


def build_protocol(root: Path = ROOT) -> dict[str, Any]:
    failures = validate_foundation_inputs(root)
    if failures:
        raise ValueError("; ".join(failures))
    registry = read_json(root / "fuzzing/target-registry.json")
    seeded = read_json(
        root / "artifacts/sprints/sprint-2/story-2.2/seeded-failure-report.json"
    )
    baseline = read_json(
        root
        / "artifacts/sprints/sprint-2/story-2.2/baseline-reconciliation-report.json"
    )
    gate = read_json(root / "fuzzing/story-gate-policy.json")
    return {
        "schema_version": 1,
        "record_type": "story_2_2_rv_15_control_map",
        "protocol_id": "RV-15",
        "foundation_execution_status": "COMPLETE",
        "foundation_result": "PASS_WITH_RETAINED_BLOCKING_FIXTURES",
        "full_protocol_status": "NOT_COMPLETE",
        "completed": [
            "Twelve baseline trust-boundary target contracts are registered with owner-sprint gates.",
            "Pinned engines, sanitizers, dictionaries, corpora, budgets, signatures, minimization, and regression-retention rules are hash-bound.",
            "Six security failure classes produce schema-shaped non-pass results, minimized reproducers, ownership, and blocking gate dispositions.",
            "Two clean fake-boundary runs reconcile target identity, classification, coverage, evidence, and regression hashes exactly.",
        ],
        "remaining": [
            "Execute every target against its implemented product boundary for the policy duration and retain coverage, sanitizer, and raw result evidence.",
            "Inventory and fuzz every future unsafe, FFI, native, parser, IPC, model-output, path, Git, SQLite, archive, and text-decoding boundary.",
            "Complete required macOS execution separately; Linux or fake-boundary evidence cannot substitute for macOS evidence.",
            "Repeat RV-15 continuously for changed boundaries and against the integrated release candidate in Sprint 25.",
        ],
        "observations": {
            "registered_target_count": len(registry["targets"]),
            "active_ffi_boundary_count": registry["ffi_discovery"][
                "active_boundary_count"
            ],
            "owner_gate_count": len(gate["owner_gates"]),
            "seeded_non_pass_result_count": seeded["summary"][
                "non_pass_result_count"
            ],
            "minimized_reproducer_count": seeded["summary"][
                "minimized_reproducer_count"
            ],
            "blocking_gate_result_count": seeded["summary"][
                "blocking_gate_result_count"
            ],
            "clean_run_count": baseline["summary"]["clean_run_count"],
            "clean_runs_reconcile_exactly": all(baseline["comparison"].values()),
        },
        "evidence": evidence_records(root, BASE_EVIDENCE_PATHS),
        "private_user_data_used": False,
        "network_used": False,
        "product_boundary_execution_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_evidence_substituted": False,
        "release_claim": "none",
    }


def validate_protocol(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["RV-15 control map must be an object"]
    failures: list[str] = []
    if (
        value.get("schema_version") != 1
        or value.get("record_type") != "story_2_2_rv_15_control_map"
        or value.get("protocol_id") != "RV-15"
        or value.get("foundation_execution_status") != "COMPLETE"
        or value.get("foundation_result")
        != "PASS_WITH_RETAINED_BLOCKING_FIXTURES"
        or value.get("full_protocol_status") != "NOT_COMPLETE"
    ):
        failures.append("RV-15 control-map identity or scope is invalid")
    observations = value.get("observations", {})
    if observations != {
        "registered_target_count": 12,
        "active_ffi_boundary_count": 0,
        "owner_gate_count": 13,
        "seeded_non_pass_result_count": 6,
        "minimized_reproducer_count": 6,
        "blocking_gate_result_count": 6,
        "clean_run_count": 2,
        "clean_runs_reconcile_exactly": True,
    }:
        failures.append("RV-15 observations are incomplete or invalid")
    records = value.get("evidence", [])
    paths = [item.get("path") for item in records]
    if paths != list(BASE_EVIDENCE_PATHS) or len(paths) != len(set(paths)):
        failures.append("RV-15 retained evidence closure is invalid")
    for item in records:
        path = item.get("path")
        if not safe_relative_path(path) or not (root / path).is_file():
            failures.append(f"RV-15 evidence path is invalid: {path}")
        elif item.get("sha256") != sha256_file(root / path) or item.get(
            "bytes"
        ) != (root / path).stat().st_size:
            failures.append(f"RV-15 evidence identity is invalid: {path}")
    if (
        value.get("private_user_data_used") is not False
        or value.get("network_used") is not False
        or value.get("product_boundary_execution_claim") != "none"
        or value.get("macos_execution_status") != "blocked-macos"
        or value.get("macos_evidence_substituted") is not False
        or value.get("release_claim") != "none"
    ):
        failures.append("RV-15 control map made an unsupported claim")
    try:
        expected = build_protocol(root)
    except (OSError, ValueError, KeyError, TypeError) as error:
        failures.append(f"cannot rebuild RV-15 control map: {error}")
    else:
        if value != expected:
            failures.append("RV-15 control map is stale or non-deterministic")
    return failures


def build_disposition(root: Path = ROOT) -> dict[str, Any]:
    protocol = read_json(root / PROTOCOL_PATH.relative_to(ROOT))
    failures = validate_protocol(protocol, root)
    if failures:
        raise ValueError("; ".join(failures))
    return {
        "schema_version": 1,
        "record_type": "story_2_2_security_reviewer_disposition",
        "evidence_date": "2026-08-10",
        "task_2_2_2_3_status": "COMPLETE",
        "rv_15_foundation_status": "COMPLETE",
        "rv_15_full_protocol_status": "NOT_COMPLETE",
        "shared_linux_foundation_status": "PASS",
        "story_2_2_status": "BLOCKED_REQUIRED_MAC_HARDWARE",
        "seeded_security_failures_retained": True,
        "ordinary_development_waiver_permitted": False,
        "product_boundaries_tested": False,
        "independent_review_performed": False,
        "independent_review_status": "PENDING",
        "macos_evidence_substituted": False,
        "release_approval": False,
    }


def validate_disposition(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["Story 2.2 reviewer disposition must be an object"]
    failures: list[str] = []
    if (
        value.get("schema_version") != 1
        or value.get("record_type")
        != "story_2_2_security_reviewer_disposition"
        or value.get("task_2_2_2_3_status") != "COMPLETE"
        or value.get("rv_15_foundation_status") != "COMPLETE"
        or value.get("rv_15_full_protocol_status") != "NOT_COMPLETE"
    ):
        failures.append("Story 2.2 reviewer disposition identity is invalid")
    if (
        value.get("product_boundaries_tested") is not False
        or value.get("independent_review_performed") is not False
        or value.get("independent_review_status") != "PENDING"
        or value.get("macos_evidence_substituted") is not False
        or value.get("release_approval") is not False
    ):
        failures.append("Story 2.2 reviewer disposition overclaimed review or approval")
    try:
        expected = build_disposition(root)
    except (OSError, ValueError, KeyError, TypeError) as error:
        failures.append(f"cannot rebuild Story 2.2 reviewer disposition: {error}")
    else:
        if value != expected:
            failures.append("Story 2.2 reviewer disposition is stale or non-deterministic")
    return failures


def build_map(root: Path = ROOT) -> dict[str, Any]:
    protocol = read_json(root / PROTOCOL_PATH.relative_to(ROOT))
    disposition = read_json(root / DISPOSITION_PATH.relative_to(ROOT))
    failures = [*validate_protocol(protocol, root), *validate_disposition(disposition, root)]
    if failures:
        raise ValueError("; ".join(failures))
    return {
        "schema_version": 1,
        "task_id": "2.2.2.3",
        "status": "complete-evidence-map-blocked-macos",
        "requirements": [
            {
                "requirement_id": requirement_id,
                "product_requirement_status": "not-complete",
                **MAPPINGS[requirement_id],
            }
            for requirement_id in EXPECTED_REQUIREMENTS
        ],
        "reviewer_protocol": {
            "protocol_id": "RV-15",
            "foundation_status": protocol["foundation_execution_status"],
            "full_protocol_status": protocol["full_protocol_status"],
        },
        "reviewer_disposition": {
            "task_status": disposition["task_2_2_2_3_status"],
            "independent_review_status": disposition["independent_review_status"],
            "release_approval": disposition["release_approval"],
        },
        "artifacts": evidence_records(root, MAP_EVIDENCE_PATHS),
        "summary": {
            "mapped_requirement_count": 5,
            "product_requirements_complete": 0,
            "rv_15_foundation_complete": True,
            "rv_15_full_protocol_complete": False,
            "retained_artifact_count": len(MAP_EVIDENCE_PATHS),
            "retained_regression_count": len(REGRESSION_PATHS),
            "independent_review_complete": False,
            "story_gate_complete": False,
        },
        "macos": {
            "status": "blocked-macos",
            "evidence_substitution": "prohibited",
            "support_claim": "none",
        },
        "product_boundary_execution_claim": "none",
        "release_claim": "none",
    }


def validate_map(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["Story 2.2 security evidence map must be an object"]
    failures: list[str] = []
    if (
        value.get("schema_version") != 1
        or value.get("task_id") != "2.2.2.3"
        or value.get("status") != "complete-evidence-map-blocked-macos"
    ):
        failures.append("Story 2.2 security evidence map identity is invalid")
    requirements = value.get("requirements", [])
    requirement_ids = [item.get("requirement_id") for item in requirements]
    if requirement_ids != list(EXPECTED_REQUIREMENTS) or len(
        requirement_ids
    ) != len(set(requirement_ids)):
        failures.append("Story 2.2 security requirement closure is invalid")
    for item in requirements:
        if item.get("product_requirement_status") != "not-complete":
            failures.append(
                f"Story 2.2 overclaimed product requirement: {item.get('requirement_id')}"
            )
        for path in item.get("evidence", []):
            if not safe_relative_path(path) or path not in MAP_EVIDENCE_PATHS:
                failures.append(
                    f"Story 2.2 requirement evidence is invalid: {item.get('requirement_id')}"
                )
    artifacts = value.get("artifacts", [])
    paths = [item.get("path") for item in artifacts]
    if paths != list(MAP_EVIDENCE_PATHS) or len(paths) != len(set(paths)):
        failures.append("Story 2.2 retained evidence closure is invalid")
    for item in artifacts:
        path = item.get("path")
        if not safe_relative_path(path) or not (root / path).is_file():
            failures.append(f"Story 2.2 evidence path is invalid: {path}")
        elif item.get("sha256") != sha256_file(root / path) or item.get(
            "bytes"
        ) != (root / path).stat().st_size:
            failures.append(f"Story 2.2 evidence identity is invalid: {path}")
    summary = value.get("summary", {})
    if summary != {
        "mapped_requirement_count": 5,
        "product_requirements_complete": 0,
        "rv_15_foundation_complete": True,
        "rv_15_full_protocol_complete": False,
        "retained_artifact_count": len(MAP_EVIDENCE_PATHS),
        "retained_regression_count": 6,
        "independent_review_complete": False,
        "story_gate_complete": False,
    }:
        failures.append("Story 2.2 security evidence summary is invalid")
    if value.get("macos") != {
        "status": "blocked-macos",
        "evidence_substitution": "prohibited",
        "support_claim": "none",
    }:
        failures.append("Story 2.2 security map made an invalid macOS claim")
    if value.get("product_boundary_execution_claim") != "none" or value.get(
        "release_claim"
    ) != "none":
        failures.append("Story 2.2 security map made a product or release claim")
    try:
        expected = build_map(root)
    except (OSError, ValueError, KeyError, TypeError) as error:
        failures.append(f"cannot rebuild Story 2.2 security evidence map: {error}")
    else:
        if value != expected:
            failures.append("Story 2.2 security evidence map is stale or non-deterministic")
    return failures


def write_evidence(root: Path = ROOT) -> None:
    write_atomic(
        root / PROTOCOL_PATH.relative_to(ROOT), canonical_json(build_protocol(root))
    )
    write_atomic(
        root / DISPOSITION_PATH.relative_to(ROOT),
        canonical_json(build_disposition(root)),
    )
    write_atomic(root / MAP_PATH.relative_to(ROOT), canonical_json(build_map(root)))


def check_all(root: Path = ROOT) -> list[str]:
    failures = []
    for name, path, validator in (
        ("RV-15 control map", PROTOCOL_PATH, validate_protocol),
        ("reviewer disposition", DISPOSITION_PATH, validate_disposition),
        ("security evidence map", MAP_PATH, validate_map),
    ):
        try:
            value = read_json(root / path.relative_to(ROOT))
        except (OSError, json.JSONDecodeError) as error:
            failures.append(f"cannot read Story 2.2 {name}: {error}")
        else:
            failures.extend(validator(value, root))
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            write_evidence()
        failures = check_all()
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"Story 2.2 security evidence failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Story 2.2 security evidence failed: {failure}", file=sys.stderr)
        return 1
    print("Story 2.2 RV-15 security evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
