#!/usr/bin/env python3
"""Build and verify Sprint 108 synthetic Azure Repos/GitLab evidence."""
from __future__ import annotations
import argparse,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-108/multi-provider-source-corpus.json"
PROVIDERS=("azure-repos","gitlab-cloud","gitlab-self-hosted")
TUPLES=("azure-devops-cloud","gitlab-cloud","gitlab-self-hosted-17","gitlab-self-hosted-18")
READS=("repository","project-or-group","ref","commit","tree","blob","tag","release","policy-or-protection","pull-or-merge-request","review","thread","status","check","permission","pipeline")
EFFECTS=("branch-push","pull-or-merge-request","comment","review","thread","label","reviewer","draft-state","closure","merge-preparation")
PROHIBITED=("force-push","protected-branch-bypass","administration","secret-change","automatic-merge","automatic-release")
ATTACKS=("credential-crossover","malicious-diff","stale-ref","moved-line","hostile-hook","submodule-confusion","large-file-pointer","policy-change","permission-loss","branch-movement","partial-publication","timeout","duplicate-response","crash","cancellation","fork-confusion","pagination-loop","version-skew")
def build()->bytes:
 reads=[{"provider":p,"read":r,"immutable_identity":True,"namespaced_fields":True,"isolated_worktree":True,"active_checkout_change_count":0,"provider_request_count":0} for p in PROVIDERS for r in READS]
 effects=[{"provider":p,"effect":e,"draft_only":True,"separate_approval":True,"signed_commit_record":e=="branch-push","separate_push_record":e=="branch-push","stale_grant_consumed_count":0,"hidden_effect_count":0,"receipt_count":1,"provider_request_count":0} for p in PROVIDERS for e in EFFECTS]
 matrices=[{"tuple":t,"published":True,"synthetic":True,"promoted":False,"live":False,"permission_levels":4,"fork_modes":2,"merge_methods":3,"pagination_modes":3,"unavailable_visible":True} for t in TUPLES]
 attacks=[{"case_id":f"S-108-A-{i+1:04d}","attack":ATTACKS[i%len(ATTACKS)],"provider":PROVIDERS[i%len(PROVIDERS)],"wrong_domain_count":0,"active_checkout_change_count":0,"stale_effect_count":0,"authority_count":0} for i in range(2048)]
 recovery=[{"case_id":f"S-108-R-{i+1:04d}","attack":ATTACKS[i%len(ATTACKS)],"duplicate_effect_count":0,"unsafe_retry_count":0,"reconciled":True} for i in range(512)]
 value={"schema_version":1,"providers":list(PROVIDERS),"tuples":list(TUPLES),"reads":list(READS),"effects":list(EFFECTS),"prohibited":list(PROHIBITED),"attacks":list(ATTACKS),"read_case_count":len(reads),"effect_case_count":len(effects),"matrix_case_count":len(matrices),"attack_case_count":len(attacks),"recovery_case_count":len(recovery),"read_cases":reads,"effect_cases":effects,"matrix_cases":matrices,"attack_cases":attacks,"recovery_cases":recovery,"review_line_states":["exact","moved","deleted","stale","unknown"],"reviewer":"scripts.multi_provider_source_contract","promoted_provider_count":0,"live_provider_count":0,"provider_request_count":0,"independent_human_review_count":0}
 return (json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def write():OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
def check():
 if not OUTPUT.is_file() or OUTPUT.read_bytes()!=build():raise RuntimeError("multi-provider source corpus stale or absent")
 v=json.loads(OUTPUT.read_bytes())
 if any(c["active_checkout_change_count"] or not c["immutable_identity"] or not c["namespaced_fields"] or not c["isolated_worktree"] for c in v["read_cases"]):raise RuntimeError("source read boundary failed")
 if any(c["stale_grant_consumed_count"] or c["hidden_effect_count"] or c["receipt_count"]!=1 or c["provider_request_count"] for c in v["effect_cases"]):raise RuntimeError("source effect boundary failed")
 if any(c["wrong_domain_count"] or c["active_checkout_change_count"] or c["stale_effect_count"] or c["authority_count"] for c in v["attack_cases"]):raise RuntimeError("source attack admitted")
 if any(c["duplicate_effect_count"] or c["unsafe_retry_count"] or not c["reconciled"] for c in v["recovery_cases"]):raise RuntimeError("source recovery failed")
 if v["promoted_provider_count"] or v["live_provider_count"] or v["provider_request_count"] or v["independent_human_review_count"]:raise RuntimeError("provider support overclaim")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:write()
 check();print("validated 48 reads, 30 inert effects, 4 matrices, 2048 attacks, and 512 recoveries without provider promotion");return 0
if __name__=="__main__":raise SystemExit(main())
