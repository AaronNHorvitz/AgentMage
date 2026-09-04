#!/usr/bin/env python3
"""Build deterministic Sprint 102 non-GA inherited checkpoint fixtures."""
from __future__ import annotations
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-102/inherited-checkpoint-corpus.json"
DOCUMENTS=("README.md","PRD.md","IMPLEMENTATION-PLAN.md","Agent-Scaffolding-Inventory.md","TASKS.md","SECURITY-REVIEW.md","SECURITY.md","WINDOWS-BOUNDARIES.md")
BLOCKERS=("failed","skipped","stale","unavailable","flaky","suppressed","unreviewed","unreconciled")
def sha(data:bytes)->str:return hashlib.sha256(data).hexdigest()
def build()->bytes:
 registry=json.loads((ROOT/"requirements/registry.json").read_text());trace=json.loads((ROOT/"requirements/traceability-report.json").read_text());status=json.loads((ROOT/"architecture/status-model.json").read_text())
 reports=[]
 for path in sorted((ROOT/"artifacts/sprints").glob("sprint-*/local-evidence-report.json"),key=lambda p:int(p.parent.name.split("-")[1])):
  sprint=int(path.parent.name.split("-")[1])
  if sprint==102:continue
  value=json.loads(path.read_text());reports.append({"sprint":sprint,"status":value["summary"]["sprint_status"],"sha256":sha(path.read_bytes())})
 cases=[{"case_id":f"S-102-C-{index+1:03d}","blocker":BLOCKERS[index%len(BLOCKERS)],"visible":True,"legacy_g_product_closed":False,"g_ga_closed":False,"release_authorized":False,"support_promoted":False}for index in range(256)]
 value={"schema_version":1,"decision_0008_non_ga_checkpoint":True,"requirement_count":len(registry["requirements"]),"traceability_count":len(trace["requirements"]),"completion_record_count":len(reports),"completion_records":reports,"documents":[{"path":path,"sha256":sha((ROOT/path).read_bytes()),"present":True}for path in DOCUMENTS],"blocker_cases":cases,"blocker_case_count":len(cases),"deferred_register_path":"artifacts/sprints/sprint-101/requirement-closure-corpus.json","source_sbom_present":(ROOT/"supply-chain/sbom.cdx.json").is_file(),"current_product_lifecycle":status["current_product"]["lifecycle_status"],"supported_platform_count":sum(x["support_status"]=="supported"for x in status["platforms"]),"enabled_model_count":sum(x["enabled"]for x in status["models"]),"clean_platform_bundle_count":0,"signed_manifest_count":0,"independent_review_count":0,"user_approval_count":0,"legacy_g_product_closed":False,"g_ga_closed":False}
 return(json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def check():
 expected=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=expected:raise RuntimeError("inherited checkpoint corpus stale or absent")
 value=json.loads(expected)
 if value["requirement_count"]!=294 or value["traceability_count"]!=294 or value["completion_record_count"]!=112:raise RuntimeError("inherited checkpoint inventory drift")
 if len(value["documents"])!=8 or any(not x["present"]for x in value["documents"]):raise RuntimeError("checkpoint documentation drift")
 if value["blocker_case_count"]!=256 or any(not x["visible"]or x["legacy_g_product_closed"]or x["g_ga_closed"]or x["release_authorized"]or x["support_promoted"]for x in value["blocker_cases"]):raise RuntimeError("checkpoint blocker visibility drift")
 for key in ("supported_platform_count","enabled_model_count","clean_platform_bundle_count","signed_manifest_count","independent_review_count","user_approval_count"):
  if value[key]!=0:raise RuntimeError(f"checkpoint overclaim: {key}")
 if value["current_product_lifecycle"]!="scaffolded"or value["legacy_g_product_closed"]or value["g_ga_closed"]:raise RuntimeError("checkpoint release truth drift")
def main():
 parser=argparse.ArgumentParser();parser.add_argument("--write",action="store_true");args=parser.parse_args()
 if args.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();print("validated 112 inherited reports and 256 non-GA blocker cases without release authority");return 0
if __name__=="__main__":raise SystemExit(main())
