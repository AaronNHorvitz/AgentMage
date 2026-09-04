#!/usr/bin/env python3
"""Validate Sprint 72 hosted repository contracts."""
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def validate():
    failures=[]; corpus=json.loads((ROOT/"docs/verification/sprint-72-hosted-repository-corpus.json").read_text()); deps=json.loads((ROOT/"docs/verification/sprint-72-hosted-dependency-manifest.json").read_text()); counts={"discovery_cases":13,"source_cases":15,"governance_cases":23,"hostile_cases":13}; seen=set()
    for key,count in counts.items():
        values=corpus.get(key,[])
        if len(values)!=count or len(values)!=len(set(values)): failures.append(f"{key} drifted")
        if seen.intersection(values): failures.append("case overlap")
        seen.update(values)
    for key in ("network_clients","git_clients","archive_extractors","credential_values","mutation_operations","background_workers"):
        if deps.get(key)!=[]: failures.append(f"unadmitted dependency: {key}")
    source=(ROOT/"capabilities/knowledge/src/hosted_repository.rs").read_text()
    for text in ("pub fn immutable_source_link_sha256(","pub fn build_hosted_repository_view(","hosted_state_changed: false","local_state_changed: false"):
        if text not in source: failures.append(f"boundary absent: {text}")
    return failures
if __name__=="__main__":
    problems=validate()
    if problems: print("\n".join(problems)); raise SystemExit(1)
    print("validated 64 Sprint 72 hosted repository cases")
