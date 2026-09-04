#!/usr/bin/env python3
"""Build deterministic Sprint 123 provider resilience fixtures."""
from __future__ import annotations
import argparse,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-123/provider-resilience-corpus.json"
VERSIONS=("minimum","maximum","future","provider-changed")
FAILURES=("rate-limit","quota","revocation","permission-reduction","outage","partition","latency","clock-skew","eventual-consistency","event-flood","pagination-loop")
PRESSURES=("malformed-schema","oversized-schema","oversized-log","oversized-archive","oversized-artifact","telemetry-cardinality","repository-count","worker-concurrency","disk","memory","cpu","gpu","cancellation")
ATTACKS=("prompt-attack","secret-canary","host-confusion","account-confusion","project-confusion","environment-confusion","credential-confusion","object-confusion","authority-escalation","hidden-effect","false-completion")
TRANSITIONS=("read","event","write","execute","deploy","reconciliation","persistence","recovery")
BLOCKERS=("failed","skipped","stale","flake","quarantine","suppressed","unreconciled","unreviewed")
def build()->bytes:
 conformance=[{"case_id":f"S-123-C-{i+1:04d}","version":VERSIONS[i%len(VERSIONS)],"failure":FAILURES[(i//len(VERSIONS))%len(FAILURES)],"pressure":PRESSURES[(i//(len(VERSIONS)*len(FAILURES)))%len(PRESSURES)],"seed":i+1,"synthetic":True,"replay_sha256":f"{i+1:064x}","disposition":"refused","stale_support_claim_count":0,"authority_leak_count":0}for i in range(2048)]
 attacks=[{"case_id":f"S-123-A-{i+1:04d}","attack":ATTACKS[i%len(ATTACKS)],"transition":TRANSITIONS[(i//len(ATTACKS))%len(TRANSITIONS)],"phase":("before","during","after")[(i//(len(ATTACKS)*len(TRANSITIONS)))%3],"synthetic":True,"canary_disclosure_count":0,"credential_crossover_count":0,"duplicate_effect_count":0,"false_completion_count":0,"uncertainty_explicit":True,"cleanup_bounded":True}for i in range(2048)]
 blockers=[{"case_id":f"S-123-B-{i+1:04d}","state":BLOCKERS[i%len(BLOCKERS)],"synthetic":True,"visible":True,"gate_closed":False,"suppressed_count":0}for i in range(512)]
 value={"schema_version":1,"versions":list(VERSIONS),"failures":list(FAILURES),"pressures":list(PRESSURES),"attacks":list(ATTACKS),"transitions":list(TRANSITIONS),"blockers":list(BLOCKERS),"conformance_case_count":len(conformance),"attack_case_count":len(attacks),"blocker_case_count":len(blockers),"conformance_cases":conformance,"attack_cases":attacks,"blocker_cases":blockers,"deterministic_seeds":True,"shrinking_contract":True,"coverage_contract":True,"sanitizer_contract":True,"raw_result_schema":True,"environment_identity_bound":True,"approved_live_environment_count":0,"native_fuzz_run_count":0,"prolonged_campaign_minutes":0,"review_vector_count":0,"promoted_provider_count":0,"independent_red_team_review_count":0}
 return(json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def write():OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
def check():
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=build():raise RuntimeError("provider resilience corpus stale or absent")
 v=json.loads(OUTPUT.read_bytes())
 if any(c["stale_support_claim_count"]or c["authority_leak_count"]or c["disposition"]!="refused"for c in v["conformance_cases"]):raise RuntimeError("conformance failure")
 if any(c["canary_disclosure_count"]or c["credential_crossover_count"]or c["duplicate_effect_count"]or c["false_completion_count"]or not c["uncertainty_explicit"]or not c["cleanup_bounded"]for c in v["attack_cases"]):raise RuntimeError("attack failure")
 if any(not c["visible"]or c["gate_closed"]or c["suppressed_count"]for c in v["blocker_cases"]):raise RuntimeError("blocker hidden")
 if any(v[k]for k in("approved_live_environment_count","native_fuzz_run_count","prolonged_campaign_minutes","review_vector_count","promoted_provider_count","independent_red_team_review_count")):raise RuntimeError("resilience overclaim")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:write()
 check();print("validated 2048 conformance, 2048 attack, and 512 blocker cases without provider promotion");return 0
if __name__=="__main__":raise SystemExit(main())
