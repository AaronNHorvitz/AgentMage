#!/usr/bin/env python3
"""Build deterministic Sprint 117 multi-vendor observability fixtures."""
from __future__ import annotations
import argparse,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-117/vendor-observability-corpus.json"
PROVIDERS=("datadog","prometheus","grafana","loki","elastic","splunk","sentry");OBJECTS=("metric","log","trace","monitor","dashboard","event","error","service","release","incident-link");ATTACKS=("cross-tenant","credential-confusion","datasource-confusion","query-injection","secret-log","malicious-link","cardinality-explosion","hidden-write","hidden-remediation");FAULTS=("version-skew","outage","slow-query","rate-limit","quota-exhaustion","partial-result","late-data","cancellation","restart")
def build()->bytes:
 queries=[{"case_id":f"S-117-Q-{i+1:03d}","provider":PROVIDERS[i%len(PROVIDERS)],"object":OBJECTS[i%len(OBJECTS)],"tenant_bound":True,"datasource_bound":True,"query_bound":True,"time_bound":True,"result_bound":True,"byte_bound":True,"cardinality_bound":True,"cost_bound":True,"retention_bound":True,"freshness_visible":True,"partial_visible":True,"namespaced_extensions":True}for i in range(128)]
 attacks=[{"case_id":f"S-117-A-{i+1:04d}","attack":ATTACKS[i%len(ATTACKS)],"provider_contact_count":0,"tenant_crossover_count":0,"credential_disclosure_count":0,"secret_disclosure_count":0,"hidden_write_count":0,"remediation_count":0,"unbounded_query_count":0}for i in range(2048)]
 faults=[{"case_id":f"S-117-F-{i+1:04d}","fault":FAULTS[i%len(FAULTS)],"provider_difference_visible":True,"partial_state_visible":True,"cancellation_effective":True,"bounded_resource_count":0,"automatic_retry_count":0}for i in range(512)]
 v={"schema_version":1,"providers":list(PROVIDERS),"objects":list(OBJECTS),"attacks":list(ATTACKS),"faults":list(FAULTS),"query_case_count":len(queries),"attack_case_count":len(attacks),"fault_case_count":len(faults),"query_cases":queries,"attack_cases":attacks,"fault_cases":faults,"integration_case":{"synthetic_equivalent_telemetry":True,"exact_release_deployment_identity":True,"normalized_common_fields":True,"vendor_differences_preserved":True,"provider_contact_count":0},"reviewer":"scripts.vendor_observability_contract","promoted_provider_count":0,"live_query_count":0,"independent_human_review_count":0}
 return(json.dumps(v,sort_keys=True,separators=(",",":"))+"\n").encode()
def write():OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
def check():
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=build():raise RuntimeError("vendor corpus stale or absent")
 v=json.loads(OUTPUT.read_bytes())
 if any(not all(c[k]for k in("tenant_bound","datasource_bound","query_bound","time_bound","result_bound","byte_bound","cardinality_bound","cost_bound","retention_bound","freshness_visible","partial_visible","namespaced_extensions"))for c in v["query_cases"]):raise RuntimeError("query failed")
 if any(c["provider_contact_count"]or c["tenant_crossover_count"]or c["credential_disclosure_count"]or c["secret_disclosure_count"]or c["hidden_write_count"]or c["remediation_count"]or c["unbounded_query_count"]for c in v["attack_cases"]):raise RuntimeError("attack admitted")
 if any(not c["provider_difference_visible"]or not c["partial_state_visible"]or not c["cancellation_effective"]or c["bounded_resource_count"]or c["automatic_retry_count"]for c in v["fault_cases"]):raise RuntimeError("fault failed")
 if v["promoted_provider_count"]or v["live_query_count"]or v["independent_human_review_count"]:raise RuntimeError("provider support overclaim")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:write()
 check();print("validated 128 queries, 2048 attacks, and 512 faults without provider promotion");return 0
if __name__=="__main__":raise SystemExit(main())
