#!/usr/bin/env python3
"""Generate and validate source-bound local Story 23.7 evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-23/story-23.7"
RAW_PATH: Final = EVIDENCE_DIR / "verified-chat-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "verified-chat-report.json"
COMMANDS: Final = (
    ("npm", "--prefix", "shells/vscode", "test"),
    ("npm", "--prefix", "shells/vscode", "run", "lint"),
    ("npm", "--prefix", "shells/vscode", "run", "format:check"),
    (
        "cargo", "test", "-p", "agentmage-host", "engineering_runtime::tests",
        "--all-features", "--locked",
    ),
    (
        "cargo", "test", "-p", "agentmage-host", "protocol::tests",
        "--all-features", "--locked",
    ),
    (
        "python3", "-m", "pytest", "-q", "tests/test_architecture_decision.py",
        "tests/test_vscode_api_surfaces.py",
    ),
    ("python3", "scripts/validate_docs.py"),
)
SOURCES: Final = (
    "shells/vscode/package.json",
    "shells/vscode/src/verified_chat.ts",
    "shells/vscode/src/verified_chat_channel.ts",
    "shells/vscode/src/verified_chat_projection.ts",
    "shells/vscode/src/verified_chat_protocol.ts",
    "shells/vscode/src/provider.ts",
    "shells/vscode/test/verified_chat_channel.test.ts",
    "shells/vscode/test/verified_chat_projection.test.ts",
    "shells/vscode/test/verified_chat_protocol.test.ts",
    "kernel/contracts/src/engineering.rs",
    "kernel/engine/src/persistent_supervisor.rs",
    "kernel/engine/src/engineering_persistence.rs",
    "shells/host/src/engineering_runtime.rs",
    "docs/architecture/verified-chat-surface.md",
    "architecture/vscode-api-surfaces.json",
    "scripts/story_23_7_verified_chat_evidence.py",
    "tests/test_story_23_7_verified_chat_evidence.py",
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
        "record_type": "agentmage-story-23-7-verified-chat-evidence",
        "story_id": "23.7",
        "generated_on": "2026-08-31",
        "status": "PASS_LOCAL_DURABLE_VERIFIED_CHAT",
        "commands": [" ".join(command) for command in COMMANDS],
        "artifacts": [artifact(path) for path in SOURCES]
        + [artifact(RAW_PATH.relative_to(ROOT).as_posix())],
        "product_truth": {
            "csp_restricted_disposable_webview": True,
            "authenticated_ordered_command_channel": True,
            "rust_owned_durable_state_and_authority": True,
            "digest_verified_atomic_event_replay": True,
            "bounded_ordered_view_delivery": True,
            "cancellable_single_agent_presentation": True,
            "draft_and_approved_plan_reconstruction": True,
            "accessible_source_controls": True,
            "webview_policy_tool_or_completion_authority": False,
            "private_vscode_api": False,
            "hidden_reasoning_persisted_or_rendered": False,
            "installed_vsix_campaign_complete": False,
            "assistive_technology_audit_complete": False,
            "qualified_production_model_complete": False,
            "independent_review_complete": False,
            "windows_validation_complete": False,
            "macos_validation_complete": False,
            "release_claim": "none",
        },
        "remaining_external_work": [
            "run the packaged VSIX keyboard and assistive-technology campaign",
            "repeat with an independently qualified production model profile",
            "run final local Windows validation against the exact final candidate commit",
            "run deferred macOS validation against the exact final candidate commit",
            "obtain independent security, accessibility, and release review",
        ],
    }


def validate_report(value: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict):
        return ["Story 23.7 report is not an object"]
    truth = value.get("product_truth", {})
    required_true = (
        "csp_restricted_disposable_webview",
        "authenticated_ordered_command_channel",
        "rust_owned_durable_state_and_authority",
        "digest_verified_atomic_event_replay",
        "bounded_ordered_view_delivery",
        "cancellable_single_agent_presentation",
        "draft_and_approved_plan_reconstruction",
        "accessible_source_controls",
    )
    required_false = (
        "webview_policy_tool_or_completion_authority",
        "private_vscode_api",
        "hidden_reasoning_persisted_or_rendered",
        "installed_vsix_campaign_complete",
        "assistive_technology_audit_complete",
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
        failures.append("Story 23.7 cannot claim a release")
    return failures


def validate() -> list[str]:
    failures = [f"missing source: {path}" for path in SOURCES if not (ROOT / path).is_file()]
    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return failures + [f"cannot read retained Story 23.7 evidence: {error}"]
    for index in range(len(COMMANDS)):
        if f"COMMAND_{index}_EXIT=0" not in raw:
            failures.append(f"command {index} did not retain a passing exit")
    for marker in (
        "active run gate admits only cancellation",
        "durable approved Plan binding is reconstructed",
        "bounded delivery preserves order",
        "reload snapshot restores only closed session",
    ):
        if marker not in raw:
            failures.append(f"raw evidence lacks fixture marker: {marker}")
    for prohibited in ("test result: FAILED", "not ok", "BEGIN PRIVATE KEY"):
        if prohibited in raw:
            failures.append(f"raw evidence contains prohibited marker: {prohibited}")
    failures.extend(validate_report(report))
    if report != expected_report():
        failures.append("Story 23.7 report is stale or widened")
    return failures


def capture() -> int:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    RAW_PATH.write_text("Story 23.7 evidence capture in progress\n", encoding="utf-8")
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
    print("Story 23.7 local durable Verified Chat evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
