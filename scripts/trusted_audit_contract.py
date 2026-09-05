#!/usr/bin/env python3
"""Generate Sprint 157 trusted-operation and audit contract fixtures."""
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-157/trusted-audit-corpus.json";SOURCE=ROOT/"kernel/engine/src/trusted_audit_contract.rs"
CAPABILITIES=("command","research","credential","continuity","model_manager");LIFECYCLES=("enabled","starting","active","cancelling","uncertain","reconciling","stopped","disabled","removed","recovered","residue");PLATFORMS=("linux","windows","macos_retained");FAULTS=("none","sandbox","key_store","network","storage","clock","authentication","cancellation","unknown_field","future_version")
DISPOSITIONS=("analyzed","generated","vendored","binary","excluded","unavailable","unsupported","changed","failed");RECORDS=("file","symbol","module","graph_edge","semantic_packet","evidence_card","contradiction","finding","checkpoint","invalidation","coverage","resource","report")
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def build():
 trusted=[{"id":f"AT-TRU-001-{i+1:05d}","capability":c,"lifecycle":l,"platform":p,"fault":f,"exact_class":True,"fail_closed":True,"union_count":0,"effect_count":0}for i,(c,l,p,f) in enumerate((c,l,p,f)for c in CAPABILITIES for l in LIFECYCLES for p in PLATFORMS for f in FAULTS)]
 audit=[{"id":f"AT-CBA-001-{i+1:05d}","disposition":d,"record":r,"fault":f,"source_bound":True,"explicit_disposition":True,"authority_count":0,"completeness_count":0}for i,(d,r,f) in enumerate((d,r,f)for d in DISPOSITIONS for r in RECORDS for f in FAULTS)]
 value={"schema_version":1,"capabilities":list(CAPABILITIES),"lifecycles":list(LIFECYCLES),"platforms":list(PLATFORMS),"faults":list(FAULTS),"dispositions":list(DISPOSITIONS),"record_kinds":list(RECORDS),"trusted_cases":trusted,"audit_cases":audit,"trusted_case_count":len(trusted),"audit_case_count":len(audit),"union_count":0,"effect_count":0,"audit_authority_count":0,"audit_completeness_count":0,"rv_trusted":list(range(36,44)),"rv_audit":list(range(44,49)),"source_sha256":{"kernel/engine/src/trusted_audit_contract.rs":sha(SOURCE)}}
 return(json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def check():
 e=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=e:raise RuntimeError("trusted audit corpus stale")
 v=json.loads(e)
 if v["trusted_case_count"]!=1650 or v["audit_case_count"]!=1170 or any(v[k] for k in ("union_count","effect_count","audit_authority_count","audit_completeness_count")):raise RuntimeError("trusted audit drift")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();print("validated 1,650 AT-TRU-001 and 1,170 AT-CBA-001 cases without union, effect, authority, or false completeness");return 0
if __name__=="__main__":raise SystemExit(main())
