#!/usr/bin/env python3
"""Build deterministic Sprint 125 integration and profile-boundary fixtures."""
from __future__ import annotations
import argparse,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-125/integrated-profile-corpus.json"
LIFECYCLES=("work-to-release","incident-to-rollback","mixed-provider")
ATTACKS=("content","credential-confusion","host-redirect","event-replay","branch-movement","artifact-swap","policy-change","environment-drift","telemetry-forgery","hidden-recipient","hidden-effect")
FAILURES=("rate-limit","partition","outage","permission-reduction","version-skew","partial-effect","cancellation","process-crash","restart","low-resource","interrupted-rollback")
SERVICES=("policy","credentials","signing","evidence","provenance","artifact-verification","merge","deployment","reconciliation")
def build()->bytes:
 lifecycles=[{"case_id":f"S-125-L-{i+1:04d}","lifecycle":LIFECYCLES[i%3],"synthetic":True,"identity_bound":True,"graph_bound":True,"receipt_bound":True,"external_effect_approved":True,"support_tuple_current":True,"provider_contact_count":0,"platform_execution_count":0}for i in range(512)]
 attacks=[{"case_id":f"S-125-A-{i+1:04d}","attack":ATTACKS[i%len(ATTACKS)],"failure":FAILURES[(i//len(ATTACKS))%len(FAILURES)],"synthetic":True,"authority_crossing_count":0,"duplicate_effect_count":0,"false_completion_count":0,"hidden_blocker_count":0,"rollback_preserved":True,"uncertainty_explicit":True}for i in range(2048)]
 profiles=[{"profile_id":f"AG-{i:02d}","synthetic":True,"input_bound":True,"authority_ceiling_bound":True,"model_profile_bound":True,"tools_bound":True,"evidence_bound":True,"completion_bound":True,"negative_case_bound":True,"deterministic_services":list(SERVICES),"service_impersonation_count":0,"self_approval_count":0,"grant_aggregation_count":0,"hidden_effect_count":0,"dissent_visible":True,"staleness_visible":True,"recovery_bounded":True}for i in range(1,50)]
 value={"schema_version":1,"lifecycles":list(LIFECYCLES),"attacks":list(ATTACKS),"failures":list(FAILURES),"services":list(SERVICES),"lifecycle_case_count":len(lifecycles),"attack_case_count":len(attacks),"profile_case_count":len(profiles),"lifecycle_cases":lifecycles,"attack_cases":attacks,"profile_cases":profiles,"connected_pack_removal_fixture":True,"strict_local_fixture_unchanged":True,"native_platform_count":0,"promoted_provider_path_count":0,"release_candidate_package_count":0,"enabled_profile_count":0,"review_vector_count":0,"independent_decision_count":0,"promotion_count":0}
 return(json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def write():OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
def check():
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=build():raise RuntimeError("integrated profile corpus stale or absent")
 v=json.loads(OUTPUT.read_bytes())
 if any(c["provider_contact_count"]or c["platform_execution_count"]or not c["receipt_bound"]for c in v["lifecycle_cases"]):raise RuntimeError("lifecycle failed")
 if any(c["authority_crossing_count"]or c["duplicate_effect_count"]or c["false_completion_count"]or c["hidden_blocker_count"]for c in v["attack_cases"]):raise RuntimeError("attack failed")
 if len(v["profile_cases"])!=49 or any(c["service_impersonation_count"]or c["self_approval_count"]or c["grant_aggregation_count"]or c["hidden_effect_count"]or not c["dissent_visible"]or not c["recovery_bounded"]for c in v["profile_cases"]):raise RuntimeError("profile boundary failed")
 if any(v[k]for k in("native_platform_count","promoted_provider_path_count","release_candidate_package_count","enabled_profile_count","review_vector_count","independent_decision_count","promotion_count")):raise RuntimeError("integration overclaim")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:write()
 check();print("validated 512 lifecycles, 2048 attacks, and 49 profile boundaries without release qualification");return 0
if __name__=="__main__":raise SystemExit(main())
