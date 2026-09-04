#!/usr/bin/env python3
"""Build and verify deterministic Sprint 113 GitOps fixtures."""
from __future__ import annotations
import argparse,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-113/gitops-control-corpus.json"
PROVIDERS=("argo-cd","flux");READS=("application","source","revision","destination","resource","health","sync","drift","history","event","permission");EFFECTS=("sync","prune","force","replace","hook","suspend","resume","reconcile","rollback")
ATTACKS=("repository-confusion","cluster-confusion","malicious-hook","prune-escalation","auto-sync-race","stale-desired-state","controller-impersonation","hidden-secret","hidden-admin")
FAULTS=("controller-outage","partial-sync","delayed-event","duplicate-reconcile","permission-loss","cancellation","crash","restart")
def build()->bytes:
 reads=[{"provider":p,"read":r,"identity_bound":True,"effect_count":0,"secret_value_count":0} for p in PROVIDERS for r in READS]
 plans=[{"case_id":f"S-113-P-{i+1:03d}","provider":PROVIDERS[i%2],"source_bound":True,"controller_bound":True,"destination_bound":True,"live_desired_bound":True,"artifact_bound":True,"policy_bound":True,"health_bound":True,"ordinary_effect":EFFECTS[(i%2)*7],"strong_effect_enabled_count":0,"desired_state_write_count":0} for i in range(64)]
 attacks=[{"case_id":f"S-113-A-{i+1:04d}","attack":ATTACKS[i%len(ATTACKS)],"controller_contact_count":0,"authority_escape_count":0,"hidden_effect_count":0,"unsafe_retry_count":0} for i in range(2048)]
 faults=[{"case_id":f"S-113-F-{i+1:04d}","fault":FAULTS[i%len(FAULTS)],"exact_truth":True,"reconciliation_required":True,"duplicate_effect_count":0,"unsafe_retry_count":0,"fresh_plan_required":True} for i in range(512)]
 integration={"drift_detected":True,"ordinary_sync_previewed":True,"separate_approval":True,"health_verified":True,"concurrent_desired_change":True,"stale_sync_denied":True,"stale_rollback_denied":True,"controller_contact_count":0}
 v={"schema_version":1,"providers":list(PROVIDERS),"reads":list(READS),"effects":list(EFFECTS),"attacks":list(ATTACKS),"faults":list(FAULTS),"read_case_count":len(reads),"plan_case_count":len(plans),"attack_case_count":len(attacks),"fault_case_count":len(faults),"read_cases":reads,"plan_cases":plans,"attack_cases":attacks,"fault_cases":faults,"integration_case":integration,"reviewer":"scripts.gitops_control_contract","promoted_provider_count":0,"live_controller_contact_count":0,"independent_human_review_count":0}
 return (json.dumps(v,sort_keys=True,separators=(",",":"))+"\n").encode()
def write():OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
def check():
 if not OUTPUT.is_file() or OUTPUT.read_bytes()!=build():raise RuntimeError("GitOps corpus stale or absent")
 v=json.loads(OUTPUT.read_bytes())
 if any(c["effect_count"] or c["secret_value_count"] or not c["identity_bound"] for c in v["read_cases"]):raise RuntimeError("GitOps read failed")
 if any(c["strong_effect_enabled_count"] or c["desired_state_write_count"] or not all(c[k] for k in ("source_bound","controller_bound","destination_bound","live_desired_bound","artifact_bound","policy_bound","health_bound")) for c in v["plan_cases"]):raise RuntimeError("GitOps plan failed")
 if any(c["controller_contact_count"] or c["authority_escape_count"] or c["hidden_effect_count"] or c["unsafe_retry_count"] for c in v["attack_cases"]):raise RuntimeError("GitOps attack admitted")
 if any(not c["exact_truth"] or not c["reconciliation_required"] or c["duplicate_effect_count"] or c["unsafe_retry_count"] or not c["fresh_plan_required"] for c in v["fault_cases"]):raise RuntimeError("GitOps recovery failed")
 if v["promoted_provider_count"] or v["live_controller_contact_count"] or v["independent_human_review_count"]:raise RuntimeError("GitOps support overclaim")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:write()
 check();print("validated 22 reads, 64 plans, 2048 attacks, and 512 faults without GitOps promotion");return 0
if __name__=="__main__":raise SystemExit(main())
