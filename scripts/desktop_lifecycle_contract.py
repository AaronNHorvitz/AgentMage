#!/usr/bin/env python3
"""Validate Sprint 77's source-level desktop lifecycle contract."""
import json
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
SOURCE=ROOT/"shells/host/src/desktop_lifecycle.rs"
CORPUS=ROOT/"docs/verification/sprint-77-desktop-lifecycle-corpus.json"
REQUIRED=("DesktopStatusSnapshot","DesktopWriterLease","DesktopRecoveryCheckpoint","ReadOnlySafeMode","DesktopPackageContract","DesktopLocalAssetKind","DesktopProtocolOperation","direct_file_access","direct_model_access","direct_key_access","direct_tool_access","hidden_network","unsafe_open","drag_drop_escape","clipboard_leakage","approval_bypass")
FORBIDDEN=("std::fs","std::process","TcpStream","reqwest","Command::new")
EXPECTED={"status":6,"recovery":10,"package":10,"protocol":12}

def validate():
    failures=[]
    source=SOURCE.read_text().split("#[cfg(test)]",1)[0]
    corpus=json.loads(CORPUS.read_text())
    for token in REQUIRED:
        if token not in source: failures.append(f"required desktop lifecycle boundary absent: {token}")
    for token in FORBIDDEN:
        if token in source: failures.append(f"desktop lifecycle executor admitted: {token}")
    cases=corpus.get("cases",[])
    if corpus.get("case_count")!=38 or len(cases)!=38 or len(set(cases))!=38: failures.append("desktop lifecycle corpus count drifted")
    if corpus.get("categories")!=EXPECTED or sum(EXPECTED.values())!=38: failures.append("desktop lifecycle categories drifted")
    if corpus.get("local_contract_passed") is not True or corpus.get("signed_package_present") is not False or corpus.get("native_accessibility_complete") is not False or corpus.get("platform_support") is not False or corpus.get("substitution_set")!=[]: failures.append("desktop lifecycle truth state drifted")
    return failures

if __name__=="__main__":
    failures=validate()
    if failures: print("\n".join(failures)); raise SystemExit(1)
    print("validated 38 desktop status, recovery, package, and protocol cases")
