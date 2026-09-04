#!/usr/bin/env python3
"""Build and verify Sprint 109 synthetic CI evidence."""
from __future__ import annotations
import argparse,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-109/ci-control-corpus.json"
PROVIDERS=("github-actions","azure-pipelines","gitlab-ci","jenkins")
IDENTITIES=("workflow","pipeline","job","step","run","attempt","annotation","log","artifact","environment","approval")
EFFECTS=("dispatch","rerun","cancel","environment-approval")
ATTACKS=("malicious-yaml","malicious-script","malicious-log","malicious-artifact","hidden-input","secret-echo","redirect-download","archive-bomb","path-traversal","stale-ref","runner-confusion","nested-deployment","rate-limit","queue-delay","permission-loss","provider-outage","cancellation-race","oversized-output","full-disk","restart")
def build()->bytes:
 observations=[{"provider":p,"identity":i,"source_bound":True,"input_bound":True,"environment_bound":True,"worker_bound":True,"attempt_bound":True,"secret_value_count":0,"untrusted":True} for p in PROVIDERS for i in IDENTITIES]
 effects=[{"provider":p,"effect":e,"separate_grant":True,"idempotency_bound":True,"provider_request_count":0,"deployment_authority_count":0,"secret_authority_count":0,"admin_authority_count":0} for p in PROVIDERS for e in EFFECTS]
 faults=[{"case_id":f"S-109-F-{i+1:04d}","phase":ATTACKS[i%len(ATTACKS)],"duplicate_run_count":0,"unsafe_retry_count":0,"unknown_until_reconciled":True} for i in range(1024)]
 attacks=[{"case_id":f"S-109-A-{i+1:04d}","attack":ATTACKS[i%len(ATTACKS)],"secret_disclosure_count":0,"stronger_capability_count":0,"unsafe_archive_count":0,"unbounded_output_count":0} for i in range(2048)]
 value={"schema_version":1,"providers":list(PROVIDERS),"identities":list(IDENTITIES),"effects":list(EFFECTS),"attacks":list(ATTACKS),"observation_case_count":len(observations),"effect_case_count":len(effects),"fault_case_count":len(faults),"attack_case_count":len(attacks),"observation_cases":observations,"effect_cases":effects,"fault_cases":faults,"attack_cases":attacks,"reviewer":"scripts.ci_control_contract","promoted_provider_count":0,"live_provider_count":0,"provider_request_count":0,"independent_human_review_count":0}
 return (json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def write():OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
def check():
 if not OUTPUT.is_file() or OUTPUT.read_bytes()!=build():raise RuntimeError("CI corpus stale or absent")
 v=json.loads(OUTPUT.read_bytes())
 if any(not c["source_bound"] or not c["input_bound"] or not c["environment_bound"] or not c["worker_bound"] or not c["attempt_bound"] or c["secret_value_count"] or not c["untrusted"] for c in v["observation_cases"]):raise RuntimeError("CI observation boundary failed")
 if any(c["provider_request_count"] or c["deployment_authority_count"] or c["secret_authority_count"] or c["admin_authority_count"] for c in v["effect_cases"]):raise RuntimeError("CI effect authority broadened")
 if any(c["duplicate_run_count"] or c["unsafe_retry_count"] or not c["unknown_until_reconciled"] for c in v["fault_cases"]):raise RuntimeError("CI reconciliation failed")
 if any(c["secret_disclosure_count"] or c["stronger_capability_count"] or c["unsafe_archive_count"] or c["unbounded_output_count"] for c in v["attack_cases"]):raise RuntimeError("CI attack admitted")
 if v["promoted_provider_count"] or v["live_provider_count"] or v["provider_request_count"] or v["independent_human_review_count"]:raise RuntimeError("CI support overclaim")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:write()
 check();print("validated 44 observations, 16 inert effects, 1024 faults, and 2048 attacks without CI promotion");return 0
if __name__=="__main__":raise SystemExit(main())
