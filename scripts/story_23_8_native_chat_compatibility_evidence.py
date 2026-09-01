#!/usr/bin/env python3
"""Generate and validate source-bound local Story 23.8 evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-23/story-23.8"
RAW_PATH: Final = EVIDENCE_DIR / "native-chat-compatibility-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "native-chat-compatibility-report.json"
COMMANDS: Final = (
    ("npm", "--prefix", "shells/vscode", "test"),
    ("npm", "--prefix", "shells/vscode", "run", "lint"),
    ("npm", "--prefix", "shells/vscode", "run", "format:check"),
    (
        "cargo", "test", "-p", "agentmage-host", "runtime_parity_tests",
        "--all-features", "--locked",
    ),
    (
        "cargo", "test", "-p", "agentmage-host", "protocol::tests",
        "--all-features", "--locked",
    ),
    (
        "python3", "-m", "pytest", "-q", "tests/test_vscode_api_surfaces.py",
        "tests/test_architecture_decision.py",
    ),
    ("python3", "scripts/validate_docs.py"),
)
SOURCES: Final = (
    "shells/vscode/package.json",
    "shells/vscode/src/extension.ts",
    "shells/vscode/src/native_chat_compatibility.ts",
    "shells/vscode/src/participant_ingress.ts",
    "shells/vscode/src/provider.ts",
    "shells/vscode/src/runtime_transport.ts",
    "shells/vscode/test/native_chat_compatibility.test.ts",
    "shells/vscode/test/participant_ingress.test.ts",
    "architecture/vscode-api-surfaces.json",
    "scripts/vscode_api_surfaces.py",
    "tests/test_vscode_api_surfaces.py",
    "docs/architecture/native-chat-compatibility-disclosure.md",
    "docs/architecture/stable-chat-participant-ingress.md",
    "scripts/story_23_8_native_chat_compatibility_evidence.py",
    "tests/test_story_23_8_native_chat_compatibility_evidence.py",
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
        "record_type": "agentmage-story-23-8-native-chat-compatibility-evidence",
        "story_id": "23.8",
        "generated_on": "2026-08-31",
        "status": "PASS_LOCAL_NATIVE_CHAT_COMPATIBILITY",
        "commands": [" ".join(command) for command in COMMANDS],
        "artifacts": [artifact(path) for path in SOURCES]
        + [artifact(RAW_PATH.relative_to(ROOT).as_posix())],
        "product_truth": {
            "stable_participant_and_provider_registered": True,
            "all_supplied_references_required_and_accounted": True,
            "unresolved_reference_model_execution": False,
            "supported_history_and_part_boundaries_preserved": True,
            "unsupported_semantics_visible_before_execution": True,
            "strict_local_route_and_usage_disclosed": True,
            "external_tools_advertised_as_supported": False,
            "exact_token_usage_claimed": False,
            "verified_chat_transition_visible": True,
            "private_or_proposed_production_api": False,
            "native_agent_host_claim": False,
            "installed_vsix_version_campaign_complete": False,
            "qualified_production_model_complete": False,
            "independent_review_complete": False,
            "windows_validation_complete": False,
            "macos_validation_complete": False,
            "release_claim": "none",
        },
        "remaining_external_work": [
            "run the packaged VSIX across the declared stable VS Code version range",
            "run keyboard and assistive-technology interaction for both native surfaces",
            "repeat with an independently qualified production model profile",
            "run final local Windows validation against the exact final candidate commit",
            "run deferred macOS validation against the exact final candidate commit",
            "obtain independent compatibility, security, accessibility, and release review",
        ],
    }


def validate_report(value: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict):
        return ["Story 23.8 report is not an object"]
    truth = value.get("product_truth", {})
    required_true = (
        "stable_participant_and_provider_registered",
        "all_supplied_references_required_and_accounted",
        "supported_history_and_part_boundaries_preserved",
        "unsupported_semantics_visible_before_execution",
        "strict_local_route_and_usage_disclosed",
        "verified_chat_transition_visible",
    )
    required_false = (
        "unresolved_reference_model_execution",
        "external_tools_advertised_as_supported",
        "exact_token_usage_claimed",
        "private_or_proposed_production_api",
        "native_agent_host_claim",
        "installed_vsix_version_campaign_complete",
        "qualified_production_model_complete",
        "independent_review_complete",
        "windows_validation_complete",
        "macos_validation_complete",
    )
    failures.extend(
        f"{field} must remain true" for field in required_true if truth.get(field) is not True
    )
    failures.extend(
        f"{field} must remain false" for field in required_false if truth.get(field) is not False
    )
    if truth.get("release_claim") != "none":
        failures.append("Story 23.8 cannot claim a release")
    return failures


def validate() -> list[str]:
    failures = [f"missing source: {path}" for path in SOURCES if not (ROOT / path).is_file()]
    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return failures + [f"cannot read retained Story 23.8 evidence: {error}"]
    for index in range(len(COMMANDS)):
        if f"COMMAND_{index}_EXIT=0" not in raw:
            failures.append(f"command {index} did not retain a passing exit")
    for marker in (
        "provider preserves every supported role text part",
        "route and usage disclose exact available facts",
        "stale reference remains visible and stops before a model turn",
        "unsupported supplied reference stops instead of weakening context",
    ):
        if marker not in raw:
            failures.append(f"raw evidence lacks fixture marker: {marker}")
    for prohibited in ("test result: FAILED", "not ok", "BEGIN PRIVATE KEY"):
        if prohibited in raw:
            failures.append(f"raw evidence contains prohibited marker: {prohibited}")
    failures.extend(validate_report(report))
    if report != expected_report():
        failures.append("Story 23.8 report is stale or widened")
    return failures


def capture() -> int:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    RAW_PATH.write_text("Story 23.8 evidence capture in progress\n", encoding="utf-8")
    REPORT_PATH.write_text("{}\n", encoding="utf-8")
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
    print("Story 23.8 local native Chat compatibility evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
