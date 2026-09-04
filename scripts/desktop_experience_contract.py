#!/usr/bin/env python3
"""Validate Sprint 76's bounded source-level desktop experience contract."""
import json
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
SOURCE=ROOT/"shells/host/src/desktop_experience.rs"
CORPUS=ROOT/"docs/verification/sprint-76-desktop-corpus.json"
REQUIRED=("DesktopArchitectureSelection","ConversationNavigationFilter","DesktopTranscriptItemKind","DesktopConversationAction","DesktopCheckpointComparison","DesktopWorkspaceSelection","DesktopApprovalScreen","effect_applied: false","obsidian_dependency: false","client_path_authority: false")
FORBIDDEN=("std::fs","std::process","TcpStream","reqwest","Command::new","effect_applied: true","external_web_service: true","client_authority: true")
EXPECTED_CATEGORIES={"architecture":4,"navigation":10,"transcript":5,"conversation_action":8,"checkpoint":5,"workspace":5,"approval":5}

def validate():
    failures=[]
    source=SOURCE.read_text()
    corpus=json.loads(CORPUS.read_text())
    for token in REQUIRED:
        if token not in source: failures.append(f"required desktop boundary absent: {token}")
    for token in FORBIDDEN:
        if token in source: failures.append(f"desktop authority admitted: {token}")
    cases=corpus.get("cases",[])
    if corpus.get("case_count")!=42 or len(cases)!=42 or len(set(cases))!=42: failures.append("desktop corpus count or identity drifted")
    if corpus.get("categories")!=EXPECTED_CATEGORIES or sum(EXPECTED_CATEGORIES.values())!=42: failures.append("desktop corpus categories drifted")
    if corpus.get("local_contract_passed") is not True or corpus.get("native_desktop_evidence") is not False or corpus.get("platform_support") is not False or corpus.get("substitution_set")!=[]: failures.append("desktop truth state drifted")
    return failures

if __name__=="__main__":
    failures=validate()
    if failures: print("\n".join(failures)); raise SystemExit(1)
    print("validated 42 desktop conversation and workspace contract cases")
