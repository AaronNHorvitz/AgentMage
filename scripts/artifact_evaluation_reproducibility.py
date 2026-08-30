#!/usr/bin/env python3
"""Regenerate the Story 2.3 evaluation corpus in two clean roots."""

from __future__ import annotations

import argparse
import copy
import json
import subprocess
import sys
import tempfile
from pathlib import Path, PurePosixPath
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.engineering_artifact_admission_fixtures import (  # noqa: E402
    ZERO_SHA256,
    canonical_json,
    sealed,
    sha256_bytes,
    write_atomic,
)


FIXTURE_DIR: Final = ROOT / "fixtures" / "artifact-evaluation" / "v1"
OUTPUT_PATH: Final = FIXTURE_DIR / "reproducibility-report.json"
GENERATOR_PATHS: Final = (
    "scripts/artifact_evaluation_text_log_fixtures.py",
    "scripts/artifact_evaluation_document_fixtures.py",
    "scripts/artifact_evaluation_lifecycle_scenarios.py",
    "scripts/artifact_evaluation_plan_fixtures.py",
    "scripts/artifact_evaluation_crash_fixtures.py",
    "scripts/artifact_evaluation_terminal_fixtures.py",
    "scripts/artifact_evaluation_artifact_metrics.py",
    "scripts/artifact_evaluation_workflow_metrics.py",
    "scripts/artifact_evaluation_golden_gate.py",
)
SUPPORT_PATHS: Final = (
    "scripts/document_fixture_generator.py",
    "scripts/engineering_artifact_admission_fixtures.py",
    "fixtures/artifact-admission/v1/context-accounting-manifests.json",
    "fixtures/artifact-admission/v1/resource-gate-observations.json",
)
SOURCE_PATHS: Final = SUPPORT_PATHS + GENERATOR_PATHS
OUTPUTS: Final = (
    ("fixtures/artifact-evaluation/v1/text-reference-corpus-v1.zip", ("fixture_identity",)),
    ("fixtures/artifact-evaluation/v1/text-reference-manifest.json", ("fixture_identity", "provenance_ledger")),
    ("fixtures/artifact-evaluation/v1/document-variant-corpus-v1.zip", ("fixture_identity",)),
    ("fixtures/artifact-evaluation/v1/document-variant-manifest.json", ("fixture_identity", "provenance_ledger")),
    ("fixtures/artifact-evaluation/v1/lifecycle-scenarios.json", ("summary", "provenance_ledger")),
    ("fixtures/artifact-evaluation/v1/workflow-plan-fixtures.json", ("summary",)),
    ("fixtures/artifact-evaluation/v1/workflow-crash-points.json", ("summary",)),
    ("fixtures/artifact-evaluation/v1/workflow-terminal-outcomes.json", ("summary",)),
    ("fixtures/artifact-evaluation/v1/artifact-golden-metrics.json", ("golden", "summary")),
    ("fixtures/artifact-evaluation/v1/workflow-golden-metrics.json", ("golden", "summary")),
    ("fixtures/artifact-evaluation/v1/golden-manifest-v1.json", ("golden", "provenance_ledger")),
)
CLEAN_ROOT_CONTRACT: Final = {
    "ambient_repository_files_copied": False,
    "ephemeral_empty_root_required": True,
    "generator_order_pinned": True,
    "isolated_python_mode": True,
    "network_access": False,
    "only_declared_sources_copied": True,
    "temporary_root_paths_recorded": False,
}
SIDE_EFFECT_CONTRACT: Final = {
    "external_effects": 0,
    "network_calls": 0,
    "product_effects": 0,
    "product_runtime_executions": 0,
    "writes_outside_ephemeral_roots": 0,
}


def safe_relative(value: str) -> bool:
    path = PurePosixPath(value)
    return bool(value) and not path.is_absolute() and ".." not in path.parts


def copy_source(relative: str, clean_root: Path) -> None:
    source = ROOT / relative
    destination = clean_root / relative
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(source.read_bytes())


def output_identities(clean_root: Path) -> list[dict[str, Any]]:
    identities = []
    for relative, roles in OUTPUTS:
        content = (clean_root / relative).read_bytes()
        identities.append(
            {
                "path": relative,
                "roles": list(roles),
                "byte_length": len(content),
                "sha256": sha256_bytes(content),
            }
        )
    return identities


def generate_in_clean_root(clean_root: Path) -> list[dict[str, Any]]:
    if any(clean_root.iterdir()):
        raise ValueError("clean generation root was not empty")
    for relative in SOURCE_PATHS:
        copy_source(relative, clean_root)
    for relative in GENERATOR_PATHS:
        completed = subprocess.run(
            [sys.executable, "-I", str(clean_root / relative), "--write"],
            cwd=clean_root,
            capture_output=True,
            text=True,
            timeout=120,
            check=False,
        )
        if completed.returncode != 0:
            diagnostic = (completed.stderr or completed.stdout).strip().splitlines()
            summary = diagnostic[-1] if diagnostic else "no diagnostic"
            raise RuntimeError(f"clean-root generator failed for {relative}: {summary}")
    return output_identities(clean_root)


def identity_set_sha256(identities: list[dict[str, Any]]) -> str:
    return sha256_bytes(canonical_json(identities))


def source_ledger() -> list[dict[str, Any]]:
    return [
        {
            "path": relative,
            "byte_length": (ROOT / relative).stat().st_size,
            "sha256": sha256_bytes((ROOT / relative).read_bytes()),
        }
        for relative in SOURCE_PATHS
    ]


def build_report() -> dict[str, Any]:
    with tempfile.TemporaryDirectory(prefix="agentmage-eval-repro-a-") as first_directory:
        with tempfile.TemporaryDirectory(prefix="agentmage-eval-repro-b-") as second_directory:
            first = generate_in_clean_root(Path(first_directory))
            second = generate_in_clean_root(Path(second_directory))
    if first != second:
        raise ValueError("two clean-root generations were not byte-identical")
    output_set_sha256 = identity_set_sha256(first)
    category_counts = {
        category: sum(category in item[1] for item in OUTPUTS)
        for category in ("fixture_identity", "golden", "summary", "provenance_ledger")
    }
    value = {
        "schema_version": 1,
        "report_id": "artifact-evaluation-clean-root-reproducibility-v1",
        "task_id": "2.3.4.1",
        "generated_on": "2026-08-30",
        "status": "clean_root_reproducibility_pass",
        "synthetic_only": True,
        "determinism_declared": True,
        "product_runtime_executed": False,
        "product_gate_claim": "none",
        "clean_root_contract": CLEAN_ROOT_CONTRACT,
        "clean_run_side_effect_contract": SIDE_EFFECT_CONTRACT,
        "source_count": len(SOURCE_PATHS),
        "source_provenance": source_ledger(),
        "generator_order": list(GENERATOR_PATHS),
        "run_count": 2,
        "runs": [
            {
                "run_id": run_id,
                "root_class": "ephemeral_empty_root",
                "output_count": len(first),
                "output_set_sha256": output_set_sha256,
            }
            for run_id in ("clean-root-a", "clean-root-b")
        ],
        "byte_identical": True,
        "output_count": len(first),
        "category_counts": category_counts,
        "outputs": first,
        "report_generator": {
            "path": "scripts/artifact_evaluation_reproducibility.py",
            "sha256": sha256_bytes(Path(__file__).read_bytes()),
        },
        "report_sha256": ZERO_SHA256,
    }
    return sealed(value, "report_sha256")


def valid_hash(record: dict[str, Any]) -> bool:
    unhashed = copy.deepcopy(record)
    recorded = unhashed.get("report_sha256")
    unhashed["report_sha256"] = ZERO_SHA256
    return recorded == sha256_bytes(canonical_json(unhashed))


def validate_report(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["reproducibility report must be an object"]
    failures: list[str] = []
    if not valid_hash(value):
        failures.append("reproducibility report self-hash is invalid")
    if value.get("clean_root_contract") != CLEAN_ROOT_CONTRACT:
        failures.append("clean-root isolation contract is incomplete or widened")
    if value.get("clean_run_side_effect_contract") != SIDE_EFFECT_CONTRACT:
        failures.append("reproducibility side-effect contract changed")
    expected_sources = source_ledger()
    if value.get("source_count") != len(expected_sources) or value.get("source_provenance") != expected_sources:
        failures.append("source provenance ledger is incomplete, stale, or reordered")
    if value.get("generator_order") != list(GENERATOR_PATHS):
        failures.append("clean-root generator order changed")
    outputs = value.get("outputs")
    expected_paths = [item[0] for item in OUTPUTS]
    expected_roles = [list(item[1]) for item in OUTPUTS]
    if not isinstance(outputs, list) or [item.get("path") for item in outputs if isinstance(item, dict)] != expected_paths:
        failures.append("reproducible output set is incomplete, extra, or reordered")
    elif [item.get("roles") for item in outputs] != expected_roles:
        failures.append("reproducible output roles are incomplete or changed")
    else:
        for item in outputs:
            relative = item["path"]
            checked = ROOT / relative
            if not checked.is_file() or item.get("byte_length") != checked.stat().st_size or item.get("sha256") != sha256_bytes(checked.read_bytes()):
                failures.append(f"checked output differs from clean-root identity: {relative}")
    if any(not safe_relative(path) for path in SOURCE_PATHS + tuple(expected_paths)):
        failures.append("reproducibility contract contains an unsafe path")
    output_set_sha256 = identity_set_sha256(outputs) if isinstance(outputs, list) else ""
    runs = value.get("runs")
    expected_runs = [
        {
            "run_id": run_id,
            "root_class": "ephemeral_empty_root",
            "output_count": len(OUTPUTS),
            "output_set_sha256": output_set_sha256,
        }
        for run_id in ("clean-root-a", "clean-root-b")
    ]
    if value.get("run_count") != 2 or runs != expected_runs or value.get("byte_identical") is not True:
        failures.append("two clean-root runs do not prove one byte-identical output set")
    expected_counts = {
        category: sum(category in item[1] for item in OUTPUTS)
        for category in ("fixture_identity", "golden", "summary", "provenance_ledger")
    }
    if value.get("output_count") != len(OUTPUTS) or value.get("category_counts") != expected_counts:
        failures.append("reproducibility output or category accounting is incomplete")
    if (
        value.get("status") != "clean_root_reproducibility_pass"
        or value.get("synthetic_only") is not True
        or value.get("determinism_declared") is not True
        or value.get("product_runtime_executed") is not False
        or value.get("product_gate_claim") != "none"
    ):
        failures.append("reproducibility report makes a product, runtime, or data-scope overclaim")
    return failures


def check() -> list[str]:
    try:
        actual = json.loads(OUTPUT_PATH.read_text(encoding="utf-8"))
        expected = build_report()
    except (OSError, ValueError, RuntimeError, json.JSONDecodeError, subprocess.SubprocessError) as error:
        return [f"cannot validate clean-root reproducibility: {error}"]
    failures = validate_report(actual)
    if actual != expected:
        failures.append("checked reproducibility report is stale, incomplete, or widened")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    if args.write:
        write_atomic(OUTPUT_PATH, canonical_json(build_report()))
    failures = check()
    if failures:
        for failure in failures:
            print(f"Clean-root reproducibility validation failed: {failure}", file=sys.stderr)
        return 1
    print(f"Validated {len(OUTPUTS)} byte-identical outputs across two clean roots")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
