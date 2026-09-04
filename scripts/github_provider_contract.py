#!/usr/bin/env python3
"""Validate Sprint 71 GitHub provider boundary artifacts."""
from __future__ import annotations
import json
from pathlib import Path
from typing import Final
ROOT: Final=Path(__file__).resolve().parents[1]
CORPUS: Final=ROOT/"docs/verification/sprint-71-github-provider-corpus.json"
DEPENDENCIES: Final=ROOT/"docs/verification/sprint-71-github-dependency-manifest.json"
GROUPS: Final=("positive_cases","response_cases","isolation_cases","prohibited_cases")
COUNTS: Final={"positive_cases":14,"response_cases":12,"isolation_cases":20,"prohibited_cases":12}
def validate()->list[str]:
    failures=[]; corpus=json.loads(CORPUS.read_text()); deps=json.loads(DEPENDENCIES.read_text()); seen=set()
    for group in GROUPS:
        values=corpus.get(group,[])
        if len(values)!=COUNTS[group] or len(values)!=len(set(values)): failures.append(f"{group} count or uniqueness drifted")
        if seen.intersection(values): failures.append("case overlap")
        seen.update(values)
    for field in ("network_clients","dns_resolvers","git_clients","oauth_clients","credential_helpers","secret_store_clients","provider_accounts","mutation_operations"):
        if deps.get(field)!=[]: failures.append(f"unadmitted dependency: {field}")
    source=(ROOT/"capabilities/knowledge/src/github_provider.rs").read_text()
    for fragment in ("pub fn admit_github_authentication(","pub fn authorize_github_read(","pub fn record_github_read(","external_state_changed: false","GithubReadState::RateLimited"):
        if fragment not in source: failures.append(f"boundary absent: {fragment}")
    for forbidden in ("reqwest","TcpStream","Authorization: Bearer","GITHUB_TOKEN","Command::new"):
        if forbidden in source: failures.append(f"executor admitted: {forbidden}")
    return failures
if __name__=="__main__":
    problems=validate()
    if problems:
        print("\n".join(f"error: {item}" for item in problems)); raise SystemExit(1)
    print(f"validated {sum(COUNTS.values())} Sprint 71 GitHub provider cases")
