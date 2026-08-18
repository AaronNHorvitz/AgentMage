#!/usr/bin/env python3
"""Build and validate the Story 16.1 local product-security evidence map."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path
from typing import Any, Final

try:
    from scripts.evidence_core import atomic_write, canonical_json_bytes
except ModuleNotFoundError:
    from evidence_core import atomic_write, canonical_json_bytes

ROOT: Final = Path(__file__).resolve().parents[1]
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-16/security-evidence-map.json"
LOCAL_REPORT: Final = "artifacts/sprints/sprint-16/local-evidence-report.json"
WORKER_REPORT: Final = "artifacts/sprints/sprint-16/installed-linux-worker-matrix.json"
EXPECTED_REQUIREMENTS: Final = (
    "SR-PLT-003",
    "SR-PLT-004",
    "SR-ACC-001",
    "SR-ACC-002",
    "SR-ACC-003",
    "SR-ACC-004",
    "SR-ACC-005",
    "SR-ACC-006",
    "SR-AI-005",
    "SR-TST-002",
    "SR-TST-004",
    "SR-TST-006",
    "RV-03",
    "RV-04",
)
SOURCE_PATHS: Final = (
    "docs/verification/task-16-1-3-5-product-security-evidence.md",
    "SECURITY-REVIEW.md",
    "scripts/story_16_1_security_evidence.py",
    "tests/test_story_16_1_security_evidence.py",
    LOCAL_REPORT,
    WORKER_REPORT,
)
EVIDENCE = [LOCAL_REPORT, WORKER_REPORT]
CONTRIBUTIONS: Final = {
    "SR-PLT-003": ("blocked-macos", "Native App Sandbox and XPC evidence."),
    "SR-PLT-004": ("partial-linux-evidence", "Native bookmark and path parity."),
    "SR-ACC-001": ("demonstrated-story-scope", "Independent and macOS review."),
    "SR-ACC-002": ("partial-story-evidence", "Complete grant-field mutations."),
    "SR-ACC-003": ("demonstrated-story-scope", "Supported-platform review."),
    "SR-ACC-004": ("partial-story-evidence", "Full path corpus and macOS parity."),
    "SR-ACC-005": ("demonstrated-linux-scope", "Native macOS race campaign."),
    "SR-ACC-006": ("demonstrated-linux-scope", "Native macOS ambient campaign."),
    "SR-AI-005": ("partial-story-evidence", "Complete labeled injection corpus."),
    "SR-TST-002": ("deferred-manual-fuzzing", "Final manual fuzz campaign."),
    "SR-TST-004": ("demonstrated-linux-scope", "macOS and independent review."),
    "SR-TST-006": ("partial-story-evidence", "Cross-platform resource campaign."),
    "RV-03": ("demonstrated-linux-scope", "macOS and independent review."),
    "RV-04": ("demonstrated-linux-scope", "Native macOS path campaign."),
}
DOCUMENT_FRAGMENTS: Final = (
    "Mapped controls and review protocols: **14 of 14**.",
    "the required independent worker review and native macOS evidence are absent",
    "Manual fuzzing remains deliberately deferred",
    "Linux evidence is not substituted",
)
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")


class EvidenceError(ValueError):
    """Raised when retained security evidence cannot be admitted."""


def git_revision(candidate: str) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=30,
        check=False,
    )
    revision = result.stdout.strip()
    if result.returncode or REVISION.fullmatch(revision) is None:
        raise EvidenceError("source revision unavailable")
    return revision


def git_bytes(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=60,
        check=False,
    )
    if result.returncode or not result.stdout:
        raise EvidenceError(f"committed source unavailable: {path}")
    return result.stdout


def validate_sources(values: dict[str, str]) -> list[str]:
    failures: list[str] = []
    document = values[SOURCE_PATHS[0]]
    security_review = values["SECURITY-REVIEW.md"]
    for fragment in DOCUMENT_FRAGMENTS:
        if document.count(fragment) != 1:
            failures.append(f"Story 16.1 document fragment changed: {fragment}")
    for requirement in EXPECTED_REQUIREMENTS:
        if f"`{requirement}`" not in document:
            failures.append(f"Story 16.1 mapped requirement missing: {requirement}")
        if f"`{requirement}`" not in security_review:
            failures.append(f"Story 16.1 security requirement missing: {requirement}")

    try:
        local = json.loads(values[LOCAL_REPORT])
        worker = json.loads(values[WORKER_REPORT])
    except json.JSONDecodeError:
        return [*failures, "Story 16.1 retained evidence JSON invalid"]
    if (
        local.get("summary")
        != {"local_contract_passed": True, "release_approval": False, "sprint_status": "BLOCKED"}
        or local.get("verification_evidence", {}).get("independent_review") is not False
        or local.get("platform_evidence", {}).get("macos_xpc_worker") is not False
    ):
        failures.append("Story 16.1 local report scope changed")
    if (
        worker.get("status") != "pass-installed-linux-worker-operation-attack-lifecycle-matrix"
        or worker.get("linux_attack_matrix_complete") is not True
        or worker.get("linux_lifecycle_campaign_complete") is not True
        or worker.get("attack_matrix_complete") is not False
        or worker.get("cleanup_campaign_complete") is not False
        or worker.get("macos_evidence_substituted") is not False
        or worker.get("private_host_data_used") is not False
        or worker.get("repository_credentials_injected") is not False
        or worker.get("release_claim") is not False
    ):
        failures.append("Story 16.1 installed worker report scope changed")
    return failures


def source_records(revision: str) -> list[dict[str, Any]]:
    values: dict[str, str] = {}
    records: list[dict[str, Any]] = []
    for path in SOURCE_PATHS:
        data = git_bytes(revision, path)
        values[path] = data.decode()
        records.append(
            {"bytes": len(data), "path": path, "sha256": hashlib.sha256(data).hexdigest()}
        )
    if failures := validate_sources(values):
        raise EvidenceError("; ".join(failures))
    return records


def expected_mappings() -> list[dict[str, Any]]:
    return [
        {
            "contribution": CONTRIBUTIONS[requirement][0],
            "evidence": EVIDENCE,
            "remaining": CONTRIBUTIONS[requirement][1],
            "requirement_id": requirement,
        }
        for requirement in EXPECTED_REQUIREMENTS
    ]


def build_report(revision: str) -> dict[str, Any]:
    return {
        "artifact_id": "story-16-1-local-product-security-evidence-map",
        "external_network_used": False,
        "independent_review_complete": False,
        "limitations": [
            "Native macOS worker evidence and independent human review remain blockers.",
            "Manual fuzzing remains deferred; no release or supported-platform claim is made.",
        ],
        "mappings": expected_mappings(),
        "private_user_data_used": False,
        "requirement_ids": list(EXPECTED_REQUIREMENTS),
        "schema_version": 1,
        "source_revision": revision,
        "sources": source_records(revision),
        "status": "pass-local-mapping-with-external-blockers",
        "task_ids": ["16.1.3.5"],
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    expected = {
        "artifact_id": "story-16-1-local-product-security-evidence-map",
        "external_network_used": False,
        "independent_review_complete": False,
        "limitations": [
            "Native macOS worker evidence and independent human review remain blockers.",
            "Manual fuzzing remains deferred; no release or supported-platform claim is made.",
        ],
        "mappings": expected_mappings(),
        "private_user_data_used": False,
        "requirement_ids": list(EXPECTED_REQUIREMENTS),
        "schema_version": 1,
        "status": "pass-local-mapping-with-external-blockers",
        "task_ids": ["16.1.3.5"],
    }
    failures = [
        f"Story 16.1 security evidence {key} changed"
        for key, value in expected.items()
        if report.get(key) != value
    ]
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        failures.append("Story 16.1 security evidence revision invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(
        SOURCE_PATHS
    ):
        failures.append("Story 16.1 security evidence sources changed")
    elif any(
        not isinstance(item.get("bytes"), int)
        or item["bytes"] <= 0
        or SHA256.fullmatch(str(item.get("sha256", ""))) is None
        for item in sources
    ):
        failures.append("Story 16.1 security evidence source identity invalid")
    return failures


def validate_current(report: dict[str, Any]) -> list[str]:
    failures = validate_report(report)
    revision = str(report.get("source_revision", ""))
    if REVISION.fullmatch(revision) is None:
        return failures
    try:
        expected_sources = source_records(revision)
    except EvidenceError as error:
        failures.append(str(error))
    else:
        if report.get("sources") != expected_sources:
            failures.append("Story 16.1 security evidence source bytes changed")
    return failures


def load_report() -> dict[str, Any]:
    try:
        report = json.loads(REPORT_PATH.read_text())
    except (OSError, json.JSONDecodeError) as error:
        raise EvidenceError(f"evidence report unavailable: {error}") from error
    if not isinstance(report, dict):
        raise EvidenceError("evidence report root invalid")
    return report


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
        print(f"Story 16.1 security evidence: FAIL: {error}")
        return 1
    if failures:
        for failure in failures:
            print(f"Story 16.1 security evidence: FAIL: {failure}")
        return 1
    print("Story 16.1 security evidence: pass-local-with-external-blockers")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
