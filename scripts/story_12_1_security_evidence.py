#!/usr/bin/env python3
"""Build and validate the Story 12.1 product-security evidence map."""

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
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-12/story-12.1/security-evidence-map.json"
PREFIX: Final = "artifacts/sprints/sprint-12/story-12.1/"
EXPECTED_REQUIREMENTS: Final = (
    "SR-ACC-001", "SR-ACC-007",
    "SR-AI-003", "SR-AI-004", "SR-AI-005", "SR-AI-006", "SR-AI-007",
    "SR-AI-008", "SR-AI-009", "SR-AI-010", "SR-AI-011",
    "SR-OPS-001", "SR-TST-004", "SR-TST-005", "SR-TST-006",
)
EVIDENCE_PATHS: Final = tuple(
    PREFIX + name
    for name in (
        "agent-progress.json", "agent-runtime.json", "attachment-resolution.json",
        "cancellation-recovery.json", "deterministic-runtime.json",
        "hostile-model-candidates.json", "material-claims.json",
        "reasoning-mode-equivalence.json", "reasoning-verification.json",
        "runtime-fixtures.json", "runtime-schemas.json", "runtime-transcripts.json",
        "session-environment.json", "task-classification.json",
    )
)
SOURCE_PATHS: Final = (
    "docs/verification/task-12-1-3-5-product-security-evidence.md",
    "SECURITY-REVIEW.md",
    "scripts/story_12_1_security_evidence.py",
    "tests/test_story_12_1_security_evidence.py",
    *EVIDENCE_PATHS,
)


def paths(*names: str) -> list[str]:
    return [PREFIX + name for name in names]


MAPPINGS: Final = {
    "SR-ACC-001": {"contribution": "demonstrated-story-scope", "evidence": paths("agent-runtime.json", "task-classification.json", "hostile-model-candidates.json"), "remaining": "Grant-mediated production execution and release composition."},
    "SR-ACC-007": {"contribution": "demonstrated-story-scope", "evidence": paths("agent-runtime.json", "agent-progress.json", "runtime-transcripts.json"), "remaining": "Live user-decision UI and end-to-end handoff denial."},
    "SR-AI-003": {"contribution": "demonstrated-story-scope", "evidence": paths("task-classification.json", "reasoning-verification.json", "material-claims.json"), "remaining": "Production model and rendered-UI evaluation."},
    "SR-AI-004": {"contribution": "demonstrated-story-scope", "evidence": paths("agent-runtime.json", "hostile-model-candidates.json", "runtime-transcripts.json"), "remaining": "End-to-end consequential action matrix."},
    "SR-AI-005": {"contribution": "partial-story-evidence", "evidence": paths("hostile-model-candidates.json"), "remaining": "Complete direct and indirect injection corpus."},
    "SR-AI-006": {"contribution": "partial-story-evidence", "evidence": paths("runtime-fixtures.json", "hostile-model-candidates.json", "reasoning-mode-equivalence.json"), "remaining": "Production model and runtime reliability thresholds."},
    "SR-AI-007": {"contribution": "demonstrated-story-scope", "evidence": paths("reasoning-verification.json", "hostile-model-candidates.json", "runtime-transcripts.json"), "remaining": "Live UI rendering and broader semantic corpus."},
    "SR-AI-008": {"contribution": "partial-story-evidence", "evidence": paths("session-environment.json", "attachment-resolution.json", "material-claims.json"), "remaining": "End-to-end secret-canary and adjacent-workspace campaign."},
    "SR-AI-009": {"contribution": "partial-story-evidence", "evidence": paths("agent-runtime.json", "deterministic-runtime.json", "cancellation-recovery.json"), "remaining": "Complete OS process, memory, CPU or GPU, disk, and concurrency campaign."},
    "SR-AI-010": {"contribution": "partial-story-evidence", "evidence": paths("session-environment.json", "material-claims.json", "runtime-schemas.json"), "remaining": "Production inference provenance and durable audit integration."},
    "SR-AI-011": {"contribution": "partial-story-evidence", "evidence": paths("reasoning-mode-equivalence.json", "reasoning-verification.json", "material-claims.json"), "remaining": "Versioned factual and coding evaluation at release thresholds."},
    "SR-OPS-001": {"contribution": "partial-story-evidence", "evidence": paths("runtime-schemas.json", "agent-progress.json", "runtime-transcripts.json"), "remaining": "Complete durable product audit-event registry and store."},
    "SR-TST-004": {"contribution": "partial-story-evidence", "evidence": paths("hostile-model-candidates.json", "deterministic-runtime.json", "runtime-schemas.json"), "remaining": "Every input family across production boundaries."},
    "SR-TST-005": {"contribution": "partial-story-evidence", "evidence": paths("cancellation-recovery.json", "runtime-transcripts.json"), "remaining": "At least 100 durable crash-resume trials around every transition."},
    "SR-TST-006": {"contribution": "partial-story-evidence", "evidence": paths("deterministic-runtime.json", "cancellation-recovery.json"), "remaining": "Full platform resource-governance and responsiveness campaign."},
}
DOCUMENT_FRAGMENTS: Final = (
    "Pass for complete Story 12.1 security-control mapping with explicit remaining work.",
    "Mapped requirements: **15 of 15**.",
    "Transition coverage is retained",
    "Bounded model-repair traces are retained",
    "Cancellation proof is retained",
    "Truthful interruption and terminal transcripts are retained",
    "does not mark any requirement complete for a product release",
    "manual fuzzing",
)
COMMAND_SPECS: Final = (
    (("python3", "scripts/deterministic_runtime_evidence.py"), "S-012-UT01 deterministic runtime evidence: pass"),
    (("python3", "scripts/hostile_model_candidate_evidence.py"), "S-012-UT02 hostile model candidate evidence: pass"),
    (("python3", "scripts/cancellation_recovery_evidence.py"), "S-012-RT01 cancellation recovery evidence: pass"),
    (("python3", "scripts/runtime_transcript_evidence.py"), "runtime transcript evidence: pass"),
    (("python3", "scripts/reasoning_mode_equivalence_evidence.py"), "S-012-IT01 reasoning mode evidence: pass"),
    (("python3", "-m", "unittest", "tests.test_story_12_1_security_evidence"), "Ran 4 tests"),
    (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
)
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class EvidenceError(ValueError):
    pass


def git_revision(candidate: str) -> str:
    result = subprocess.run(["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"], cwd=ROOT, capture_output=True, text=True, timeout=30, check=False)
    revision = result.stdout.strip()
    if result.returncode or REVISION.fullmatch(revision) is None:
        raise EvidenceError("source revision unavailable")
    return revision


def git_bytes(revision: str, path: str) -> bytes:
    result = subprocess.run(["git", "show", f"{revision}:{path}"], cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=60, check=False)
    if result.returncode or not result.stdout:
        raise EvidenceError(f"committed source unavailable: {path}")
    return result.stdout


def validate_sources(values: dict[str, str]) -> list[str]:
    failures = []
    document = values[SOURCE_PATHS[0]]
    failures.extend(f"Story 12.1 document fragment changed: {index}" for index, fragment in enumerate(DOCUMENT_FRAGMENTS, 1) if document.count(fragment) != 1)
    review = values["SECURITY-REVIEW.md"]
    for requirement in EXPECTED_REQUIREMENTS:
        if f"`{requirement}`" not in review:
            failures.append(f"Story 12.1 security requirement missing: {requirement}")
        if f"`{requirement}`" not in document:
            failures.append(f"Story 12.1 mapped requirement missing: {requirement}")
    for path in EVIDENCE_PATHS:
        try:
            artifact = json.loads(values[path])
        except json.JSONDecodeError:
            failures.append(f"Story 12.1 evidence JSON invalid: {path}")
            continue
        if not str(artifact.get("status", "")).startswith("pass-"):
            failures.append(f"Story 12.1 retained evidence is not passing: {path}")
        if artifact.get("external_network_used") is not False or artifact.get("private_user_data_used") is not False:
            failures.append(f"Story 12.1 retained evidence scope changed: {path}")
    return failures


def command_record(arguments: tuple[str, ...], marker: str) -> dict[str, Any]:
    return {"command_id": hashlib.sha256("\0".join(arguments).encode()).hexdigest(), "expected_marker_sha256": hashlib.sha256(marker.encode()).hexdigest(), "exit_code": 0, "status": "pass"}


def expected_commands() -> list[dict[str, Any]]:
    return [command_record(arguments, marker) for arguments, marker in COMMAND_SPECS]


def run_checked(arguments: tuple[str, ...], marker: str) -> dict[str, Any]:
    result = subprocess.run(list(arguments), cwd=ROOT, capture_output=True, text=True, timeout=300, check=False, env={**os.environ, "LANG": "C", "LC_ALL": "C"})
    if result.returncode or marker not in result.stdout + result.stderr:
        raise EvidenceError(f"verification failed: {' '.join(arguments)}")
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
        "artifact_id": "story-12-1-product-security-evidence-map",
        "external_network_used": False,
        "limitations": [
            "The map closes Story 12.1 evidence organization and does not mark any requirement complete for a product release.",
            "Production model and tool execution, durable restart, live UI, cross-platform packaging, release acceptance, and manual fuzzing remain later gates.",
        ],
        "mappings": [{"requirement_id": requirement, **MAPPINGS[requirement]} for requirement in EXPECTED_REQUIREMENTS],
        "private_user_data_used": False,
        "requirement_ids": list(EXPECTED_REQUIREMENTS),
        "schema_version": 1,
        "source_revision": revision,
        "sources": source_records(revision),
        "status": "pass-story-scope-security-mapping",
        "task_ids": ["12.1.3.5"],
        "verification_commands": [run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS],
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    exact = {
        "artifact_id": "story-12-1-product-security-evidence-map",
        "external_network_used": False,
        "limitations": [
            "The map closes Story 12.1 evidence organization and does not mark any requirement complete for a product release.",
            "Production model and tool execution, durable restart, live UI, cross-platform packaging, release acceptance, and manual fuzzing remain later gates.",
        ],
        "mappings": [{"requirement_id": requirement, **MAPPINGS[requirement]} for requirement in EXPECTED_REQUIREMENTS],
        "private_user_data_used": False,
        "requirement_ids": list(EXPECTED_REQUIREMENTS),
        "schema_version": 1,
        "status": "pass-story-scope-security-mapping",
        "task_ids": ["12.1.3.5"],
        "verification_commands": expected_commands(),
    }
    failures = [f"Story 12.1 security evidence {key} changed" for key, value in exact.items() if report.get(key) != value]
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        failures.append("Story 12.1 security evidence revision invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(SOURCE_PATHS):
        failures.append("Story 12.1 security evidence sources changed")
    elif any(not isinstance(item.get("bytes"), int) or item["bytes"] <= 0 or SHA256.fullmatch(str(item.get("sha256", ""))) is None for item in sources):
        failures.append("Story 12.1 security evidence source identity invalid")
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
            failures.append("Story 12.1 security evidence source bytes changed")
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
        print(f"Story 12.1 security evidence: FAIL: {error}")
        return 1
    if failures:
        for failure in failures:
            print(f"Story 12.1 security evidence: FAIL: {failure}")
        return 1
    print("Story 12.1 security evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
