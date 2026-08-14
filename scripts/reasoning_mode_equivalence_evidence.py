#!/usr/bin/env python3
"""Build and validate S-012-IT01 reasoning-mode evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
from pathlib import Path
from typing import Any, Final

try:
    from scripts.evidence_core import atomic_write, canonical_json_bytes
except ModuleNotFoundError:
    from evidence_core import atomic_write, canonical_json_bytes

ROOT: Final = Path(__file__).resolve().parents[1]
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-12/story-12.1/reasoning-mode-equivalence.json"
SOURCE_PATHS: Final = (
    "docs/verification/s-012-it01-reasoning-mode-equivalence-results.md",
    "fixtures/runtime/v1/reasoning-mode-fixtures.json",
    "kernel/engine/src/s012_it01.rs",
    "kernel/engine/src/lib.rs",
    "scripts/reasoning_mode_equivalence_evidence.py",
    "tests/test_reasoning_mode_equivalence_evidence.py",
)
CASE_IDS: Final = tuple(f"RME-{index:02}" for index in range(1, 6))
DOCUMENT_FRAGMENTS: Final = (
    "Pass for concise/deep authority, evidence, budget, cadence, and receipt equivalence.",
    "two synthetic tasks",
    "same six budget records",
    "byte reproducible",
    "manual fuzzing",
)
SOURCE_FRAGMENTS: Final = {
    "fixtures/runtime/v1/reasoning-mode-fixtures.json": (
        '"mode_changes_authority": false',
        '"mode_changes_evidence_standard": false',
    ),
    "kernel/engine/src/s012_it01.rs": (
        "s_012_it01_reasoning_modes_change_capacity_only",
        "s_012_it01_fixed_tasks_have_identical_mode_receipts",
        "s_012_it01_receipts_are_byte_reproducible",
        "s_012_it01_budget_and_status_cadence_are_exact",
        "s_012_it01_fake_boundaries_are_inert_and_bounded",
    ),
    "kernel/engine/src/lib.rs": ("mod s012_it01;",),
}
COMMAND_SPECS: Final = (
    (("cargo", "test", "-p", "agentmage-kernel-engine", "s_012_it01", "--locked"), "5 passed; 0 failed"),
    (("cargo", "clippy", "-p", "agentmage-kernel-engine", "--all-targets", "--locked", "--", "-D", "warnings"), "Finished `dev` profile"),
    (("python3", "-m", "unittest", "tests.test_reasoning_mode_equivalence_evidence"), "Ran 4 tests"),
    (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
)
CLAIMS: Final = {
    "focused_case_count": 5,
    "rust_test_count": 5,
    "reasoning_mode_count": 2,
    "fixed_task_count": 2,
    "cross_mode_receipt_comparison_count": 2,
    "repeatability_comparison_count": 2,
    "budget_record_count_per_task": 6,
    "status_event_count_per_task": 6,
    "fake_model_calls_per_task": 1,
    "fake_tool_calls_per_task": 1,
    "production_model_calls_per_task": 0,
    "tool_state_changes": 0,
    "private_user_data_records": 0,
    "external_network_used": False,
    "manual_fuzzing_executed": False,
    "release_support": False,
}
LIMITATIONS: Final = [
    "The integration harness uses deterministic synthetic components and does not invoke a production model, tool worker, platform adapter, filesystem effect, or network request.",
    "Fake-model proposal count is test-harness activity; production model-call budget usage remains zero.",
    "The test proves fixed-task mode equivalence, not general model-output equivalence for arbitrary prompts.",
    "Persistence, restart recovery, live UI rendering, cross-platform execution, packaging, release acceptance, and manual fuzzing remain later gates.",
]
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class EvidenceError(ValueError):
    pass


def validate_sources(values: dict[str, str]) -> list[str]:
    failures = []
    document = values[SOURCE_PATHS[0]]
    if tuple(re.findall(r"`(RME-\d{2})`", document)) != CASE_IDS:
        failures.append("S-012-IT01 case closure changed")
    failures.extend(
        f"S-012-IT01 document fragment changed: {index}"
        for index, fragment in enumerate(DOCUMENT_FRAGMENTS, 1)
        if document.count(fragment) != 1
    )
    for path, fragments in SOURCE_FRAGMENTS.items():
        failures.extend(
            f"S-012-IT01 source fragment changed: {path}:{index}"
            for index, fragment in enumerate(fragments, 1)
            if values[path].count(fragment) != 1
        )
    return failures


def git_revision(candidate: str) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"], cwd=ROOT,
        capture_output=True, text=True, timeout=30, check=False,
    )
    revision = result.stdout.strip()
    if result.returncode or REVISION.fullmatch(revision) is None:
        raise EvidenceError("source revision unavailable")
    return revision


def git_bytes(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT,
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=60, check=False,
    )
    if result.returncode or not result.stdout:
        raise EvidenceError(f"committed source unavailable: {path}")
    return result.stdout


def command_record(arguments: tuple[str, ...], marker: str) -> dict[str, Any]:
    return {
        "command_id": hashlib.sha256("\0".join(arguments).encode()).hexdigest(),
        "expected_marker_sha256": hashlib.sha256(marker.encode()).hexdigest(),
        "exit_code": 0,
        "status": "pass",
    }


def expected_commands() -> list[dict[str, Any]]:
    return [command_record(arguments, marker) for arguments, marker in COMMAND_SPECS]


def run_checked(arguments: tuple[str, ...], marker: str) -> dict[str, Any]:
    result = subprocess.run(
        list(arguments), cwd=ROOT, capture_output=True, text=True, timeout=300,
        check=False, env={**os.environ, "LANG": "C", "LC_ALL": "C"},
    )
    if result.returncode or marker not in result.stdout + result.stderr:
        raise EvidenceError(f"verification failed: {arguments[0]}")
    return command_record(arguments, marker)


def source_records(revision: str) -> list[dict[str, Any]]:
    values = {}
    records = []
    for path in SOURCE_PATHS:
        data = git_bytes(revision, path)
        values[path] = data.decode()
        records.append({"bytes": len(data), "path": path, "sha256": hashlib.sha256(data).hexdigest()})
    if failures := validate_sources(values):
        raise EvidenceError("; ".join(failures))
    return records


def build_report(revision: str) -> dict[str, Any]:
    return {
        "artifact_id": "s-012-it01-reasoning-mode-equivalence",
        "case_ids": list(CASE_IDS),
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "source_revision": revision,
        "sources": source_records(revision),
        "status": "pass-reasoning-mode-equivalence",
        "task_ids": ["12.1.3.4", "S-012-IT01"],
        "verification_commands": [run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS],
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    exact = {
        "artifact_id": "s-012-it01-reasoning-mode-equivalence",
        "case_ids": list(CASE_IDS),
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "status": "pass-reasoning-mode-equivalence",
        "task_ids": ["12.1.3.4", "S-012-IT01"],
        "verification_commands": expected_commands(),
    }
    failures = [f"S-012-IT01 evidence {key} changed" for key, value in exact.items() if report.get(key) != value]
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        failures.append("S-012-IT01 evidence revision invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(SOURCE_PATHS):
        failures.append("S-012-IT01 evidence sources changed")
    elif any(not isinstance(item.get("bytes"), int) or item["bytes"] <= 0 or SHA256.fullmatch(str(item.get("sha256", ""))) is None for item in sources):
        failures.append("S-012-IT01 evidence source identity invalid")
    return failures


def validate_current(report: dict[str, Any]) -> list[str]:
    failures = validate_report(report)
    revision = report.get("source_revision")
    if REVISION.fullmatch(str(revision or "")) is None:
        return failures
    try:
        expected_sources = source_records(str(revision))
    except EvidenceError as error:
        failures.append(str(error))
    else:
        if report.get("sources") != expected_sources:
            failures.append("S-012-IT01 evidence source bytes changed")
    return failures


def load_report() -> dict[str, Any]:
    try:
        value = json.loads(REPORT_PATH.read_text())
    except (OSError, json.JSONDecodeError) as error:
        raise EvidenceError(f"evidence report unavailable: {error}") from error
    if not isinstance(value, dict):
        raise EvidenceError("evidence report root invalid")
    return value


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    args = parser.parse_args()
    try:
        if args.write:
            revision = git_revision(args.source_revision)
            atomic_write(REPORT_PATH, canonical_json_bytes(build_report(revision)))
        failures = validate_current(load_report())
    except EvidenceError as error:
        print(f"S-012-IT01 reasoning mode evidence: FAIL: {error}")
        return 1
    if failures:
        for failure in failures:
            print(f"S-012-IT01 reasoning mode evidence: FAIL: {failure}")
        return 1
    print("S-012-IT01 reasoning mode evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
