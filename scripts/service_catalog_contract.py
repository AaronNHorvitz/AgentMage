#!/usr/bin/env python3
"""Build deterministic Sprint 119 service-catalog fixtures."""
from __future__ import annotations
import argparse,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-119/service-catalog-corpus.json"
KINDS=("component","api","resource","system","domain","group","user");LINKS=("repository","ci","artifact","environment","observability","incident","documentation","release");EXTENSIONS=("port","cortex","compass");STATES=("resolved","inferred","conflicting","stale","missing","provider-mismatch");ATTACKS=("duplicate-name","forged-url","malicious-annotation","owner-confusion","cross-tenant","injected-instruction","graph-cycle","plugin-registration","tool-registration");FAULTS=("owner-change","repository-transfer","telemetry-removal","entity-delete","catalog-stale","provider-stale")
def build()->bytes:
 entities=[{"case_id":f"S-119-E-{i+1:03d}","kind":KINDS[i%len(KINDS)],"link":LINKS[i%len(LINKS)],"state":STATES[i%len(STATES)],"entity_identity_bound":True,"owner_identity_bound":True,"provider_resolved_independently":True,"unresolved_visible":True,"descriptor_untrusted":True}for i in range(128)]
 attacks=[{"case_id":f"S-119-A-{i+1:04d}","attack":ATTACKS[i%len(ATTACKS)],"provider_contact_count":0,"identity_guess_count":0,"evidence_overwrite_count":0,"authority_grant_count":0,"registration_count":0,"cross_tenant_count":0}for i in range(2048)]
 faults=[{"case_id":f"S-119-F-{i+1:04d}","fault":FAULTS[i%len(FAULTS)],"drift_visible":True,"prior_evidence_preserved":True,"unresolved_state":True,"automatic_rebind_count":0}for i in range(512)]
 v={"schema_version":1,"kinds":list(KINDS),"links":list(LINKS),"extensions":list(EXTENSIONS),"states":list(STATES),"attacks":list(ATTACKS),"faults":list(FAULTS),"entity_case_count":len(entities),"attack_case_count":len(attacks),"fault_case_count":len(faults),"entity_cases":entities,"attack_cases":attacks,"fault_cases":faults,"integration_case":{"synthetic_service_graph":True,"all_provider_links_independent":True,"owner_relation_independent":True,"provider_contact_count":0},"reviewer":"scripts.service_catalog_contract","promoted_provider_count":0,"registered_extension_count":0,"live_query_count":0,"independent_human_review_count":0}
 return(json.dumps(v,sort_keys=True,separators=(",",":"))+"\n").encode()
def write():OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
def check():
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=build():raise RuntimeError("catalog corpus stale or absent")
 v=json.loads(OUTPUT.read_bytes())
 if any(not all(c[k]for k in("entity_identity_bound","owner_identity_bound","provider_resolved_independently","unresolved_visible","descriptor_untrusted"))for c in v["entity_cases"]):raise RuntimeError("entity failed")
 if any(c["provider_contact_count"]or c["identity_guess_count"]or c["evidence_overwrite_count"]or c["authority_grant_count"]or c["registration_count"]or c["cross_tenant_count"]for c in v["attack_cases"]):raise RuntimeError("attack admitted")
 if any(not c["drift_visible"]or not c["prior_evidence_preserved"]or not c["unresolved_state"]or c["automatic_rebind_count"]for c in v["fault_cases"]):raise RuntimeError("fault failed")
 if v["promoted_provider_count"]or v["registered_extension_count"]or v["live_query_count"]or v["independent_human_review_count"]:raise RuntimeError("catalog support overclaim")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:write()
 check();print("validated 128 entities, 2048 attacks, and 512 faults without catalog promotion");return 0
if __name__=="__main__":raise SystemExit(main())
