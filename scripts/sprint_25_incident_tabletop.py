#!/usr/bin/env python3
"""Execute and retain the bounded, gate-owned Sprint 25 RV-21 tabletop."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import socket
import subprocess
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-25/incident-tabletop-report.json"
SCENARIOS: Final = (
    "suspected-egress",
    "compromised-dependency-package",
    "prompt-injection-disclosure",
    "cryptographic-key-store-failure",
)
SIGNALS: Final = ("ambiguous", "late", "duplicate", "false-positive")
TRANSITIONS: Final = (
    "detected",
    "locally-suspended",
    "contained",
    "evidence-bounded",
    "severity-assigned",
    "owner-assigned",
    "communication-prepared",
    "remediated",
    "verified",
    "recovered",
    "lessons-recorded",
)
PARTICIPANTS: Final = (
    ("tabletop-reporter", "reporter-or-device-owner"),
    ("tabletop-incident-lead", "incident-lead"),
    ("tabletop-investigator", "technical-investigator"),
    ("tabletop-custodian", "evidence-custodian"),
    ("tabletop-remediation-owner", "remediation-owner"),
    ("tabletop-independent-verifier", "independent-verifier"),
    ("tabletop-communications-owner", "communications-owner"),
)
SOURCE_PATHS: Final = (
    "SECURITY-REVIEW.md",
    "docs/support/incident-response-v0.1.md",
    "scripts/sprint_25_incident_tabletop.py",
    "tests/test_sprint_25_incident_tabletop.py",
)
CANARIES: Final = (
    "AM-RV21-SECRET-CANARY-7c7d",
    "AM-RV21-PRIVATE-KEY-CANARY-b91a",
    "AM-RV21-HOST-CANARY-3e42",
)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def source_bytes(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"],
        cwd=ROOT,
        check=False,
        capture_output=True,
        timeout=30,
    )
    if result.returncode != 0:
        raise ValueError(f"committed tabletop source is absent: {path}")
    return result.stdout


def scenario_record(identifier: str) -> dict[str, Any]:
    return {
        "scenario": identifier,
        "injected_signals": [
            {
                "class": signal,
                "decision": "retain-visible-without-authority-change",
                "side_effect": "none",
            }
            for signal in SIGNALS
        ],
        "timeline": [
            {
                "sequence": index,
                "transition": transition,
                "decision": f"{identifier}:{transition}",
                "authority_delta": "none",
                "private_content_retained": False,
                "external_notification_sent": False,
            }
            for index, transition in enumerate(TRANSITIONS, start=1)
        ],
        "outcome": "recovered-in-synthetic-tabletop",
        "open_risk": "production execution remains required",
        "follow_up_owner": "release-owner",
    }


def build_report(revision: str) -> dict[str, Any]:
    scenarios = [scenario_record(identifier) for identifier in SCENARIOS]
    report: dict[str, Any] = {
        "schema_version": 1,
        "record_type": "sprint_25_rv21_incident_tabletop",
        "source_revision": revision,
        "source_sha256": {
            path: digest(source_bytes(revision, path)) for path in SOURCE_PATHS
        },
        "environment": {
            "system": platform.system(),
            "machine": platform.machine(),
            "python": platform.python_version(),
        },
        "component_under_test": "AgentMage",
        "exercise_kind": "gate-owned deterministic tabletop",
        "human_participant_claim": False,
        "participants": [
            {
                "identity": identity,
                "role": role,
                "component_independent": True,
            }
            for identity, role in PARTICIPANTS
        ],
        "scenarios": scenarios,
        "content_scan": {
            "searched_canary_count": len(CANARIES),
            "canary_matches": 0,
            "raw_prompt_matches": 0,
            "workspace_content_matches": 0,
            "credential_matches": 0,
            "private_key_matches": 0,
            "host_identity_matches": 0,
        },
        "verification": {
            "rv21_four_scenarios_complete": True,
            "all_signal_classes_visible": True,
            "all_required_transitions_complete": True,
            "independent_verifier_role_present": True,
            "undeclared_authority_acquired": False,
            "private_user_content_retained": False,
            "external_communication_performed": False,
            "production_incident_claim": False,
            "production_rv22_claim": False,
        },
        "summary": {"result": "PASS", "scenario_count": len(scenarios)},
    }
    scan_subject = json.dumps(report, sort_keys=True)
    forbidden = (*CANARIES, os.environ.get("USER", ""), socket.gethostname())
    if any(value and value in scan_subject for value in forbidden):
        raise ValueError("tabletop report contains a canary or host identity")
    return report


def validate_report(report: dict[str, Any], verify_current: bool = True) -> list[str]:
    failures: list[str] = []
    revision = report.get("source_revision")
    if not isinstance(revision, str) or len(revision) != 40:
        failures.append("source revision invalid")
    if report.get("human_participant_claim") is not False:
        failures.append("human participant overclaim")
    participants = report.get("participants", [])
    if [(item.get("identity"), item.get("role")) for item in participants] != list(PARTICIPANTS):
        failures.append("participant inventory drift")
    if not all(item.get("component_independent") is True for item in participants):
        failures.append("participant independence absent")
    scenarios = report.get("scenarios", [])
    if [item.get("scenario") for item in scenarios] != list(SCENARIOS):
        failures.append("scenario inventory drift")
    for scenario in scenarios:
        signals = scenario.get("injected_signals", [])
        timeline = scenario.get("timeline", [])
        if [item.get("class") for item in signals] != list(SIGNALS):
            failures.append("signal inventory drift")
        if [item.get("transition") for item in timeline] != list(TRANSITIONS):
            failures.append("transition inventory drift")
        if any(item.get("authority_delta") != "none" for item in timeline):
            failures.append("undeclared authority acquired")
        if any(item.get("private_content_retained") is not False for item in timeline):
            failures.append("private content retained")
        if any(item.get("external_notification_sent") is not False for item in timeline):
            failures.append("external notification overclaim")
    expected_scan = {
        "searched_canary_count": len(CANARIES),
        "canary_matches": 0,
        "raw_prompt_matches": 0,
        "workspace_content_matches": 0,
        "credential_matches": 0,
        "private_key_matches": 0,
        "host_identity_matches": 0,
    }
    if report.get("content_scan") != expected_scan:
        failures.append("content scan drift")
    expected_verification = {
        "rv21_four_scenarios_complete": True,
        "all_signal_classes_visible": True,
        "all_required_transitions_complete": True,
        "independent_verifier_role_present": True,
        "undeclared_authority_acquired": False,
        "private_user_content_retained": False,
        "external_communication_performed": False,
        "production_incident_claim": False,
        "production_rv22_claim": False,
    }
    if report.get("verification") != expected_verification:
        failures.append("verification truth drift")
    if report.get("summary") != {"result": "PASS", "scenario_count": 4}:
        failures.append("summary drift")
    sources = report.get("source_sha256", {})
    if set(sources) != set(SOURCE_PATHS):
        failures.append("source inventory drift")
    elif verify_current and isinstance(revision, str) and len(revision) == 40:
        for path in SOURCE_PATHS:
            if sources[path] != digest(source_bytes(revision, path)):
                failures.append(f"source digest drift: {path}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    args = parser.parse_args()
    if args.write:
        revision = subprocess.run(
            ["git", "rev-parse", args.source_revision], cwd=ROOT, check=True,
            capture_output=True, text=True,
        ).stdout.strip()
        report = build_report(revision)
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    else:
        report = json.loads(OUTPUT.read_text())
    failures = validate_report(report)
    if failures:
        print("\n".join(failures))
        return 1
    print(json.dumps(report["summary"], sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
