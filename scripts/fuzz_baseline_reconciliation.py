#!/usr/bin/env python3
"""Reconcile two clean deterministic runs of the fake fuzz boundary."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import sys
import tempfile
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.seeded_fuzz_failures import (  # noqa: E402
    SEED_CORPUS_PATH,
    TOOLCHAIN_POLICY_PATH,
    campaign,
)


REPORT_PATH = (
    ROOT
    / "artifacts/sprints/sprint-2/story-2.2/baseline-reconciliation-report.json"
)
BASE_INPUT_PATHS = (
    "Cargo.lock",
    "scripts/seeded_fuzz_failures.py",
    "fuzzing/target-registry.json",
    "fuzzing/toolchain-policy.json",
    "fuzzing/seeds/security-failures-v1.json",
    "schemas/testing/fuzz-result.schema.json",
    "artifacts/sprints/sprint-2/story-2.1/platform-result-recorder-report.json",
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
        prefix=".agentmage-fuzz-reconciliation-", dir=path.parent
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


def allowlisted_inputs(source_root: Path = ROOT) -> tuple[str, ...]:
    toolchain = read_json(
        source_root / TOOLCHAIN_POLICY_PATH.relative_to(ROOT)
    )
    dictionary_paths = tuple(
        item["path"] for item in toolchain.get("dictionaries", [])
    )
    paths = tuple(sorted((*BASE_INPUT_PATHS, *dictionary_paths)))
    if len(paths) != len(set(paths)):
        raise ValueError("clean fuzz baseline input allowlist contains duplicates")
    return paths


def inventory(root: Path, relative_paths: tuple[str, ...]) -> list[dict[str, Any]]:
    return [
        {
            "path": relative,
            "sha256": sha256_file(root / relative),
            "bytes": (root / relative).stat().st_size,
        }
        for relative in relative_paths
    ]


def verify_toolchain_inputs(clean_root: Path) -> None:
    toolchain = read_json(clean_root / TOOLCHAIN_POLICY_PATH.relative_to(ROOT))
    for item in toolchain["dictionaries"]:
        path = clean_root / item["path"]
        if sha256_file(path) != item["sha256"]:
            raise ValueError(f"clean fuzz dictionary hash mismatch: {item['path']}")
    seed = next(
        item
        for item in toolchain["seed_corpora"]
        if item["path"] == SEED_CORPUS_PATH.relative_to(ROOT).as_posix()
    )
    if sha256_file(clean_root / seed["path"]) != seed["sha256"]:
        raise ValueError("clean fuzz seed corpus hash mismatch")


def prepare_clean_root(
    destination: Path, source_root: Path = ROOT
) -> tuple[str, ...]:
    if not destination.is_dir() or any(destination.iterdir()):
        raise ValueError("clean fuzz destination must be an existing empty directory")
    relative_paths = allowlisted_inputs(source_root)
    for relative in relative_paths:
        source = source_root / relative
        if not source.is_file():
            raise ValueError(f"clean fuzz input is missing: {relative}")
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, target)
    copied = tuple(
        sorted(
            path.relative_to(destination).as_posix()
            for path in destination.rglob("*")
            if path.is_file()
        )
    )
    if copied != relative_paths:
        raise ValueError("clean fuzz root contains a non-allowlisted input")
    verify_toolchain_inputs(destination)
    return relative_paths


def normalized_result(result: dict[str, Any]) -> dict[str, Any]:
    return {
        "record_id": result["record_id"],
        "record_sha256": result["record_sha256"],
        "target_id": result["target"]["target_id"],
        "result_status": result["run"]["result_status"],
        "failure_class": result["failure"]["failure_class"],
        "crash_signature": result["failure"]["crash_signature"],
        "coverage_sha256": result["coverage"]["coverage_sha256"],
        "sanitizer_state_sha256": result["sanitizer"]["state_sha256"],
        "evidence_sha256": sha256_bytes(canonical_json(result["evidence"])),
        "timeout_resource_event": result["timeout_resource_event"],
        "minimized_sha256": result["minimized_reproducer"]["minimized_sha256"],
        "regression_path": result["minimized_reproducer"]["path"],
        "ownership": result["ownership"],
    }


def run_clean_baseline(source_root: Path, run_label: str) -> dict[str, Any]:
    with tempfile.TemporaryDirectory(prefix="agentmage-clean-fuzz-") as temporary:
        clean_root = Path(temporary)
        started_empty = not any(clean_root.iterdir())
        paths = prepare_clean_root(clean_root, source_root)
        before = inventory(clean_root, paths)
        report, regressions = campaign(clean_root)
        after = inventory(clean_root, paths)
        if before != after:
            raise ValueError("clean fuzz campaign mutated an allowlisted input")

        results = [normalized_result(item) for item in report["results"]]
        regression_inventory = [
            {
                "path": path,
                "sha256": sha256_bytes(content),
                "bytes": len(content),
            }
            for path, content in sorted(regressions.items())
        ]
        run_core = {
            "clean_root_started_empty": started_empty,
            "allowlisted_input_count": len(paths),
            "input_set_sha256": sha256_bytes(canonical_json(before)),
            "inputs_unchanged": before == after,
            "fake_boundary_engine": "agentmage-bounded-fake-fuzzer",
            "product_boundary_count": 0,
            "result_count": len(results),
            "non_pass_result_count": sum(
                item["result_status"] != "pass" for item in results
            ),
            "timeout_or_hang_result_count": sum(
                item["result_status"] in {"timeout", "hang"} for item in results
            ),
            "campaign_report_sha256": sha256_bytes(canonical_json(report)),
            "normalized_result_set_sha256": sha256_bytes(canonical_json(results)),
            "regression_set_sha256": sha256_bytes(
                canonical_json(regression_inventory)
            ),
            "results": results,
            "regressions": regression_inventory,
        }
        return {
            "run_label": run_label,
            **run_core,
            "reconciliation_sha256": sha256_bytes(canonical_json(run_core)),
        }


def comparable_run(run: dict[str, Any]) -> dict[str, Any]:
    return {
        key: value
        for key, value in run.items()
        if key not in {"run_label", "reconciliation_sha256"}
    }


def build_report(root: Path = ROOT) -> dict[str, Any]:
    runs = [
        run_clean_baseline(root, "clean-run-a"),
        run_clean_baseline(root, "clean-run-b"),
    ]
    first, second = runs
    first_results = first["results"]
    second_results = second["results"]
    comparison = {
        "input_sets_exact": first["input_set_sha256"] == second["input_set_sha256"],
        "campaign_reports_exact": first["campaign_report_sha256"]
        == second["campaign_report_sha256"],
        "result_sets_exact": first["normalized_result_set_sha256"]
        == second["normalized_result_set_sha256"],
        "regression_sets_exact": first["regression_set_sha256"]
        == second["regression_set_sha256"],
        "target_identities_exact": [item["target_id"] for item in first_results]
        == [item["target_id"] for item in second_results],
        "classifications_exact": [
            (item["result_status"], item["failure_class"])
            for item in first_results
        ]
        == [
            (item["result_status"], item["failure_class"])
            for item in second_results
        ],
        "coverage_fields_exact": [
            item["coverage_sha256"] for item in first_results
        ]
        == [item["coverage_sha256"] for item in second_results],
        "evidence_hashes_exact": [item["evidence_sha256"] for item in first_results]
        == [item["evidence_sha256"] for item in second_results],
        "all_non_pass_cases_preserved": all(
            item["result_status"] != "pass" for run in runs for item in run["results"]
        ),
        "timeout_cases_preserved_non_pass": all(
            item["result_status"] != "pass"
            for run in runs
            for item in run["results"]
            if item["timeout_resource_event"]["event"] == "run-timeout"
        ),
    }
    return {
        "schema_version": 1,
        "task_id": "2.2.2.2",
        "status": "pass-clean-fake-boundary-reconciliation",
        "execution_boundary": "bounded-synthetic-fake-boundary",
        "runs": runs,
        "comparison": comparison,
        "summary": {
            "clean_run_count": 2,
            "allowlisted_input_count_per_run": first["allowlisted_input_count"],
            "result_count_per_run": first["result_count"],
            "non_pass_result_count_per_run": first["non_pass_result_count"],
            "timeout_or_hang_result_count_per_run": first[
                "timeout_or_hang_result_count"
            ],
            "regression_count_per_run": len(first["regressions"]),
            "exact_comparison_count": sum(comparison.values()),
            "comparison_count": len(comparison),
        },
        "temporary_roots_retained": False,
        "raw_payloads_retained": False,
        "private_user_data_used": False,
        "ambient_environment_values_recorded": False,
        "network_used": False,
        "original_development_machine_required": False,
        "product_boundary_execution_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }


def validate_run(run: Any, expected_label: str) -> list[str]:
    if not isinstance(run, dict):
        return [f"clean fuzz {expected_label} must be an object"]
    failures: list[str] = []
    if run.get("run_label") != expected_label:
        failures.append(f"clean fuzz run label is invalid: {expected_label}")
    core = comparable_run(run)
    if run.get("reconciliation_sha256") != sha256_bytes(canonical_json(core)):
        failures.append(f"clean fuzz run hash is invalid: {expected_label}")
    if (
        run.get("clean_root_started_empty") is not True
        or run.get("inputs_unchanged") is not True
        or run.get("product_boundary_count") != 0
    ):
        failures.append(f"clean fuzz isolation is invalid: {expected_label}")
    results = run.get("results", [])
    if len(results) != 6 or len({item.get("record_id") for item in results}) != 6:
        failures.append(f"clean fuzz results were omitted or duplicated: {expected_label}")
    if any(
        item.get("result_status") == "pass"
        or item.get("result_status") != item.get("failure_class")
        for item in results
    ):
        failures.append(f"clean fuzz non-pass classification changed: {expected_label}")
    timeout_cases = [
        item
        for item in results
        if item.get("timeout_resource_event", {}).get("event") == "run-timeout"
    ]
    if len(timeout_cases) != 1 or any(
        item.get("result_status") == "pass" for item in timeout_cases
    ):
        failures.append(f"clean fuzz timeout became pass: {expected_label}")
    if len(run.get("regressions", [])) != 6:
        failures.append(f"clean fuzz regressions were omitted: {expected_label}")
    return failures


def validate_report(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["clean fuzz baseline reconciliation report must be an object"]
    failures: list[str] = []
    if (
        value.get("schema_version") != 1
        or value.get("task_id") != "2.2.2.2"
        or value.get("status") != "pass-clean-fake-boundary-reconciliation"
        or value.get("execution_boundary") != "bounded-synthetic-fake-boundary"
    ):
        failures.append("clean fuzz baseline report identity is invalid")
    runs = value.get("runs", [])
    if len(runs) != 2:
        failures.append("clean fuzz baseline must retain exactly two run summaries")
    else:
        failures.extend(validate_run(runs[0], "clean-run-a"))
        failures.extend(validate_run(runs[1], "clean-run-b"))
        if comparable_run(runs[0]) != comparable_run(runs[1]):
            failures.append("clean fuzz run summaries do not reconcile exactly")
    comparison = value.get("comparison", {})
    if len(comparison) != 10 or any(item is not True for item in comparison.values()):
        failures.append("clean fuzz exact reconciliation is incomplete")
    summary = value.get("summary", {})
    if summary != {
        "clean_run_count": 2,
        "allowlisted_input_count_per_run": 13,
        "result_count_per_run": 6,
        "non_pass_result_count_per_run": 6,
        "timeout_or_hang_result_count_per_run": 1,
        "regression_count_per_run": 6,
        "exact_comparison_count": 10,
        "comparison_count": 10,
    }:
        failures.append("clean fuzz baseline summary is invalid")
    if (
        value.get("temporary_roots_retained") is not False
        or value.get("raw_payloads_retained") is not False
        or value.get("private_user_data_used") is not False
        or value.get("ambient_environment_values_recorded") is not False
        or value.get("network_used") is not False
        or value.get("original_development_machine_required") is not False
    ):
        failures.append("clean fuzz baseline retained prohibited data or effects")
    if value.get("product_boundary_execution_claim") != "none":
        failures.append("clean fuzz baseline made a product execution claim")
    if value.get("macos_execution_status") != "blocked-macos" or value.get(
        "macos_support_claim"
    ) != "none":
        failures.append("clean fuzz baseline made an invalid macOS claim")
    try:
        expected = build_report(root)
    except (OSError, ValueError, KeyError, TypeError) as error:
        failures.append(f"cannot rebuild clean fuzz baseline report: {error}")
    else:
        if value != expected:
            failures.append("clean fuzz baseline report is stale or non-deterministic")
    return failures


def write_report(root: Path = ROOT) -> None:
    write_atomic(
        root / REPORT_PATH.relative_to(ROOT), canonical_json(build_report(root))
    )


def check_report(root: Path = ROOT) -> list[str]:
    path = root / REPORT_PATH.relative_to(ROOT)
    try:
        value = read_json(path)
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read clean fuzz baseline report: {error}"]
    return validate_report(value, root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            write_report()
        failures = check_report()
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"clean fuzz baseline reconciliation failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"clean fuzz baseline reconciliation failed: {failure}", file=sys.stderr)
        return 1
    print("Story 2.2 clean fake-boundary baseline reconciled")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
