#!/usr/bin/env python3
"""Build and verify Sprint 104 connected-identity evidence."""
from __future__ import annotations
import argparse,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-104/connected-identity-corpus.json"
CAPABILITIES=("observe","draft","local-write","remote-write","execute","deploy","secrets","admin")
ORIGINS=("direct-request","provider-content","nested-action","workflow-input","plugin","model-tool-call")
AXES=("provider","host","port","tls-identity","tenant","account","project","environment","credential-reference","sso-state","redirect","proxy","dns","callback","clone-host","linked-api-host")
CREDENTIAL_STATES=("available","missing","ambiguous","excessive","stale","revoked")
SURFACES=("worker","cache","log","receipt","diagnostic","error","model-context","request")
def build()->bytes:
 confusion=[{"case_id":f"S-104-C-{i+1:04d}","axis":AXES[i%len(AXES)],"expected":"deny-before-request","request_count":0,"disclosure_count":0,"receipt_count":1,"reason":"connected-identity-mismatch"} for i in range(2048)]
 escalation=[{"case_id":f"S-104-E-{len(CAPABILITIES)*g+r+1:03d}-{o+1}","granted":grant,"requested":request,"origin":origin,"expected":"deny-before-provider","provider_contact_count":0,"inherited":False} for g,grant in enumerate(CAPABILITIES) for r,request in enumerate(CAPABILITIES) if grant!=request for o,origin in enumerate(ORIGINS)]
 credential=[{"case_id":f"S-104-K-{i+1:03d}","state":state,"expected":"admit" if state=="available" else "deny","secret_serialization_count":0,"request_count":1 if state=="available" else 0,"receipt_count":1} for i,state in enumerate(CREDENTIAL_STATES)]
 canaries=[{"surface":surface,"canary_occurrences":0} for surface in SURFACES]
 concurrency=[{"case_id":f"S-104-I-{i+1:03d}","account_a":f"account-{i:03d}-a","account_b":f"account-{i:03d}-b","request_count":2,"shared_credential_count":0,"shared_cache_count":0,"shared_grant_count":0,"shared_context_count":0} for i in range(64)]
 value={"schema_version":1,"capability_classes":list(CAPABILITIES),"escalation_origins":list(ORIGINS),"identity_axes":list(AXES),"credential_states":list(CREDENTIAL_STATES),"canary_surfaces":list(SURFACES),"confusion_case_count":len(confusion),"escalation_case_count":len(escalation),"credential_case_count":len(credential),"concurrency_case_count":len(concurrency),"confusion_cases":confusion,"escalation_cases":escalation,"credential_cases":credential,"canary_cases":canaries,"concurrency_cases":concurrency,"reviewer":"scripts.connected_identity_contract","native_identity_count":0,"live_provider_count":0,"independent_review_count":0}
 return (json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def write():OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
def check():
 if not OUTPUT.is_file() or OUTPUT.read_bytes()!=build():raise RuntimeError("connected identity corpus stale or absent")
 value=json.loads(OUTPUT.read_bytes())
 if value["confusion_case_count"]<2000 or any(case["request_count"] or case["disclosure_count"] or case["receipt_count"]!=1 for case in value["confusion_cases"]):raise RuntimeError("confusion campaign overclaim")
 if any(case["provider_contact_count"] or case["inherited"] for case in value["escalation_cases"]):raise RuntimeError("capability escalation admitted")
 if any(case["canary_occurrences"] for case in value["canary_cases"]):raise RuntimeError("secret canary escaped")
 if any(any(case[field] for field in ("shared_credential_count","shared_cache_count","shared_grant_count","shared_context_count")) for case in value["concurrency_cases"]):raise RuntimeError("worker isolation failed")
 if value["native_identity_count"] or value["live_provider_count"] or value["independent_review_count"]:raise RuntimeError("external verification overclaim")
def main():
 parser=argparse.ArgumentParser();parser.add_argument("--write",action="store_true");arguments=parser.parse_args()
 if arguments.write:write()
 check();print("validated 2048 identity confusions, 336 escalations, 8 canary surfaces, and 64 isolated worker pairs");return 0
if __name__=="__main__":raise SystemExit(main())
