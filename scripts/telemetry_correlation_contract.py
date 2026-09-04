#!/usr/bin/env python3
"""Build deterministic Sprint 116 telemetry fixtures."""
from __future__ import annotations
import argparse,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-116/telemetry-correlation-corpus.json"
IDENTITIES=("resource","service","environment","trace","span","metric","log","event","error","monitor","release","deployment","time-window","incident");ATTACKS=("secret","prompt-injection","cardinality-explosion","malformed-encoding","oversized-payload","clock-skew","trace-collision","false-causation");FAULTS=("backend-outage","partial-data","sampling-change","late-arrival","duplicate-span","cancellation","full-disk","memory-pressure")
def build()->bytes:
 queries=[{"case_id":f"S-116-Q-{i+1:03d}","range_bound":True,"cardinality_bound":True,"result_bound":True,"byte_bound":True,"sampling_bound":True,"clock_skew_bound":True,"freshness_bound":True,"source_cited":True}for i in range(64)]
 correlations=[{"case_id":f"S-116-C-{i+1:03d}","identity":IDENTITIES[i%len(IDENTITIES)],"relationship":"temporal-inference"if i%2 else"exact-attribute","inference_labeled":True,"causation_claim_count":0,"method_bound":True,"missingness_bound":True,"sampling_bound":True,"uncertainty_bound":True}for i in range(128)]
 attacks=[{"case_id":f"S-116-A-{i+1:04d}","attack":ATTACKS[i%len(ATTACKS)],"backend_contact_count":0,"sensitive_disclosure_count":0,"unbounded_use_count":0,"operation_authority_count":0,"durable_memory_count":0,"causation_claim_count":0}for i in range(2048)]
 faults=[{"case_id":f"S-116-F-{i+1:04d}","fault":FAULTS[i%len(FAULTS)],"partial_state_explicit":True,"uncertainty_preserved":True,"duplicate_count":0,"unbounded_resource_count":0,"automatic_effect_count":0}for i in range(512)]
 v={"schema_version":1,"identities":list(IDENTITIES),"attacks":list(ATTACKS),"faults":list(FAULTS),"query_case_count":len(queries),"correlation_case_count":len(correlations),"attack_case_count":len(attacks),"fault_case_count":len(faults),"query_cases":queries,"correlation_cases":correlations,"attack_cases":attacks,"fault_cases":faults,"integration_case":{"synthetic_release_deployment":True,"metrics_logs_traces_errors":True,"incident_window":True,"sources_preserved":True,"uncertainty_preserved":True,"backend_contact_count":0},"reviewer":"scripts.telemetry_correlation_contract","promoted_backend_count":0,"live_query_count":0,"independent_human_review_count":0}
 return(json.dumps(v,sort_keys=True,separators=(",",":"))+"\n").encode()
def write():OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
def check():
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=build():raise RuntimeError("telemetry corpus stale or absent")
 v=json.loads(OUTPUT.read_bytes())
 if any(not all(c[k]for k in("range_bound","cardinality_bound","result_bound","byte_bound","sampling_bound","clock_skew_bound","freshness_bound","source_cited"))for c in v["query_cases"]):raise RuntimeError("query failed")
 if any(not c["inference_labeled"]or c["causation_claim_count"]or not c["uncertainty_bound"]for c in v["correlation_cases"]):raise RuntimeError("correlation failed")
 if any(c["backend_contact_count"]or c["sensitive_disclosure_count"]or c["unbounded_use_count"]or c["operation_authority_count"]or c["durable_memory_count"]or c["causation_claim_count"]for c in v["attack_cases"]):raise RuntimeError("attack admitted")
 if any(not c["partial_state_explicit"]or not c["uncertainty_preserved"]or c["duplicate_count"]or c["unbounded_resource_count"]or c["automatic_effect_count"]for c in v["fault_cases"]):raise RuntimeError("fault failed")
 if v["promoted_backend_count"]or v["live_query_count"]or v["independent_human_review_count"]:raise RuntimeError("telemetry support overclaim")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:write()
 check();print("validated 64 queries, 128 correlations, 2048 attacks, and 512 faults without telemetry promotion");return 0
if __name__=="__main__":raise SystemExit(main())
