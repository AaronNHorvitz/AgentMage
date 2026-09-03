#!/usr/bin/env python3
"""Generate and validate source-bound local Story 23.5 evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-23/story-23.5"
RAW_PATH: Final = EVIDENCE_DIR / "participant-ingress-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "participant-ingress-report.json"
COMMANDS: Final = (
    ("npm", "--prefix", "shells/vscode", "test"),
    ("npm", "--prefix", "shells/vscode", "run", "lint"),
    ("npm", "--prefix", "shells/vscode", "run", "format:check"),
    ("cargo", "test", "-p", "agentmage-host", "protocol::tests", "--all-features", "--locked"),
    ("cargo", "test", "-p", "agentmage-host", "engineering_runtime::tests", "--all-features", "--locked"),
    ("python3", "-m", "pytest", "-q", "tests/test_architecture_decision.py", "tests/test_vscode_api_surfaces.py"),
    ("python3", "scripts/validate_docs.py"),
)
SOURCES: Final = (
    "shells/vscode/package.json",
    "shells/vscode/src/extension.ts",
    "shells/vscode/src/index.ts",
    "shells/vscode/src/participant_ingress.ts",
    "shells/vscode/src/verified_chat_protocol.ts",
    "shells/vscode/test/participant_ingress.test.ts",
    "shells/vscode/test/verified_chat_protocol.test.ts",
    "shells/host/src/protocol.rs",
    "shells/host/src/engineering_runtime.rs",
    "docs/architecture/stable-chat-participant-ingress.md",
    "architecture/language-build-matrix.json",
    "architecture/vscode-api-surfaces.json",
    "scripts/story_23_5_participant_ingress_evidence.py",
    "tests/test_story_23_5_participant_ingress_evidence.py",
)


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def artifact(relative: str) -> dict[str, Any]:
    path = ROOT / relative
    return {
        "path": relative,
        "byte_length": path.stat().st_size,
        "sha256": digest(path),
    }


def expected_report() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "agentmage-story-23-5-participant-ingress-evidence",
        "story_id": "23.5",
        "generated_on": "2026-08-31",
        "status": "PASS_LOCAL_STABLE_PARTICIPANT_INGRESS",
        "commands": [" ".join(command) for command in COMMANDS],
        "artifacts": [artifact(path) for path in SOURCES]
        + [artifact(RAW_PATH.relative_to(ROOT).as_posix())],
        "product_truth": {
            "stable_chat_participant_registered": True,
            "current_request_reference_resolution_only": True,
            "authenticated_rust_artifact_rpc": True,
            "complete_part_accounting": True,
            "ambient_workspace_enumeration": False,
            "typescript_parser_or_runtime_authority": False,
            "silent_provider_part_drop": False,
            "exact_model_revalidation_before_capture": True,
            "accessibility_source_contract": True,
            "installed_vsix_campaign_complete": False,
            "supported_platform_matrix_complete": False,
            "assistive_technology_review_complete": False,
            "qualified_production_model_complete": False,
            "independent_review_complete": False,
            "release_claim": "none",
        },
        "remaining_external_work": [
            "run packaged VSIX keyboard and supported screen-reader interaction",
            "run local and remote placement matrices on every declared platform",
            "repeat with an independently qualified production profile",
            "obtain independent security, accessibility, and release review",
        ],
    }


def validate_report(value: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict):
        return ["Story 23.5 report is not an object"]
    truth = value.get("product_truth", {})
    required_true = (
        "stable_chat_participant_registered",
        "current_request_reference_resolution_only",
        "authenticated_rust_artifact_rpc",
        "complete_part_accounting",
        "exact_model_revalidation_before_capture",
        "accessibility_source_contract",
    )
    required_false = (
        "ambient_workspace_enumeration",
        "typescript_parser_or_runtime_authority",
        "silent_provider_part_drop",
        "installed_vsix_campaign_complete",
        "supported_platform_matrix_complete",
        "assistive_technology_review_complete",
        "qualified_production_model_complete",
        "independent_review_complete",
    )
    failures.extend(f"{field} must remain true" for field in required_true if truth.get(field) is not True)
    failures.extend(f"{field} must remain false" for field in required_false if truth.get(field) is not False)
    if truth.get("release_claim") != "none":
        failures.append("Story 23.5 cannot claim a release")
    return failures


def validate() -> list[str]:
    failures = [f"missing source: {path}" for path in SOURCES if not (ROOT / path).is_file()]
    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return failures + [f"cannot read retained Story 23.5 evidence: {error}"]
    for index in range(len(COMMANDS)):
        if f"COMMAND_{index}_EXIT=0" not in raw:
            failures.append(f"command {index} did not retain a passing exit")
    for prohibited in ("COMMAND_0_EXIT=1", "test result: FAILED", "not ok", "BEGIN PRIVATE KEY"):
        if prohibited in raw:
            failures.append(f"raw evidence contains prohibited marker: {prohibited}")
    failures.extend(validate_report(report))
    if report != expected_report():
        failures.append("Story 23.5 report is stale or widened")
    return failures


def capture() -> int:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    records: list[str] = []
    for index, command in enumerate(COMMANDS):
        result = subprocess.run(
            command,
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
        )
        records.append(
            f"$ {' '.join(command)}\n{result.stdout}\nCOMMAND_{index}_EXIT={result.returncode}"
        )
        if result.returncode != 0:
            RAW_PATH.write_text("\n\n".join(records) + "\n", encoding="utf-8")
            return result.returncode
    RAW_PATH.write_text("\n\n".join(records) + "\n", encoding="utf-8")
    REPORT_PATH.write_text(
        json.dumps(expected_report(), indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    if args.write and capture() != 0:
        return 1
    failures = validate()
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    print("Story 23.5 local stable participant-ingress evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
