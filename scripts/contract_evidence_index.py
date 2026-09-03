#!/usr/bin/env python3
"""Retain and validate the Task 1.2.5.2 contract evidence bundle."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts" / "sprints" / "sprint-1" / "story-1.2"
RAW_PATH: Final = EVIDENCE_DIR / "raw-contract-boundary-results.log"
SECURITY_MAP_PATH: Final = EVIDENCE_DIR / "security-evidence-map.json"
INDEX_PATH: Final = EVIDENCE_DIR / "contract-evidence-index.json"
PROTOCOLS: Final = ["RV-03", "RV-04", "RV-08", "RV-11", "RV-12", "RV-17"]
RAW_MARKERS: Final = [
    "Validated 37 Engineering Runtime schemas and 6 reused contracts.",
    "generated Engineering Runtime schemas are current, closed, and compile",
    "dependency rule validation passed",
    "Ran 8 tests",
    "runtime ownership validation passed",
    "Ran 15 tests",
    "vscode api surface validation passed",
    "Ran 13 tests",
    "status model validation passed",
    "Ran 29 tests",
    "Task 1.2.5.1 contract boundary gate passed locally",
]
GROUPS: Final = {
    "machine-readable-contract-indexes": [
        "architecture/engineering-runtime-change-manifest.json",
        "architecture/runtime-ownership.json",
        "architecture/vscode-api-surfaces.json",
        "architecture/status-model.json",
        "artifacts/sprints/sprint-1/story-1.2/contract-boundary-report.json",
    ],
    "dependency-dispositions": [
        "architecture/dependency-dispositions.json",
        "supply-chain/dependency-provenance.json",
        "supply-chain/sbom.cdx.json",
    ],
    "diagrams": [
        "docs/architecture/foundational-artifact-and-workflow-runtime.md",
        "docs/architecture/dependency-direction.md",
        "RUNTIME-BOUNDARIES.md",
    ],
    "migration-notes": [
        "architecture/schema-evolution-and-rollback.json",
        "architecture/rollback-design.json",
        "architecture/parser-ocr-platform-placement.json",
    ],
    "raw-local-results": [
        "artifacts/sprints/sprint-1/story-1.2/raw-contract-boundary-results.log"
    ],
    "security-mapping": [
        "SECURITY-REVIEW.md",
        "artifacts/sprints/sprint-1/story-1.2/security-evidence-map.json",
    ],
    "gate-implementation": [
        "scripts/contract_boundary_gate.py",
        "scripts/contract_evidence_index.py",
        "tests/test_contract_boundary_gate.py",
        "tests/test_contract_evidence_index.py",
    ],
}
TRUTH: Final = {
    "synthetic_data_only": True,
    "local_contract_evidence_retained": True,
    "protocol_complete": False,
    "native_platform_evidence": False,
    "external_review_complete": False,
    "platform_support_claimed": False,
    "release_readiness": False,
}


def _sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _artifact(path: str) -> dict[str, str]:
    return {"path": path, "sha256": _sha(ROOT / path)}


def _mapping(
    protocol_id: str,
    title: str,
    contribution: str,
    demonstrated: str,
    remaining: str,
    evidence: list[str],
) -> dict[str, Any]:
    return {
        "protocol_id": protocol_id,
        "title": title,
        "status": "partial-local-contract-evidence",
        "contribution": contribution,
        "demonstrated": demonstrated,
        "remaining": remaining,
        "evidence": [_artifact(path) for path in evidence],
        "protocol_complete": False,
    }


def expected_security_map() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "foundational-contract-security-evidence-map",
        "decision_id": "ADR-0042",
        "task_id": "1.2.5.2",
        "generated_on": "2026-08-29",
        "security_review": _artifact("SECURITY-REVIEW.md"),
        "protocols": [
            _mapping(
                "RV-03",
                "Sandbox and Ambient-Access Resistance",
                "placement-and-authority-contract-only",
                "Parser/OCR placement denies worker network, ambient workspace, credentials, host parsing, TypeScript fallback, and launch outside the platform adapter; mutation gates reject widening.",
                "Run native canary and escape campaigns for every admitted worker and platform; macOS requires its post-GA professional sandbox review. No native RV-03 pass is claimed here.",
                [
                    "architecture/parser-ocr-platform-placement.json",
                    "artifacts/sprints/sprint-1/story-1.2/contract-boundary-report.json",
                    "artifacts/sprints/sprint-1/story-1.2/raw-contract-boundary-results.log",
                ],
            ),
            _mapping(
                "RV-04",
                "Path and Race Safety",
                "cross-locus-path-contract-only",
                "The placement record forbids paths as authority or cross-locus reinterpretation and requires workspace-side remote hosts with digest-bound byte transfer.",
                "Run the required 500-fixture path, alias, link, normalization, rename, replacement, and mount-race campaign for each implemented ingestion path and native platform.",
                [
                    "architecture/parser-ocr-platform-placement.json",
                    "RUNTIME-BOUNDARIES.md",
                    "artifacts/sprints/sprint-1/story-1.2/contract-boundary-report.json",
                ],
            ),
            _mapping(
                "RV-08",
                "Data Minimization and Secret Leakage",
                "byte-crossing-and-diagnostic-contract-only",
                "Source bytes cannot enter arguments, environment, plaintext shared temporary files, unvalidated output, or default raw return; diagnostics and receipts remain bounded and content-free.",
                "Inject synthetic secrets through every implemented ingress, parser, OCR, cache, journal, diagnostic, export, backup, temporary, crash, and model-context surface and scan all retained locations.",
                [
                    "architecture/parser-ocr-platform-placement.json",
                    "architecture/schema-evolution-and-rollback.json",
                    "artifacts/sprints/sprint-1/story-1.2/contract-boundary-report.json",
                ],
            ),
            _mapping(
                "RV-11",
                "Prompt Injection and Authority Escalation",
                "ownership-and-absence-regression-only",
                "Dependency, ownership, API-surface, and capability-absence mutations cannot move parser content into policy, grant, effect, completion, TypeScript, or fallback authority.",
                "Run at least 200 labeled attacks against every newly implemented source, filename, parser warning, OCR result, tool result, model output, and user-content surface; this story does not claim that campaign complete.",
                [
                    "architecture/runtime-ownership.json",
                    "architecture/vscode-api-surfaces.json",
                    "artifacts/sprints/sprint-1/story-1.2/contract-boundary-report.json",
                    "artifacts/sprints/sprint-1/story-1.2/raw-contract-boundary-results.log",
                ],
            ),
            _mapping(
                "RV-12",
                "Grant Mutation and Replay",
                "schema-and-single-owner-regression-only",
                "Closed runtime schemas, dependency direction, singleton ownership, and current-truth gates preserve the existing grant, policy, dispatcher, and verifier authorities without adding a replay path.",
                "Run at least 500 integrated mutations including concurrent and crash-interrupted consumption across the implemented artifact/workflow paths and native platforms, retaining stable denials and audit events.",
                [
                    "architecture/engineering-runtime-change-manifest.json",
                    "architecture/runtime-ownership.json",
                    "artifacts/sprints/sprint-1/story-1.2/contract-boundary-report.json",
                    "artifacts/sprints/sprint-1/story-1.2/raw-contract-boundary-results.log",
                ],
            ),
            _mapping(
                "RV-17",
                "Crash Recovery and State Integrity",
                "migration-and-cancellation-plan-only",
                "The migration plan requires verified old-or-new generations, reconciliation before retry, no effect replay, no stale cache publication, and no later-user-data overwrite; worker cancellation requires reap before terminal state.",
                "After runtime implementation, inject crashes before and after every durable transition and during grant, worker, model, receipt, checkpoint, migration, and shutdown boundaries; retain the required 100-of-100 recovery results.",
                [
                    "architecture/schema-evolution-and-rollback.json",
                    "architecture/parser-ocr-platform-placement.json",
                    "artifacts/sprints/sprint-1/story-1.2/contract-boundary-report.json",
                ],
            ),
        ],
        "product_truth": dict(TRUTH),
    }


def expected_index() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "foundational-contract-evidence-index",
        "decision_id": "ADR-0042",
        "story_id": "1.2",
        "task_id": "1.2.5.2",
        "generated_on": "2026-08-29",
        "status": "complete-local-index-external-protocols-open",
        "groups": [
            {
                "id": group_id,
                "artifacts": [_artifact(path) for path in paths],
            }
            for group_id, paths in GROUPS.items()
        ],
        "raw_result": {
            "command": "npm run contract-boundary:check",
            "data_class": "synthetic-local-contract-tests-only",
            "artifact": _artifact(
                "artifacts/sprints/sprint-1/story-1.2/raw-contract-boundary-results.log"
            ),
            "required_markers": list(RAW_MARKERS),
        },
        "security_protocols": list(PROTOCOLS),
        "product_truth": dict(TRUTH),
    }


def render(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, ensure_ascii=True) + "\n"


def validate_raw(text: str) -> list[str]:
    failures: list[str] = []
    for marker in RAW_MARKERS:
        if marker not in text:
            failures.append(f"raw contract results missing marker: {marker}")
    for prohibited in ("Traceback (most recent call last)", "FAILED (", "not ok "):
        if prohibited in text:
            failures.append(f"raw contract results contain failure marker: {prohibited}")
    return failures


def validate_security_map(value: Any) -> list[str]:
    if value != expected_security_map():
        return ["security evidence map is stale, incomplete, reordered, or widened"]
    protocols = value.get("protocols", [])
    if [item.get("protocol_id") for item in protocols] != PROTOCOLS:
        return ["security evidence map protocol set changed"]
    if any(item.get("protocol_complete") is not False for item in protocols):
        return ["security evidence map falsely completes a protocol"]
    return []


def validate_index(value: Any) -> list[str]:
    if value != expected_index():
        return ["contract evidence index is stale, incomplete, reordered, or widened"]
    return []


def _capture_raw() -> tuple[str, int]:
    result = subprocess.run(
        ["npm", "run", "contract-boundary:check"],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        check=False,
    )
    return result.stdout, result.returncode


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    if args.write:
        raw, returncode = _capture_raw()
        if returncode != 0:
            sys.stderr.write(raw)
            print("contract evidence build failed: boundary command failed", file=sys.stderr)
            return 1
        failures = validate_raw(raw)
        if failures:
            for failure in failures:
                print(f"contract evidence build failed: {failure}", file=sys.stderr)
            return 1
        EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
        RAW_PATH.write_text(raw, encoding="utf-8")
        SECURITY_MAP_PATH.write_text(render(expected_security_map()), encoding="utf-8")
        INDEX_PATH.write_text(render(expected_index()), encoding="utf-8")
    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        security_map = json.loads(SECURITY_MAP_PATH.read_text(encoding="utf-8"))
        index = json.loads(INDEX_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(f"contract evidence validation failed: {error}", file=sys.stderr)
        return 1
    failures = [
        *validate_raw(raw),
        *validate_security_map(security_map),
        *validate_index(index),
    ]
    if failures:
        for failure in failures:
            print(f"contract evidence validation failed: {failure}", file=sys.stderr)
        return 1
    print("Task 1.2.5.2 contract evidence index and security mapping validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
