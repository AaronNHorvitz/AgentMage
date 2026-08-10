#!/usr/bin/env python3
"""Build and validate the AgentMage trust-boundary fuzz target registry."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import sys
import tempfile
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
REGISTRY_PATH = ROOT / "fuzzing/target-registry.json"
REPORT_PATH = ROOT / "artifacts/sprints/sprint-2/story-2.2/target-registry-report.json"
FIXTURE_SET_PATH = "fixtures/story-2.1/later-input-class-fixtures-v1.json"
BOUNDARIES = (
    ("FT-MANIFEST-001", "manifests", [3]),
    ("FT-CONFIG-001", "configuration", [3]),
    ("FT-IPC-001", "ipc-messages", [8, 9]),
    ("FT-MODEL-OUTPUT-001", "model-output", [13]),
    ("FT-GRANT-001", "capability-grants", [5]),
    ("FT-PATH-001", "paths", [6]),
    ("FT-TEXT-001", "text-encodings", [16]),
    ("FT-GIT-001", "git-objects", [17]),
    ("FT-REPOSITORY-PARSER-001", "repository-parsers", [18, 19]),
    ("FT-SQLITE-001", "sqlite-imports", [11]),
    ("FT-ARCHIVE-001", "archives", [33]),
    ("FT-FFI-001", "ffi-boundaries", [4]),
)
SCENARIOS = (
    "normal",
    "boundary",
    "malformed",
    "hostile",
    "oversized",
    "cancellation",
    "recovery",
)
SOURCE_ROOTS = ("kernel", "capabilities", "runtimes", "platforms", "shells")
NATIVE_SUFFIXES = {".c", ".cc", ".cpp", ".h", ".hpp", ".m", ".mm"}
FFI_PATTERN = re.compile(
    r'extern\s+"C"|#\s*\[\s*(?:no_mangle|link)|\bunsafe\s+(?:extern|fn|\{)'
)


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-fuzz-target-registry-", dir=path.parent
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


def discover_ffi_boundaries(root: Path = ROOT) -> list[dict[str, str]]:
    discovered = []
    for source_root in SOURCE_ROOTS:
        directory = root / source_root
        if not directory.exists():
            continue
        for path in sorted(item for item in directory.rglob("*") if item.is_file()):
            relative = path.relative_to(root).as_posix()
            if path.suffix.lower() in NATIVE_SUFFIXES:
                discovered.append({"path": relative, "reason": "native-source-file"})
                continue
            if path.suffix.lower() == ".rs":
                text = path.read_text(encoding="utf-8")
                if FFI_PATTERN.search(text):
                    discovered.append({"path": relative, "reason": "rust-ffi-token"})
    return discovered


def target_record(
    target_id: str, boundary_class: str, owner_sprints: list[int]
) -> dict[str, Any]:
    return {
        "target_id": target_id,
        "boundary_class": boundary_class,
        "status": "baseline-contract-registered",
        "first_execution_owner_sprints": owner_sprints,
        "seed_selection": {
            "fixture_set": FIXTURE_SET_PATH,
            "input_class": boundary_class,
            "required_scenarios": list(SCENARIOS),
        },
        "input_contract": {
            "accept_arbitrary_bytes": True,
            "maximum_materialized_input_bytes": 4096,
            "oversized_declared_input_bytes": 4097,
            "private_user_data_permitted": False,
            "real_credentials_permitted": False,
            "host_path_resolution_permitted": False,
        },
        "execution_contract": {
            "network_permitted": False,
            "external_process_permitted": False,
            "user_file_write_permitted": False,
            "ambient_authority_permitted": False,
            "pre_cancellation_required": True,
            "idempotent_cleanup_required": True,
        },
        "outcome_contract": {
            "panic_or_crash_is_non_pass": True,
            "timeout_is_non_pass": True,
            "resource_exhaustion_is_non_pass": True,
            "secret_exposure_is_non_pass": True,
            "authorization_bypass_is_non_pass": True,
            "path_escape_is_non_pass": True,
        },
        "activation_contract": {
            "fake_boundary_baseline_required": True,
            "concrete_harness_required_before_owner_gate": True,
            "versioned_seed_corpus_required": True,
            "result_record_required": True,
            "regression_path_required": True,
        },
        "product_boundary_implemented": False,
        "product_support_claim": "none",
    }


def build_registry(root: Path = ROOT) -> dict[str, Any]:
    fixtures = read_json(root / FIXTURE_SET_PATH)
    fixture_classes = fixtures.get("input_classes", [])
    boundary_classes = [boundary_class for _, boundary_class, _ in BOUNDARIES]
    if fixture_classes != boundary_classes:
        raise ValueError("fuzz target and fixture input classes do not align")
    ffi_boundaries = discover_ffi_boundaries(root)
    targets = [target_record(*boundary) for boundary in BOUNDARIES]
    value = {
        "schema_version": 1,
        "registry_id": "agentmage-fuzz-target-registry-v1",
        "registry_version": "1.0.0",
        "status": "baseline-contracts-registered",
        "targets": targets,
        "coverage": {
            "required_boundary_class_count": len(BOUNDARIES),
            "registered_target_count": len(targets),
            "missing_boundary_classes": [],
            "duplicate_boundary_classes": [],
            "complete": True,
        },
        "ffi_discovery": {
            "source_roots": list(SOURCE_ROOTS),
            "native_suffixes": sorted(NATIVE_SUFFIXES),
            "active_boundary_count": len(ffi_boundaries),
            "active_boundaries": ffi_boundaries,
            "unregistered_boundary_count": 0,
            "future_boundary_registration_required": True,
            "gate_on_unregistered_boundary": "block",
        },
        "future_story_gate_contract": {
            "registration_required": True,
            "execution_required_after_boundary_implementation": True,
            "missing_target_disposition": "block",
            "missing_corpus_disposition": "block",
            "missing_result_disposition": "block",
            "failed_or_timed_out_result_disposition": "block",
            "waiver_mechanism": "signed-dated-risk-decision-outside-normal-development-credentials",
        },
        "network_used": False,
        "private_user_data_used": False,
        "product_support_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }
    return {**value, "registry_sha256": sha256_bytes(canonical_json(value))}


def validate_registry(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["fuzz target registry must be an object"]
    failures: list[str] = []
    if (
        value.get("schema_version") != 1
        or value.get("registry_id") != "agentmage-fuzz-target-registry-v1"
        or value.get("registry_version") != "1.0.0"
    ):
        failures.append("fuzz target registry identity is invalid")
    targets = value.get("targets", [])
    ids = [item.get("target_id") for item in targets if isinstance(item, dict)]
    classes = [item.get("boundary_class") for item in targets if isinstance(item, dict)]
    if ids != [item[0] for item in BOUNDARIES] or len(ids) != len(set(ids)):
        failures.append("fuzz target identity closure is invalid")
    if classes != [item[1] for item in BOUNDARIES] or len(classes) != len(set(classes)):
        failures.append("fuzz target boundary closure is invalid")
    for item in targets:
        if not isinstance(item, dict):
            failures.append("fuzz target contract is not an object")
            continue
        if item.get("status") != "baseline-contract-registered":
            failures.append(f"fuzz target is not registered: {item.get('target_id')}")
        selection = item.get("seed_selection", {})
        if selection.get("required_scenarios") != list(SCENARIOS):
            failures.append(f"fuzz target fixture scenarios are incomplete: {item.get('target_id')}")
        input_contract = item.get("input_contract", {})
        if (
            input_contract.get("maximum_materialized_input_bytes") != 4096
            or input_contract.get("private_user_data_permitted") is not False
            or input_contract.get("host_path_resolution_permitted") is not False
        ):
            failures.append(f"fuzz target input contract is unsafe: {item.get('target_id')}")
        execution = item.get("execution_contract", {})
        if any(
            execution.get(field) is not False
            for field in (
                "network_permitted",
                "external_process_permitted",
                "user_file_write_permitted",
                "ambient_authority_permitted",
            )
        ):
            failures.append(f"fuzz target execution contract is unsafe: {item.get('target_id')}")
        if item.get("product_boundary_implemented") is not False or item.get(
            "product_support_claim"
        ) != "none":
            failures.append(f"fuzz target made a product claim: {item.get('target_id')}")
    ffi = value.get("ffi_discovery", {})
    discovered = discover_ffi_boundaries(root)
    if (
        ffi.get("active_boundary_count") != len(discovered)
        or ffi.get("active_boundaries") != discovered
        or ffi.get("unregistered_boundary_count") != 0
        or ffi.get("gate_on_unregistered_boundary") != "block"
    ):
        failures.append("fuzz FFI discovery closure is invalid")
    gate = value.get("future_story_gate_contract", {})
    if any(
        gate.get(field) != "block"
        for field in (
            "missing_target_disposition",
            "missing_corpus_disposition",
            "missing_result_disposition",
            "failed_or_timed_out_result_disposition",
        )
    ):
        failures.append("fuzz future-story gate does not fail closed")
    if value.get("network_used") is not False or value.get(
        "private_user_data_used"
    ) is not False:
        failures.append("fuzz target registry used network or private data")
    if value.get("product_support_claim") != "none":
        failures.append("fuzz target registry made a product claim")
    if value.get("macos_execution_status") != "blocked-macos" or value.get(
        "macos_support_claim"
    ) != "none":
        failures.append("fuzz target registry made an invalid macOS claim")
    try:
        expected = build_registry(root)
    except (OSError, ValueError, KeyError, TypeError) as error:
        failures.append(f"cannot rebuild fuzz target registry: {error}")
    else:
        if value != expected:
            failures.append("fuzz target registry is stale or non-deterministic")
    return failures


def build_report(root: Path = ROOT) -> dict[str, Any]:
    registry_path = root / REGISTRY_PATH.relative_to(ROOT)
    registry = read_json(registry_path)
    failures = validate_registry(registry, root)
    if failures:
        raise ValueError("; ".join(failures))
    return {
        "schema_version": 1,
        "task_id": "2.2.1.1",
        "status": "pass-baseline-contract-registration",
        "registry": {
            "path": REGISTRY_PATH.relative_to(ROOT).as_posix(),
            "sha256": sha256_bytes(registry_path.read_bytes()),
            "self_sha256": registry["registry_sha256"],
            "version": registry["registry_version"],
        },
        "coverage": registry["coverage"],
        "ffi_discovery": registry["ffi_discovery"],
        "fixture_set": FIXTURE_SET_PATH,
        "future_story_gate_contract": registry["future_story_gate_contract"],
        "network_calls": 0,
        "private_user_data_used": False,
        "product_boundary_execution_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }


def validate_report(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["fuzz target registry report must be an object"]
    failures: list[str] = []
    if (
        value.get("schema_version") != 1
        or value.get("task_id") != "2.2.1.1"
        or value.get("status") != "pass-baseline-contract-registration"
    ):
        failures.append("fuzz target registry report identity is invalid")
    coverage = value.get("coverage", {})
    if coverage != {
        "required_boundary_class_count": 12,
        "registered_target_count": 12,
        "missing_boundary_classes": [],
        "duplicate_boundary_classes": [],
        "complete": True,
    }:
        failures.append("fuzz target registry report coverage is invalid")
    if value.get("network_calls") != 0 or value.get("private_user_data_used") is not False:
        failures.append("fuzz target registry report used network or private data")
    if value.get("product_boundary_execution_claim") != "none":
        failures.append("fuzz target registry report made a product execution claim")
    if value.get("macos_execution_status") != "blocked-macos" or value.get(
        "macos_support_claim"
    ) != "none":
        failures.append("fuzz target registry report made an invalid macOS claim")
    try:
        expected = build_report(root)
    except (OSError, ValueError, KeyError, TypeError) as error:
        failures.append(f"cannot rebuild fuzz target registry report: {error}")
    else:
        if value != expected:
            failures.append("fuzz target registry report is stale or non-deterministic")
    return failures


def write_artifacts(root: Path = ROOT) -> None:
    registry_path = root / REGISTRY_PATH.relative_to(ROOT)
    write_atomic(registry_path, canonical_json(build_registry(root)))
    write_atomic(root / REPORT_PATH.relative_to(ROOT), canonical_json(build_report(root)))


def check_artifacts(root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    try:
        registry = read_json(root / REGISTRY_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        failures.append(f"cannot read fuzz target registry: {error}")
    else:
        failures.extend(validate_registry(registry, root))
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        failures.append(f"cannot read fuzz target registry report: {error}")
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
        print(f"fuzz target registry failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"fuzz target registry failed: {failure}", file=sys.stderr)
        return 1
    print("Story 2.2 fuzz target registry validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
