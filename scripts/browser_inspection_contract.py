#!/usr/bin/env python3
"""Validate Sprint 83 sandboxed-browser contracts."""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "shells/host/src/browser_inspection.rs"
CORPUS = ROOT / "docs/verification/sprint-83-browser-inspection-corpus.json"
GUIDE = ROOT / "docs/guides/sandboxed-browser-inspection.md"
REQUIRED = (
    "BrowserInspectionAction", "Navigate", "Inspect", "Find", "Click", "Screenshot",
    "Download", "BrowserProfileClass", "PublicEphemeral", "AuthenticatedBrokered",
    "BrowserInspectionRequest", "BrowserActionPreview", "BrowserResultObservation",
    "plan_browser_inspection", "verify_browser_result", "action_grant_sha256",
    "redaction_required", "quarantine_required", "terminated", 'strip_prefix("https://")',
)
FORBIDDEN = ("std::net", "std::process", "std::fs", "TcpStream", "Command::new", "reqwest::")
EXPECTED = {"action": 6, "profile": 6, "domain": 6, "grant": 6, "result": 6, "cleanup": 6}


def validate() -> list[str]:
    failures: list[str] = []
    source = SOURCE.read_text(encoding="utf-8")
    production = source.split("#[cfg(test)]", 1)[0]
    corpus = json.loads(CORPUS.read_text(encoding="utf-8"))
    guide = GUIDE.read_text(encoding="utf-8")
    for token in REQUIRED:
        if token not in production:
            failures.append(f"browser inspection boundary absent: {token}")
    for token in FORBIDDEN:
        if token in production:
            failures.append(f"browser inspection contract acquired executor: {token}")
    cases = corpus.get("cases", [])
    if corpus.get("case_count") != 36 or len(cases) != 36 or len(set(cases)) != 36:
        failures.append("browser inspection corpus count drifted")
    if corpus.get("categories") != EXPECTED or sum(EXPECTED.values()) != 36:
        failures.append("browser inspection corpus categories drifted")
    if (corpus.get("local_contract_passed") is not True
            or corpus.get("native_browser_executed") is not False
            or corpus.get("download_scan_complete") is not False
            or corpus.get("privacy_review_present") is not False
            or corpus.get("substitution_set") != []):
        failures.append("browser inspection corpus truth state drifted")
    if "cookies, tokens, and credential values never enter" not in guide:
        failures.append("authenticated-profile privacy statement drifted")
    if source.count("fn sprint_83_") != 4:
        failures.append("Sprint 83 focused test inventory drifted")
    return failures


if __name__ == "__main__":
    errors = validate()
    if errors:
        print("\n".join(errors))
        raise SystemExit(1)
    print("validated 6 browser actions and 36 isolation cases")
