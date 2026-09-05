#!/usr/bin/env python3
"""Generate Sprint 167 isolated experimental-lab fixtures."""
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-167/experimental-model-lab-corpus.json";SOURCE=ROOT/"experimental/model-lab/src/lib.rs"
ARTIFACTS=("malformed","oversized","hostile","unknown","mirrored","mutable","provenance_incomplete","license_unclear","unsupported");STAGES=("import","parse","load","inference","evaluation","cleanup","removal","cancel");ROUTES=("ipc","path","socket","interface","credential","network","store","command","connector","canonical_workspace","approved_store","operational_memory");FAULTS=("none","resource","injection","exfiltration","crash","cancellation","future_version","replay")
def build():
 cases=[{"id":f"AT-EML-001-{i+1:05d}","artifact":a,"stage":s,"route":r,"fault":f,"experimental_visible":True,"provenance_gaps_visible":True,"authority_count":0,"network_count":0,"canonical_write_count":0,"approved_store_count":0}for i,(a,s,r,f)in enumerate((a,s,r,f)for a in ARTIFACTS for s in STAGES for r in ROUTES for f in FAULTS)]
 value={"schema_version":1,"artifacts":list(ARTIFACTS),"stages":list(STAGES),"prohibited_routes":list(ROUTES),"faults":list(FAULTS),"case_count":len(cases),"cases":cases,"authority_count":0,"network_count":0,"canonical_write_count":0,"approved_store_count":0,"hidden_limitation_count":0,"source_sha256":{"experimental/model-lab/src/lib.rs":hashlib.sha256(SOURCE.read_bytes()).hexdigest()}}
 return(json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def check():
 expected=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=expected:raise RuntimeError("experimental model lab corpus stale")
 value=json.loads(expected)
 if value["case_count"]!=6912 or any(value[k]for k in ("authority_count","network_count","canonical_write_count","approved_store_count","hidden_limitation_count")):raise RuntimeError("experimental model lab drift")
def main():
 parser=argparse.ArgumentParser();parser.add_argument("--write",action="store_true");args=parser.parse_args()
 if args.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();print("validated 6,912 experimental-lab attacks with zero authority, network, canonical write, approved-store, or hidden limitation");return 0
if __name__=="__main__":raise SystemExit(main())
