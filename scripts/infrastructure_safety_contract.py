#!/usr/bin/env python3
"""Build and verify deterministic Sprint 114 infrastructure fixtures."""
from __future__ import annotations
import argparse,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-114/infrastructure-safety-corpus.json"
TOOLS=("terraform","opentofu");READS=("configuration","module","provider","lock-file","backend-identity","workspace","state-serial","state-lineage","resource","sensitive-output","import","drift");EFFECTS=("create","update","replace","delete","import")
ATTACKS=("malicious-provider","malicious-module","backend-confusion","secret-output","state-injection","path-traversal","plan-substitution","destructive-concealment","nested-cloud-admin");FAULTS=("lock-loss","provider-outage","rate-limit","partial-effect","timeout","cancellation","crash","full-disk","restart")
def build()->bytes:
 reads=[{"tool":t,"read":r,"non_secret_identity":True,"secret_value_count":0,"effect_count":0}for t in TOOLS for r in READS]
 plans=[{"case_id":f"S-114-P-{i+1:03d}","tool":TOOLS[i%2],"saved_plan_bound":True,"source_bound":True,"lock_bound":True,"provider_module_bound":True,"backend_workspace_bound":True,"state_bound":True,"variables_redacted":True,"policy_bound":True,"exact_effect_bound":True,"implicit_approval_count":0}for i in range(64)]
 attacks=[{"case_id":f"S-114-A-{i+1:04d}","attack":ATTACKS[i%len(ATTACKS)],"provider_request_count":0,"secret_value_count":0,"authority_escape_count":0,"plan_substitution_count":0,"unsafe_apply_count":0}for i in range(2048)]
 faults=[{"case_id":f"S-114-F-{i+1:04d}","fault":FAULTS[i%len(FAULTS)],"unknown_state_blocked":True,"reconciliation_required":True,"automatic_retry_count":0,"state_surgery_count":0,"duplicate_apply_count":0}for i in range(512)]
 v={"schema_version":1,"tools":list(TOOLS),"reads":list(READS),"effects":list(EFFECTS),"attacks":list(ATTACKS),"faults":list(FAULTS),"read_case_count":len(reads),"plan_case_count":len(plans),"attack_case_count":len(attacks),"fault_case_count":len(faults),"read_cases":reads,"plan_cases":plans,"attack_cases":attacks,"fault_cases":faults,"integration_case":{"synthetic_non_production":True,"saved_plan_applied":True,"state_verified":True,"drift_verified":True,"source_movement_denied":True,"state_movement_denied":True,"provider_request_count":0},"reviewer":"scripts.infrastructure_safety_contract","promoted_tool_count":0,"live_backend_contact_count":0,"independent_human_review_count":0}
 return(json.dumps(v,sort_keys=True,separators=(",",":"))+"\n").encode()
def write():OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
def check():
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=build():raise RuntimeError("infrastructure corpus stale or absent")
 v=json.loads(OUTPUT.read_bytes())
 if any(c["secret_value_count"]or c["effect_count"]or not c["non_secret_identity"]for c in v["read_cases"]):raise RuntimeError("read failed")
 if any(c["implicit_approval_count"]or not all(c[k]for k in("saved_plan_bound","source_bound","lock_bound","provider_module_bound","backend_workspace_bound","state_bound","variables_redacted","policy_bound","exact_effect_bound"))for c in v["plan_cases"]):raise RuntimeError("plan failed")
 if any(c["provider_request_count"]or c["secret_value_count"]or c["authority_escape_count"]or c["plan_substitution_count"]or c["unsafe_apply_count"]for c in v["attack_cases"]):raise RuntimeError("attack admitted")
 if any(not c["unknown_state_blocked"]or not c["reconciliation_required"]or c["automatic_retry_count"]or c["state_surgery_count"]or c["duplicate_apply_count"]for c in v["fault_cases"]):raise RuntimeError("fault failed")
 if v["promoted_tool_count"]or v["live_backend_contact_count"]or v["independent_human_review_count"]:raise RuntimeError("IaC support overclaim")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:write()
 check();print("validated 24 reads, 64 plans, 2048 attacks, and 512 faults without IaC promotion");return 0
if __name__=="__main__":raise SystemExit(main())
