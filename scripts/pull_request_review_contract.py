#!/usr/bin/env python3
"""Validate Sprint 74 pull-request review contracts."""
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]; COUNTS={"binding_cases":12,"finding_cases":15,"suppression_cases":6,"shadow_fix_cases":8,"hostile_cases":12}
def validate():
    failures=[]; corpus=json.loads((ROOT/"docs/verification/sprint-74-pr-review-corpus.json").read_text()); deps=json.loads((ROOT/"docs/verification/sprint-74-pr-review-dependencies.json").read_text()); seen=set()
    for key,count in COUNTS.items():
        values=corpus.get(key,[])
        if len(values)!=count or len(values)!=len(set(values)): failures.append(f"{key} drifted")
        if seen.intersection(values): failures.append("case overlap")
        seen.update(values)
    for key in ("network_clients","provider_accounts","fetch_operations","publication_methods","commit_operations","push_operations","merge_operations","release_operations"):
        if deps.get(key)!=[]: failures.append(f"unadmitted dependency: {key}")
    source=(ROOT/"capabilities/knowledge/src/pull_request_review.rs").read_text()
    for text in ("pub fn build_local_pull_request_review(","publication_enabled: false","push_enabled: false","release_enabled: false"):
        if text not in source: failures.append(f"boundary absent: {text}")
    return failures
if __name__=="__main__":
    failures=validate()
    if failures: print("\n".join(failures)); raise SystemExit(1)
    print("validated 53 Sprint 74 pull-request review cases")
