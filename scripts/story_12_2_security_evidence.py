#!/usr/bin/env python3
"""Build and validate the Story 12.2 product-security evidence map."""

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
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-12/story-12.2/security-evidence-map.json"
PREFIX: Final = "artifacts/sprints/sprint-12/story-12.2/"
EXPECTED_IDENTIFIERS: Final = (
    "AT-AGENT-001",
    "AT-CLASS-001",
    "SR-ACC-001",
    "SR-AI-003",
    "SR-AI-005",
    "SR-AI-007",
    "SR-AI-015",
    "SR-AI-017",
    "SR-AI-018",
    "SR-OPS-001",
    "RV-17",
)
EVIDENCE_PATHS: Final = tuple(
    PREFIX + name
    for name in (
        "agent-contract-reference.json",
        "policy-reference.json",
        "agent-failure-corpus.json",
        "d027-s12-state.json",
        "d027-s12-policy.json",
        "d027-s12-classifier.json",
        "d027-s12-restart.json",
        "verifier-registry.json",
        "restart-reconciliation.json",
        "preclassification-policy.json",
        "advisory-policy.json",
        "classifier-failure.json",
        "reclassification.json",
    )
)
PROFILE_PATHS: Final = (
    "fixtures/agent-policy/v1/d027-policy-campaign.json",
    "fixtures/agent-policy/v1/d027-classifier-campaign.json",
    "fixtures/agent-policy/v1/d027-restart-campaign.json",
)
SOURCE_PATHS: Final = (
    "docs/verification/task-12-2-4-5-product-security-evidence.md",
    "SECURITY-REVIEW.md",
    "Agent-Scaffolding-Inventory.md",
    "kernel/engine/src/advisory_policy.rs",
    "scripts/story_12_2_security_evidence.py",
    "tests/test_story_12_2_security_evidence.py",
    *PROFILE_PATHS,
    *EVIDENCE_PATHS,
)


def paths(*names: str) -> list[str]:
    return [PREFIX + name for name in names]


MAPPINGS: Final = {
    "AT-AGENT-001": {
        "contribution": "pass-synthetic-story-scope",
        "evidence": paths("d027-s12-state.json", "agent-failure-corpus.json", "verifier-registry.json", "d027-s12-restart.json"),
        "remaining": "Complete agent-snapshot storage, production adapters, and release rerun.",
    },
    "AT-CLASS-001": {
        "contribution": "pass-synthetic-story-scope",
        "evidence": paths("d027-s12-policy.json", "d027-s12-classifier.json", "policy-reference.json", "reclassification.json"),
        "remaining": "Live fact collectors, production classifier, and release rerun.",
    },
    "SR-ACC-001": {
        "contribution": "demonstrated-story-scope",
        "evidence": paths("agent-contract-reference.json", "verifier-registry.json", "d027-s12-restart.json"),
        "remaining": "Complete production effect composition.",
    },
    "SR-AI-003": {
        "contribution": "demonstrated-story-scope",
        "evidence": paths("policy-reference.json", "d027-s12-state.json", "d027-s12-classifier.json"),
        "remaining": "Production model and rendered-UI evaluation.",
    },
    "SR-AI-005": {
        "contribution": "partial-story-evidence",
        "evidence": paths("d027-s12-classifier.json", "agent-failure-corpus.json"),
        "remaining": "Complete direct and indirect prompt-injection corpus.",
    },
    "SR-AI-007": {
        "contribution": "demonstrated-story-scope",
        "evidence": paths("classifier-failure.json", "agent-contract-reference.json", "agent-failure-corpus.json"),
        "remaining": "Live UI rendering and broader semantic corpus.",
    },
    "SR-AI-015": {
        "contribution": "partial-story-evidence",
        "evidence": paths("agent-contract-reference.json", "policy-reference.json", "d027-s12-classifier.json"),
        "remaining": "Full candidate-neutral codec and production runtime corpus.",
    },
    "SR-AI-017": {
        "contribution": "demonstrated-story-scope",
        "evidence": paths("d027-s12-policy.json", "d027-s12-classifier.json", "reclassification.json"),
        "remaining": "Live fact collectors and production classifier rerun.",
    },
    "SR-AI-018": {
        "contribution": "demonstrated-story-scope",
        "evidence": paths("d027-s12-state.json", "verifier-registry.json", "restart-reconciliation.json", "d027-s12-restart.json"),
        "remaining": "Complete agent-snapshot storage and product-loop integration.",
    },
    "SR-OPS-001": {
        "contribution": "partial-story-evidence",
        "evidence": paths("d027-s12-restart.json", "agent-failure-corpus.json"),
        "remaining": "Complete durable product audit-event registry.",
    },
    "RV-17": {
        "contribution": "demonstrated-agent-state-scope",
        "evidence": paths("d027-s12-state.json", "d027-s12-restart.json"),
        "remaining": "Remaining model, checkpoint, shutdown, and release recovery fixtures.",
    },
}
DOCUMENT_FRAGMENTS: Final = (
    "Pass for complete Story 12.2 acceptance-test execution and security-control mapping with explicit remaining release work.",
    "Mapped acceptance tests and controls: **11 of 11**.",
    "Raw policy seed and allocation are retained",
    "Transition coverage is retained",
    "Denial invariance is retained",
    "Classifier class traces and exact counts are retained",
    "Verifier evidence is retained",
    "Restart receipt and replay assertions are retained",
    "does not mark any security requirement complete for a product release",
    "manual fuzzing",
)
COMMAND_SPECS: Final = (
    (("python3", "scripts/d027_s12_state_evidence.py"), "Task 12.2.4.1 D027-S12-STATE evidence: pass"),
    (("python3", "scripts/d027_s12_policy_evidence.py"), "Task 12.2.4.2 D027-S12-POLICY evidence: pass"),
    (("python3", "scripts/d027_s12_classifier_evidence.py"), "Task 12.2.4.3 D027-S12-CLASSIFIER evidence: pass"),
    (("python3", "scripts/d027_s12_restart_evidence.py"), "Task 12.2.4.4 D027-S12-RESTART evidence: pass"),
    (("cargo", "test", "-p", "agentmage-kernel-engine", "d027_s12_classification_context_", "--locked"), "1 passed; 0 failed"),
    (("python3", "scripts/validate_agent_failure_corpus.py"), "Agent failure corpus: pass (6 cases)"),
    (("python3", "-m", "unittest", "tests.test_story_12_2_security_evidence"), "Ran 4 tests"),
    (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
)
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class EvidenceError(ValueError):
    pass


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
    failures = []
    document = values[SOURCE_PATHS[0]]
    failures.extend(
        f"Story 12.2 document fragment changed: {index}"
        for index, fragment in enumerate(DOCUMENT_FRAGMENTS, 1)
        if document.count(fragment) != 1
    )
    security = values["SECURITY-REVIEW.md"]
    inventory = values["Agent-Scaffolding-Inventory.md"]
    for identifier in EXPECTED_IDENTIFIERS:
        registry = inventory if identifier.startswith("AT-") else security
        if f"`{identifier}`" not in registry:
            failures.append(f"Story 12.2 registered identifier missing: {identifier}")
        if f"`{identifier}`" not in document:
            failures.append(f"Story 12.2 mapped identifier missing: {identifier}")
    for path in PROFILE_PATHS:
        try:
            profile = json.loads(values[path])
        except json.JSONDecodeError:
            failures.append(f"Story 12.2 profile JSON invalid: {path}")
            continue
        if profile.get("private_user_data") is not False or profile.get("external_network") is not False:
            failures.append(f"Story 12.2 profile scope changed: {path}")
    for path in EVIDENCE_PATHS:
        try:
            artifact = json.loads(values[path])
        except json.JSONDecodeError:
            failures.append(f"Story 12.2 evidence JSON invalid: {path}")
            continue
        if not str(artifact.get("status", "")).startswith("pass-"):
            failures.append(f"Story 12.2 retained evidence is not passing: {path}")
        if artifact.get("external_network_used") is not False or artifact.get("private_user_data_used") is not False:
            failures.append(f"Story 12.2 retained evidence scope changed: {path}")
    return failures


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
        list(arguments),
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=300,
        check=False,
        env={**os.environ, "LANG": "C", "LC_ALL": "C"},
    )
    if result.returncode or marker not in result.stdout + result.stderr:
        raise EvidenceError(f"verification failed: {' '.join(arguments)}")
    return command_record(arguments, marker)


def source_records(revision: str) -> list[dict[str, Any]]:
    values = {}
    records = []
    for path in SOURCE_PATHS:
        data = git_bytes(revision, path)
        values[path] = data.decode()
        records.append(
            {"bytes": len(data), "path": path, "sha256": hashlib.sha256(data).hexdigest()}
        )
    if failures := validate_sources(values):
        raise EvidenceError("; ".join(failures))
    return records


LIMITATIONS: Final = (
    "This map closes Story 12.2 evidence organization and acceptance execution; it does not mark any security requirement complete for a product release.",
    "Acceptance tests use deterministic synthetic records; production models, learned classifiers, platform workers, provider adapters, and live UI remain later integration and release gates.",
    "Complete agent-snapshot storage, full prompt-injection coverage, and the product audit-event registry remain later owning-sprint work.",
    "Cross-platform packaging, release acceptance, and manual fuzzing remain later gates.",
)


def build_report(revision: str) -> dict[str, Any]:
    return {
        "artifact_id": "story-12-2-product-security-evidence-map",
        "external_network_used": False,
        "limitations": list(LIMITATIONS),
        "mappings": [
            {"requirement_id": identifier, **MAPPINGS[identifier]}
            for identifier in EXPECTED_IDENTIFIERS
        ],
        "private_user_data_used": False,
        "requirement_ids": list(EXPECTED_IDENTIFIERS),
        "schema_version": 1,
        "source_revision": revision,
        "sources": source_records(revision),
        "status": "pass-story-scope-security-mapping",
        "task_ids": ["12.2.4.5"],
        "verification_commands": [
            run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS
        ],
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    exact = {
        "artifact_id": "story-12-2-product-security-evidence-map",
        "external_network_used": False,
        "limitations": list(LIMITATIONS),
        "mappings": [
            {"requirement_id": identifier, **MAPPINGS[identifier]}
            for identifier in EXPECTED_IDENTIFIERS
        ],
        "private_user_data_used": False,
        "requirement_ids": list(EXPECTED_IDENTIFIERS),
        "schema_version": 1,
        "status": "pass-story-scope-security-mapping",
        "task_ids": ["12.2.4.5"],
        "verification_commands": expected_commands(),
    }
    failures = [
        f"Story 12.2 security evidence {key} changed"
        for key, value in exact.items()
        if report.get(key) != value
    ]
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        failures.append("Story 12.2 security evidence revision invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(SOURCE_PATHS):
        failures.append("Story 12.2 security evidence sources changed")
    elif any(
        not isinstance(item.get("bytes"), int)
        or item["bytes"] <= 0
        or SHA256.fullmatch(str(item.get("sha256", ""))) is None
        for item in sources
    ):
        failures.append("Story 12.2 security evidence source identity invalid")
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
            failures.append("Story 12.2 security evidence source bytes changed")
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
        print(f"Story 12.2 security evidence: FAIL: {error}")
        return 1
    if failures:
        for failure in failures:
            print(f"Story 12.2 security evidence: FAIL: {failure}")
        return 1
    print("Story 12.2 security evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
