#!/usr/bin/env python3
"""Build and validate S-012-I02 task-classification evidence."""

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
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-12/story-12.1/task-classification.json"
SOURCE_PATHS: Final = (
    "docs/verification/s-012-i02-task-classification-results.md",
    "kernel/engine/src/task_classification.rs",
    "kernel/engine/src/authority.rs",
    "kernel/engine/src/work_packet.rs",
    "kernel/engine/src/lib.rs",
    "scripts/task_classification_evidence.py",
    "tests/test_task_classification_evidence.py",
)
CASE_IDS: Final = tuple(f"TC-{index:02}" for index in range(1, 6))
DOCUMENT_FRAGMENTS: Final = (
    "Pass for deterministic descriptive classification",
    "Focused cases closed: **5 of 5**.",
    "Changing intent changes classification identity but never required authority or risk",
    "natural-language intent parsing is not claimed.",
    "Manual fuzzing remains deferred",
)
SOURCE_FRAGMENTS: Final = {
    "kernel/engine/src/task_classification.rs": (
        "pub enum TaskIntent {",
        "pub const ALL: [Self; 8]",
        "pub enum TaskComplexity {",
        "pub enum TaskRisk {",
        "pub struct TaskClassification {",
        "pub fn classify_task(",
        "const fn risk_for(authority: AuthorityClass, reversible: bool)",
        "This record carries no capability, grant, operation, completion, or execution method.",
    ),
    "kernel/engine/src/authority.rs": (
        "TaskClassification,",
        "impl Sealed for crate::task_classification::TaskClassification {}",
        "impl_non_authoritative!(TaskClassification => TaskClassification);",
    ),
    "kernel/engine/src/work_packet.rs": ("pub fn validate_packet(packet: &WorkPacket)",),
    "kernel/engine/src/lib.rs": ("pub mod task_classification;",),
}
COMMAND_SPECS: Final = (
    (("cargo", "test", "-p", "agentmage-kernel-engine", "task_classification::tests", "--locked"), "5 passed; 0 failed"),
    (("cargo", "clippy", "-p", "agentmage-kernel-engine", "--all-targets", "--locked", "--", "-D", "warnings"), "Finished `dev` profile"),
    (("python3", "-m", "unittest", "tests.test_task_classification_evidence"), "Ran 4 tests"),
    (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
)
CLAIMS: Final = {
    "focused_case_count": 5,
    "intent_class_count": 8,
    "complexity_class_count": 3,
    "risk_class_count": 4,
    "authority_class_count": 8,
    "risk_intent_cross_product_count": 72,
    "classification_authority_admitted_count": 0,
    "natural_language_intent_parsing_implemented": False,
    "semantic_classifier_implemented": False,
    "tool_or_model_executions": 0,
    "persistent_state_changed": False,
    "external_network_used": False,
    "manual_fuzzing_executed": False,
    "release_support": False,
}
LIMITATIONS: Final = [
    "Intent is an explicit descriptive label supplied with an already typed task; natural-language intent parsing is not claimed.",
    "Data sensitivity, model capability, advisory classifier restrictions, confidence/disagreement behavior, and Decision 0027 quantitative campaigns remain Story 12.2 work.",
    "No tool, model, worker, persistence, network, platform, package, or release workflow is exercised or claimed.",
    "Manual fuzzing remains deferred and is not represented by the deterministic tests.",
]
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class EvidenceError(ValueError):
    pass


def validate_sources(values: dict[str, str]) -> list[str]:
    failures = []
    document = values[SOURCE_PATHS[0]]
    if tuple(re.findall(r"`(TC-\d{2})`", document)) != CASE_IDS:
        failures.append("S-012-I02 case closure changed")
    failures.extend(
        f"S-012-I02 document fragment changed: {index}"
        for index, fragment in enumerate(DOCUMENT_FRAGMENTS, 1)
        if document.count(fragment) != 1
    )
    for path, fragments in SOURCE_FRAGMENTS.items():
        failures.extend(
            f"S-012-I02 source fragment changed: {path}:{index}"
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
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL, timeout=60, check=False,
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
        list(arguments), cwd=ROOT, capture_output=True, text=True, timeout=300, check=False,
        env={**os.environ, "LANG": "C", "LC_ALL": "C"},
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
        "artifact_id": "s-012-i02-task-classification",
        "case_ids": list(CASE_IDS),
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "source_revision": revision,
        "sources": source_records(revision),
        "status": "pass-deterministic-descriptive-classification",
        "task_ids": ["12.1.1.2", "S-012-I02"],
        "verification_commands": [run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS],
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    exact = {
        "artifact_id": "s-012-i02-task-classification",
        "case_ids": list(CASE_IDS),
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "status": "pass-deterministic-descriptive-classification",
        "task_ids": ["12.1.1.2", "S-012-I02"],
        "verification_commands": expected_commands(),
    }
    failures = [f"S-012-I02 evidence {key} changed" for key, value in exact.items() if report.get(key) != value]
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        failures.append("S-012-I02 evidence revision invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(SOURCE_PATHS):
        failures.append("S-012-I02 evidence sources changed")
    elif any(not isinstance(item.get("bytes"), int) or item["bytes"] <= 0 or SHA256.fullmatch(str(item.get("sha256", ""))) is None for item in sources):
        failures.append("S-012-I02 evidence source records invalid")
    return failures


def validate_committed(report: dict[str, Any]) -> None:
    for item in report["sources"]:
        if hashlib.sha256(git_bytes(report["source_revision"], item["path"])).hexdigest() != item["sha256"]:
            raise EvidenceError(f"committed source binding changed: {item['path']}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    revision = git_revision(arguments.source_revision)
    report = build_report(revision) if arguments.write else json.loads(REPORT_PATH.read_text())
    if arguments.write:
        atomic_write(REPORT_PATH, canonical_json_bytes(report))
    if failures := validate_report(report):
        raise EvidenceError("; ".join(failures))
    validate_committed(report)
    print("S-012-I02 task classification evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
