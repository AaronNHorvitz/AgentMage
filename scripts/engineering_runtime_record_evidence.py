#!/usr/bin/env python3
"""Build and validate retained evidence for canonical Engineering Runtime records."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts" / "sprints" / "sprint-1" / "story-1.3"
RAW_PATH: Final = EVIDENCE_DIR / "canonical-record-gate-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "canonical-record-evidence-index.json"

COMMANDS: Final = [
    "npm run engineering-runtime:schemas:check",
    "npm run engineering-runtime:fixtures:check",
    "cargo test -p agentmage-kernel-contracts --test engineering_runtime_record_types --locked",
    "cargo test -p agentmage-kernel-engine --test engineering_runtime_record_corpus --locked",
    "cargo test -p agentmage-kernel-engine engineering_records::tests --locked",
    "python3 scripts/additions_only.py",
    "python3 scripts/dependency_rules.py",
]
MARKERS: Final = [
    "Validated 37 Engineering Runtime schemas and 6 reused contracts.",
    "Validated 57 deterministic Engineering Runtime fixture files.",
    "test result: ok. 3 passed; 0 failed",
    "test result: ok. 4 passed; 0 failed",
    "test result: ok. 8 passed; 0 failed",
    "Protected 294 requirement and 1407 checklist baseline entries.",
    "dependency rule validation passed",
]
SCHEMAS: Final = [
    f"schemas/engineering-runtime/{name}.schema.json"
    for name in (
        "artifact-envelope",
        "artifact-transformation",
        "artifact-ingestion-result",
        "context-manifest",
        "workflow-definition",
        "workflow-state",
        "tool-observation",
        "verification-result",
        "terminal-result",
    )
]
IMPLEMENTATIONS: Final = [
    "kernel/contracts/src/engineering_records.rs",
    "kernel/engine/src/engineering_records.rs",
    "scripts/engineering_runtime_schemas.mjs",
    "scripts/engineering_runtime_fixture_corpus.mjs",
    "scripts/engineering_runtime_record_evidence.py",
]
TESTS: Final = [
    "kernel/contracts/tests/engineering_runtime_record_types.rs",
    "kernel/engine/tests/engineering_runtime_record_corpus.rs",
    "tests/test_engineering_runtime_schemas.mjs",
    "tests/test_engineering_runtime_fixture_corpus.mjs",
    "tests/test_engineering_runtime_record_evidence.py",
]
TRUTH: Final = {
    "synthetic_data_only": True,
    "schema_gate_passed": True,
    "serialization_gate_passed": True,
    "additions_only_gate_passed": True,
    "dependency_direction_gate_passed": True,
    "mutation_gate_passed": True,
    "native_platform_execution_claimed": False,
    "external_review_claimed": False,
    "release_readiness_claimed": False,
}


def _sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _artifact(relative: str) -> dict[str, str]:
    return {"path": relative, "sha256": _sha(ROOT / relative)}


def fixture_paths() -> list[str]:
    base = ROOT / "fixtures" / "engineering-runtime" / "v2"
    return [path.relative_to(ROOT).as_posix() for path in sorted(base.rglob("*.json"))]


def expected_report() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "engineering-runtime-canonical-record-evidence-index",
        "decision_id": "ADR-0042",
        "story_id": "1.3",
        "task_id": "1.3.3.1",
        "generated_on": "2026-08-29",
        "status": "pass-local-contract-evidence",
        "commands": list(COMMANDS),
        "required_markers": list(MARKERS),
        "groups": [
            {"id": "schemas", "artifacts": [_artifact(path) for path in SCHEMAS]},
            {
                "id": "implementations",
                "artifacts": [_artifact(path) for path in IMPLEMENTATIONS],
            },
            {
                "id": "fixtures",
                "artifacts": [_artifact(path) for path in fixture_paths()],
            },
            {"id": "tests", "artifacts": [_artifact(path) for path in TESTS]},
            {"id": "raw-results", "artifacts": [_artifact(RAW_PATH.relative_to(ROOT).as_posix())]},
        ],
        "counts": {
            "canonical_schemas": len(SCHEMAS),
            "fixture_files": len(fixture_paths()),
            "fixture_cases": 56,
            "commands": len(COMMANDS),
        },
        "product_truth": dict(TRUTH),
    }


def render(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, ensure_ascii=True) + "\n"


def validate_raw(text: str) -> list[str]:
    failures = [f"raw results missing marker: {marker}" for marker in MARKERS if marker not in text]
    for prohibited in ("FAILED (", "test result: FAILED", "not ok ", "Traceback (most recent call last)"):
        if prohibited in text:
            failures.append(f"raw results contain failure marker: {prohibited}")
    return failures


def validate_report(value: Any) -> list[str]:
    if value != expected_report():
        return ["canonical record evidence index is stale, incomplete, reordered, or widened"]
    if value.get("product_truth") != TRUTH:
        return ["canonical record evidence truth was widened"]
    return []


def capture() -> tuple[str, int]:
    chunks: list[str] = []
    for command in COMMANDS:
        chunks.append(f"$ {command}\n")
        result = subprocess.run(
            command.split(),
            cwd=ROOT,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            check=False,
        )
        chunks.append(result.stdout)
        if not result.stdout.endswith("\n"):
            chunks.append("\n")
        if result.returncode != 0:
            return "".join(chunks), result.returncode
    return "".join(chunks), 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    if args.write:
        raw, returncode = capture()
        if returncode != 0:
            sys.stderr.write(raw)
            print("canonical record evidence build failed", file=sys.stderr)
            return 1
        EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
        RAW_PATH.write_text(raw, encoding="utf-8")
        failures = validate_raw(raw)
        if failures:
            for failure in failures:
                print(f"canonical record evidence build failed: {failure}", file=sys.stderr)
            return 1
        REPORT_PATH.write_text(render(expected_report()), encoding="utf-8")

    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(f"canonical record evidence validation failed: {error}", file=sys.stderr)
        return 1
    failures = validate_raw(raw) + validate_report(report)
    if failures:
        for failure in failures:
            print(f"canonical record evidence validation failed: {failure}", file=sys.stderr)
        return 1
    print("Task 1.3.3.1 canonical record evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
