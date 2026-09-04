#!/usr/bin/env python3
"""Build deterministic Sprint 126 truthful release-checkpoint fixtures."""
from __future__ import annotations
import argparse,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-126/release-checkpoint-corpus.json"
STATES=("supported","degraded","unsupported","disabled","post-ga")
BLOCKERS=("failed","skipped","stale","unavailable","flaky","quarantined","suppressed","unreconciled","unreviewed")
DOCUMENTS=("README.md","PRD.md","TASKS.md","SECURITY-REVIEW.md","DELIVERY-SYSTEM.md","WINDOWS-BOUNDARIES.md","MODEL-PROVENANCE-POLICY.md","SECURITY.md")
def build()->bytes:
 docs=[{"path":p,"present":(ROOT/p).is_file(),"source_bound":True,"support_claim":"unsupported-pre-release"}for p in DOCUMENTS]
 claims=[{"case_id":f"S-126-C-{i+1:03d}","state":STATES[i%len(STATES)],"blocker":BLOCKERS[(i//len(STATES))%len(BLOCKERS)],"synthetic":True,"visible":True,"g_ga_closed":False,"package_produced":False,"publication_allowed":False,"support_promoted":False}for i in range(256)]
 value={"schema_version":1,"states":list(STATES),"blockers":list(BLOCKERS),"documents":docs,"claim_cases":claims,"claim_case_count":len(claims),"requirement_graph_current":True,"traceability_current":True,"support_matrix_closed":True,"apple_silicon_state":"post-ga","source_sbom_present":(ROOT/"supply-chain/sbom.cdx.json").is_file(),"signed_binary_manifest_count":0,"native_reproduction_count":0,"reviewer_signature_count":0,"user_release_approval_count":0,"release_package_count":0,"promotion_count":0}
 return(json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def write():OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
def check():
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=build():raise RuntimeError("release checkpoint corpus stale or absent")
 v=json.loads(OUTPUT.read_bytes())
 if any(not d["present"]or not d["source_bound"]or d["support_claim"]!="unsupported-pre-release"for d in v["documents"]):raise RuntimeError("documentation truth failed")
 if any(not c["visible"]or c["g_ga_closed"]or c["package_produced"]or c["publication_allowed"]or c["support_promoted"]for c in v["claim_cases"]):raise RuntimeError("release gate failed")
 if any(v[k]for k in("signed_binary_manifest_count","native_reproduction_count","reviewer_signature_count","user_release_approval_count","release_package_count","promotion_count")):raise RuntimeError("release overclaim")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:write()
 check();print("validated 256 release-blocker cases and 8 truth documents without release qualification");return 0
if __name__=="__main__":raise SystemExit(main())
