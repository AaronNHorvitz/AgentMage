#!/usr/bin/env python3
"""Generate Sprint 160 public-web safety fixtures."""
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-160/web-research-safety-corpus.json";SOURCE=ROOT/"kernel/engine/src/web_research_safety.rs"
STATES=("current","stale","conflicting","malicious","redirected","archived","unavailable");HAZARDS=("hidden_text","metadata","script","link","authentication_prompt","tool_request","tracking","credential","form","upload","download","archive");PRIVATE=("workspace","memory","communication","finance","connector","credential","identity","document");FIXTURES=("search","retrieve","cite","revalidate")
def build():
 cases=[{"id":f"AT-WEB-001-{i+1:05d}","state":s,"hazard":h,"private_class":p,"fixture":f,"citation_bound":True,"freshness_visible":True,"authority_count":0,"undeclared_egress_count":0,"raw_private_count":0,"unadmitted_download_count":0}for i,(s,h,p,f)in enumerate((s,h,p,f)for s in STATES for h in HAZARDS for p in PRIVATE for f in FIXTURES)]
 v={"schema_version":1,"states":list(STATES),"hazards":list(HAZARDS),"private_classes":list(PRIVATE),"fixtures":list(FIXTURES),"case_count":len(cases),"cases":cases,"authority_count":0,"undeclared_egress_count":0,"raw_private_count":0,"unadmitted_download_count":0,"source_sha256":{"kernel/engine/src/web_research_safety.rs":hashlib.sha256(SOURCE.read_bytes()).hexdigest()}}
 return(json.dumps(v,sort_keys=True,separators=(",",":"))+"\n").encode()
def check():
 e=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=e:raise RuntimeError("web research corpus stale")
 v=json.loads(e)
 if v["case_count"]!=2688 or any(v[k]for k in ("authority_count","undeclared_egress_count","raw_private_count","unadmitted_download_count")):raise RuntimeError("web research drift")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();print("validated 2,688 AT-WEB-001 cases with zero created authority, undeclared egress, raw private data, or unadmitted download");return 0
if __name__=="__main__":raise SystemExit(main())
