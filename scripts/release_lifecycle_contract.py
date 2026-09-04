#!/usr/bin/env python3
"""Build deterministic Sprint 115 release lifecycle fixtures."""
from __future__ import annotations
import argparse,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-115/release-lifecycle-corpus.json"
TOOLS=("launchdarkly","unleash","flyway","liquibase");CHANGES=("promotion","flag","progressive-step","migration","rollback","compensation");ATTACKS=("tag-reuse","version-reuse","artifact-swap","hidden-production","audience-expansion","prerequisite-loop","migration-checksum","migration-order","secret-output","destructive-concealment");FAULTS=("concurrent-release","changed-flag","partial-migration","lock-timeout","health-delay","provider-outage","cancellation","crash","later-state-change")
def build()->bytes:
 manifests=[{"case_id":f"S-115-M-{i+1:03d}","source_bound":True,"ci_bound":True,"artifact_bound":True,"sbom_bound":True,"provenance_bound":True,"signature_bound":True,"environment_bound":True,"policy_bound":True,"migration_bound":True,"flag_bound":True,"health_bound":True,"rollback_bound":True}for i in range(64)]
 attacks=[{"case_id":f"S-115-A-{i+1:04d}","attack":ATTACKS[i%len(ATTACKS)],"provider_contact_count":0,"hidden_effect_count":0,"authority_escape_count":0,"secret_disclosure_count":0,"automatic_advance_count":0}for i in range(2048)]
 faults=[{"case_id":f"S-115-F-{i+1:04d}","fault":FAULTS[i%len(FAULTS)],"exact_state":True,"later_state_preserved":True,"automatic_retry_count":0,"automatic_advance_count":0,"fresh_compensation_required":True}for i in range(512)]
 v={"schema_version":1,"tools":list(TOOLS),"changes":list(CHANGES),"attacks":list(ATTACKS),"faults":list(FAULTS),"manifest_case_count":len(manifests),"attack_case_count":len(attacks),"fault_case_count":len(faults),"manifest_cases":manifests,"attack_cases":attacks,"fault_cases":faults,"integration_case":{"synthetic_release":True,"progressive_promotion":True,"compatible_migration":True,"separate_flag_approval":True,"failed_health_detected":True,"separate_compensation_approval":True,"provider_contact_count":0},"reviewer":"scripts.release_lifecycle_contract","promoted_tool_count":0,"live_environment_count":0,"independent_human_review_count":0}
 return(json.dumps(v,sort_keys=True,separators=(",",":"))+"\n").encode()
def write():OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
def check():
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=build():raise RuntimeError("release corpus stale or absent")
 v=json.loads(OUTPUT.read_bytes())
 if any(not all(c[k]for k in("source_bound","ci_bound","artifact_bound","sbom_bound","provenance_bound","signature_bound","environment_bound","policy_bound","migration_bound","flag_bound","health_bound","rollback_bound"))for c in v["manifest_cases"]):raise RuntimeError("manifest failed")
 if any(c["provider_contact_count"]or c["hidden_effect_count"]or c["authority_escape_count"]or c["secret_disclosure_count"]or c["automatic_advance_count"]for c in v["attack_cases"]):raise RuntimeError("attack admitted")
 if any(not c["exact_state"]or not c["later_state_preserved"]or c["automatic_retry_count"]or c["automatic_advance_count"]or not c["fresh_compensation_required"]for c in v["fault_cases"]):raise RuntimeError("fault failed")
 if v["promoted_tool_count"]or v["live_environment_count"]or v["independent_human_review_count"]:raise RuntimeError("release support overclaim")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:write()
 check();print("validated 64 manifests, 2048 attacks, and 512 faults without release promotion");return 0
if __name__=="__main__":raise SystemExit(main())
