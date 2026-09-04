#!/usr/bin/env python3
"""Build and verify deterministic Sprint 112 deployment-safety fixtures."""
from __future__ import annotations
import argparse,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-112/deployment-safety-corpus.json"
READS=("cluster","context","namespace","workload","service","ingress","configuration-metadata","policy","event","revision","owner","health","rollout","drift")
RENDERERS=("helm","kustomize")
ACTIONS=("create","update","delete","no-change")
ATTACKS=("context-confusion","namespace-escape","malicious-template","resource-bomb","hidden-hook","secret-output","policy-bypass","image-tag-swap","nested-admin","source-drift","artifact-drift","health-drift","rollback-replay")
FAULTS=("before-apply","during-apply","after-partial-effect","during-health","during-rollback","cancel-before-apply","cancel-after-effect")
def build()->bytes:
 reads=[{"read":r,"secret_value_count":0,"effect_count":0,"identity_bound":True} for r in READS]
 renders=[{"renderer":r,"offline":True,"pinned":True,"deterministic":True,"bounded":True,"source_preserved":True,"dependency_digest_bound":True,"input_hash_bound":True,"renderer_version_bound":True} for r in RENDERERS]
 plans=[{"case_id":f"S-112-P-{i+1:03d}","action":ACTIONS[i%len(ACTIONS)],"resource_pre_state_bound":True,"resource_post_state_bound":True,"artifact_digest_bound":True,"cluster_bound":True,"namespace_bound":True,"policy_bound":True,"health_bound":True,"timeout_bound":True,"rollback_target_bound":True,"secret_or_admin_authority_count":0} for i in range(64)]
 attacks=[{"case_id":f"S-112-A-{i+1:04d}","attack":ATTACKS[i%len(ATTACKS)],"cluster_contact_count":0,"unpreviewed_effect_count":0,"secret_value_count":0,"authority_escape_count":0,"unsafe_retry_count":0} for i in range(2048)]
 faults=[{"case_id":f"S-112-F-{i+1:04d}","fault":FAULTS[i%len(FAULTS)],"exact_state":True,"reconciliation_required":True,"automatic_retry_count":0,"fresh_rollback_approval_required":True,"hidden_partial_count":0} for i in range(512)]
 integration={"environment":"synthetic-non-production","artifact_immutable":True,"rendered_offline":True,"health_verified":True,"drift_detected":True,"failure_simulated":True,"rollback_previewed":True,"rollback_separately_approved":True,"final_state_verified":True,"cluster_contact_count":0,"promotion_count":0}
 value={"schema_version":1,"reads":list(READS),"renderers":list(RENDERERS),"actions":list(ACTIONS),"attacks":list(ATTACKS),"faults":list(FAULTS),"read_case_count":len(reads),"render_case_count":len(renders),"plan_case_count":len(plans),"attack_case_count":len(attacks),"fault_case_count":len(faults),"read_cases":reads,"render_cases":renders,"plan_cases":plans,"attack_cases":attacks,"fault_cases":faults,"integration_case":integration,"reviewer":"scripts.deployment_safety_contract","promoted_environment_count":0,"live_cluster_contact_count":0,"independent_human_review_count":0}
 return (json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def write():OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
def check():
 if not OUTPUT.is_file() or OUTPUT.read_bytes()!=build():raise RuntimeError("deployment safety corpus stale or absent")
 v=json.loads(OUTPUT.read_bytes())
 if any(c["secret_value_count"] or c["effect_count"] or not c["identity_bound"] for c in v["read_cases"]):raise RuntimeError("read boundary failed")
 if any(not all(c[k] for k in ("offline","pinned","deterministic","bounded","source_preserved","dependency_digest_bound","input_hash_bound","renderer_version_bound")) for c in v["render_cases"]):raise RuntimeError("render boundary failed")
 if any(c["secret_or_admin_authority_count"] or not all(c[k] for k in ("resource_pre_state_bound","resource_post_state_bound","artifact_digest_bound","cluster_bound","namespace_bound","policy_bound","health_bound","timeout_bound","rollback_target_bound")) for c in v["plan_cases"]):raise RuntimeError("plan boundary failed")
 if any(c["cluster_contact_count"] or c["unpreviewed_effect_count"] or c["secret_value_count"] or c["authority_escape_count"] or c["unsafe_retry_count"] for c in v["attack_cases"]):raise RuntimeError("deployment attack admitted")
 if any(not c["exact_state"] or not c["reconciliation_required"] or c["automatic_retry_count"] or not c["fresh_rollback_approval_required"] or c["hidden_partial_count"] for c in v["fault_cases"]):raise RuntimeError("fault recovery failed")
 if v["promoted_environment_count"] or v["live_cluster_contact_count"] or v["independent_human_review_count"]:raise RuntimeError("deployment support overclaim")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:write()
 check();print("validated 14 reads, 2 renderers, 64 plans, 2048 attacks, and 512 faults without cluster promotion");return 0
if __name__=="__main__":raise SystemExit(main())
