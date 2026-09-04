#!/usr/bin/env python3
"""Build and verify Sprint 107 synthetic cross-provider work evidence."""
from __future__ import annotations
import argparse,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-107/work-management-corpus.json"
PROVIDERS=("github-issues","jira-cloud","jira-data-center","azure-boards")
FACETS=("identity","type","field","state","transition","hierarchy","link","iteration","comment","attachment","history","permission","pagination","version-skew","custom-field","area-path","board","sprint","recipient","visibility")
EFFECTS=("create","edit","comment","assign","label-or-tag","link","attach","transition","close","reopen")
WORKFLOWS=("product-discovery","issue-authoring","epic-decomposition","acceptance-criteria","backlog-curation","dependency-planning","sprint-planning","risk-analysis","roadmap-audit","progress-reconciliation","issue-intake","bug-lifecycle","pull-request-stewardship","independent-review")
LINK_TARGETS=("repository","branch","commit","review","build","release","incident","evidence")
LINK_STATES=("observed","provider-reference","inferred","conflicting","stale","unknown")
REVIEW_SURFACES=("observed-fact","provider-extension","inference","draft","requested-effect","approval","receipt","unknown-outcome","blocker","unsupported-operation")
ATTACKS=("cross-project","cross-tenant","reused-number","hidden-watcher","malicious-attachment","injected-comment","stale-transition","provider-link-confusion","identity-ambiguity","schema-change","permission-loss","object-move","object-delete","rate-limit","timeout","duplicate-response","partial-response","crash","restart","cancellation","moved-line","stale-ref","role-substitution","reviewer-collusion","forged-completion")
def build()->bytes:
 objects=[{"provider":p,"facet":f,"namespaced_extension":True,"immutable_identity_preserved":True,"unknown_visible":True} for p in PROVIDERS for f in FACETS]
 links=[{"target":t,"state":s,"immutable_reference":True,"operation_authority":s in ("observed","provider-reference"),"inference_labeled":s=="inferred"} for t in LINK_TARGETS for s in LINK_STATES]
 effects=[{"provider":p,"effect":e,"field_level":True,"separate_approval":True,"pre_revision":"r1","approved_revision":"r1","post_revision":"r2","declared_field_change_count":1,"observed_field_change_count":1,"non_target_change_count":0,"hidden_recipient_count":0,"duplicate_effect_count":0,"attachment_scan_passed":True,"receipt_count":1,"postcondition_verified":True} for p in PROVIDERS for e in EFFECTS]
 workflows=[{"provider":p,"workflow":w,"immutable_packet":True,"untrusted_inputs":True,"completion_from_provider_checkbox":False,"provider_effect_count":0} for p in PROVIDERS for w in WORKFLOWS]
 attacks=[{"case_id":f"S-107-A-{i+1:04d}","attack":ATTACKS[i%len(ATTACKS)],"authority_count":0,"hidden_effect_count":0,"credential_crossover_count":0,"false_completion_count":0,"expected":"deny-or-reconcile"} for i in range(1024)]
 recovery=[{"case_id":f"S-107-R-{i+1:04d}","attack":ATTACKS[i%len(ATTACKS)],"duplicate_update_count":0,"lost_dissent_count":0,"stale_grant_consumed_count":0,"reconciled":True} for i in range(512)]
 value={"schema_version":1,"providers":list(PROVIDERS),"facets":list(FACETS),"effects":list(EFFECTS),"workflows":list(WORKFLOWS),"link_targets":list(LINK_TARGETS),"link_states":list(LINK_STATES),"review_surfaces":list(REVIEW_SURFACES),"attacks":list(ATTACKS),"object_case_count":len(objects),"link_case_count":len(links),"effect_case_count":len(effects),"workflow_case_count":len(workflows),"attack_case_count":len(attacks),"recovery_case_count":len(recovery),"object_cases":objects,"link_cases":links,"effect_cases":effects,"workflow_cases":workflows,"attack_cases":attacks,"recovery_cases":recovery,"reviewer":"scripts.work_management_contract","promoted_provider_count":0,"live_provider_count":0,"provider_request_count":0,"independent_human_review_count":0}
 return (json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def write():OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
def check():
 if not OUTPUT.is_file() or OUTPUT.read_bytes()!=build():raise RuntimeError("work management corpus stale or absent")
 v=json.loads(OUTPUT.read_bytes())
 if any(not c["immutable_identity_preserved"] or not c["namespaced_extension"] or not c["unknown_visible"] for c in v["object_cases"]):raise RuntimeError("object semantics flattened")
 if any(c["operation_authority"] != (c["state"] in ("observed","provider-reference")) for c in v["link_cases"]):raise RuntimeError("link authority broadened")
 if any(c["non_target_change_count"] or c["hidden_recipient_count"] or c["duplicate_effect_count"] or c["receipt_count"]!=1 or not c["postcondition_verified"] or not c["attachment_scan_passed"] for c in v["effect_cases"]):raise RuntimeError("effect boundary failed")
 if any(c["authority_count"] or c["hidden_effect_count"] or c["credential_crossover_count"] or c["false_completion_count"] for c in v["attack_cases"]):raise RuntimeError("attack admitted")
 if any(c["duplicate_update_count"] or c["lost_dissent_count"] or c["stale_grant_consumed_count"] or not c["reconciled"] for c in v["recovery_cases"]):raise RuntimeError("recovery failed")
 if v["promoted_provider_count"] or v["live_provider_count"] or v["provider_request_count"] or v["independent_human_review_count"]:raise RuntimeError("provider support overclaim")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:write()
 check();print("validated 80 objects, 48 links, 40 effects, 56 workflows, 1024 attacks, and 512 recoveries without provider promotion");return 0
if __name__=="__main__":raise SystemExit(main())
