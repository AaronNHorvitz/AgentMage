#!/usr/bin/env python3
"""Validate Sprint 73 local GitHub triage artifacts."""
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
COUNTS={"object_cases":15,"triage_cases":12,"boundary_cases":12,"event_cases":10,"hostile_cases":11}
def validate():
    failures=[]; corpus=json.loads((ROOT/"docs/verification/sprint-73-github-triage-corpus.json").read_text()); deps=json.loads((ROOT/"docs/verification/sprint-73-github-triage-dependencies.json").read_text()); seen=set()
    for key,count in COUNTS.items():
        values=corpus.get(key,[])
        if len(values)!=count or len(values)!=len(set(values)): failures.append(f"{key} drifted")
        if seen.intersection(values): failures.append("case overlap")
        seen.update(values)
    for key in ("network_clients","webhook_listeners","polling_workers","provider_accounts","credential_values","publication_methods","mutation_operations"):
        if deps.get(key)!=[]: failures.append(f"unadmitted dependency: {key}")
    source=(ROOT/"capabilities/knowledge/src/github_triage.rs").read_text()
    for text in ("pub fn build_triage_item(","pub fn admit_hosted_event(","provider_state_changed: false","local_task_created: false"):
        if text not in source: failures.append(f"boundary absent: {text}")
    for text in ("reqwest","TcpStream","GITHUB_TOKEN","Command::new"):
        if text in source: failures.append(f"executor admitted: {text}")
    return failures
if __name__=="__main__":
    failures=validate()
    if failures: print("\n".join(failures)); raise SystemExit(1)
    print("validated 60 Sprint 73 GitHub triage cases")
