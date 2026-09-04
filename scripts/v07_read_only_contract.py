#!/usr/bin/env python3
"""Validate the truthful v0.7 local read-only aggregate."""
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
MODULES=("connected_connector.rs","github_provider.rs","hosted_repository.rs","github_triage.rs","pull_request_review.rs")
FORBIDDEN=("reqwest","TcpStream","Authorization: Bearer","GITHUB_TOKEN","Command::new")
def validate():
    failures=[]; bundle=json.loads((ROOT/"docs/verification/sprint-75-v0.7-acceptance-bundle.json").read_text())
    if bundle.get("source_sprints")!=[70,71,72,73,74] or bundle.get("release_status")!="BLOCKED" or bundle.get("native_acceptance_complete") is not False or bundle.get("release_approval") is not False or bundle.get("substitution_set")!=[]: failures.append("bundle truth state drifted")
    for field in ("network_request_count","hosted_mutation_operation_count","credential_material_count","strict_local_registration_change_count"):
        if bundle.get(field)!=0: failures.append(f"effect count drifted: {field}")
    for path in bundle.get("local_report_paths",[]):
        if not (ROOT/path).is_file(): failures.append(f"missing retained report: {path}")
    sources="\n".join((ROOT/"capabilities/knowledge/src"/name).read_text() for name in MODULES)
    for value in FORBIDDEN:
        if value in sources: failures.append(f"executor admitted: {value}")
    for required in ("external_state_changed: false","provider_state_changed: false","publication_enabled: false","strict_local_baseline_restored: true"):
        if required not in sources: failures.append(f"boundary absent: {required}")
    return failures
if __name__=="__main__":
    failures=validate()
    if failures: print("\n".join(failures)); raise SystemExit(1)
    print("validated Sprint 75 local v0.7 read-only aggregate")
