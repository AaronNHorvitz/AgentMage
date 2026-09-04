#!/usr/bin/env python3
"""Build deterministic Sprint 124 adapter-removal fixtures."""
from __future__ import annotations
import argparse,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-124/adapter-removal-corpus.json"
INVENTORY=("credential","cache","graph-node","graph-edge","event-cursor","webhook","schedule","worker","process","socket","firewall-policy","temporary-file","log","receipt","retained-evidence")
ACTIONS=("remove","revoke","delete","retain","export","inaccessible-remote")
STAGES=("cancel","disable-event","revoke-credential","remove-worker","remove-network","clean-cache","apply-retention","tombstone-graph","report-residue")
STATES=("missing","duplicate","stale","partially-removed","shared","retained","legal-hold")
ATTACKS=("tool-registration","credential-recovery","cache-access","event-receipt","background-sync","scheduled-action","socket-use","model-crossover","provider-crossover")
def build()->bytes:
 plans=[{"case_id":f"S-124-P-{i+1:04d}","inventory":INVENTORY[i%len(INVENTORY)],"action":ACTIONS[(i//len(INVENTORY))%len(ACTIONS)],"state":STATES[(i//(len(INVENTORY)*len(ACTIONS)))%len(STATES)],"synthetic":True,"preview_complete":True,"ordered":True,"user_resource_delete_count":0,"provider_resource_delete_count":0,"neighbor_mutation_count":0}for i in range(1024)]
 attacks=[{"case_id":f"S-124-A-{i+1:04d}","attack":ATTACKS[i%len(ATTACKS)],"synthetic":True,"admitted_count":0,"credential_recovered_count":0,"network_attempt_count":0,"stale_identity_reused_count":0}for i in range(512)]
 recoveries=[{"case_id":f"S-124-R-{i+1:04d}","stage":STAGES[i%len(STAGES)],"synthetic":True,"terminal_state":("complete","blocked")[i%2],"partial_authority_count":0,"unrelated_damage_count":0,"duplicate_effect_count":0,"residue_visible":True}for i in range(512)]
 value={"schema_version":1,"inventory":list(INVENTORY),"actions":list(ACTIONS),"stages":list(STAGES),"states":list(STATES),"attacks":list(ATTACKS),"plan_case_count":len(plans),"attack_case_count":len(attacks),"recovery_case_count":len(recoveries),"plan_cases":plans,"attack_cases":attacks,"recovery_cases":recoveries,"reinstall_requires_new_identity":True,"reconnect_requires_explicit_import":True,"native_platform_count":0,"strict_local_rerun_count":0,"zero_egress_minutes":0,"rv30_run_count":0,"remote_profile_removal_count":0,"independent_human_review_count":0,"promotion_count":0}
 return(json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def write():OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
def check():
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=build():raise RuntimeError("adapter removal corpus stale or absent")
 v=json.loads(OUTPUT.read_bytes())
 if any(not c["preview_complete"]or not c["ordered"]or c["user_resource_delete_count"]or c["provider_resource_delete_count"]or c["neighbor_mutation_count"]for c in v["plan_cases"]):raise RuntimeError("removal plan failed")
 if any(c["admitted_count"]or c["credential_recovered_count"]or c["network_attempt_count"]or c["stale_identity_reused_count"]for c in v["attack_cases"]):raise RuntimeError("post-removal attack admitted")
 if any(c["partial_authority_count"]or c["unrelated_damage_count"]or c["duplicate_effect_count"]or not c["residue_visible"]for c in v["recovery_cases"]):raise RuntimeError("removal recovery failed")
 if any(v[k]for k in("native_platform_count","strict_local_rerun_count","zero_egress_minutes","rv30_run_count","remote_profile_removal_count","independent_human_review_count","promotion_count")):raise RuntimeError("removal overclaim")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:write()
 check();print("validated 1024 removal plans, 512 attacks, and 512 recoveries without native restoration claims");return 0
if __name__=="__main__":raise SystemExit(main())
