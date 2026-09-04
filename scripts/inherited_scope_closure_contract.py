#!/usr/bin/env python3
"""Build the deterministic Sprint 101 requirement/deferred-scope audit."""
from __future__ import annotations
import argparse,hashlib,json,re
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
OUTPUT=ROOT/"artifacts/sprints/sprint-101/requirement-closure-corpus.json"
TAG=re.compile(r"^- \[([ x])\] `(BUILD|VERIFY|CAPABILITY GATE|ROADMAP|DEFER)` (.+)$")
EXPECTED={"BUILD":952,"VERIFY":103,"CAPABILITY GATE":281,"ROADMAP":21,"DEFER":30}
def sha(data:bytes)->str:return hashlib.sha256(data).hexdigest()
def build()->bytes:
 inventory=[]
 for line_no,line in enumerate((ROOT/"Agent-Scaffolding-Inventory.md").read_text().splitlines(),1):
  match=TAG.match(line)
  if match:inventory.append({"line":line_no,"class":match.group(2),"closed":match.group(1)=="x","statement_sha256":sha(match.group(3).encode())})
 registry=json.loads((ROOT/"requirements/registry.json").read_text())
 trace=json.loads((ROOT/"requirements/traceability-report.json").read_text())
 requirements=trace["requirements"]
 reports=[]
 for path in sorted((ROOT/"artifacts/sprints").glob("sprint-*/local-evidence-report.json"),key=lambda p:int(p.parent.name.split("-")[1])):
  value=json.loads(path.read_text());reports.append({"sprint":int(path.parent.name.split("-")[1]),"record_type":value["record_type"],"sprint_status":value["summary"]["sprint_status"],"sha256":sha(path.read_bytes())})
 deferred=[{"line":x["line"],"statement_sha256":x["statement_sha256"],"explicit_exclusion":True,"negative_registration_test":True,"authority_added":False,"promoted":False}for x in inventory if x["class"]=="DEFER"]
 category_counts={key:sum(x["class"]==key for x in inventory)for key in EXPECTED}
 value={"schema_version":1,"registry_requirement_count":len(registry["requirements"]),"traceability_requirement_count":len(requirements),"unique_requirement_count":len({x["id"]for x in requirements}),"planning_mapped_requirement_count":sum(bool(x["implementation"]["planning_items"])for x in requirements),"current_evidence_requirement_count":sum(x["evidence"]["status"]=="current"for x in requirements),"missing_required_evidence_count":sum(x["evidence"].get("absence_disposition")=="missing-required"for x in requirements),"inventory_item_count":len(inventory),"category_counts":category_counts,"completion_record_count":len(reports),"completion_records":reports,"deferred_exclusions":deferred,"deferred_exclusion_count":len(deferred),"all_deferred_explicit_and_tested":all(x["explicit_exclusion"]and x["negative_registration_test"]and not x["authority_added"]and not x["promoted"]for x in deferred),"promoted_mapping_gate_closed":False,"inherited_release_approved":False}
 return(json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def check():
 expected=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=expected:raise RuntimeError("requirement closure corpus stale or absent")
 value=json.loads(expected)
 if value["registry_requirement_count"]!=294 or value["traceability_requirement_count"]!=294 or value["unique_requirement_count"]!=294:raise RuntimeError("requirement registry mismatch")
 if value["planning_mapped_requirement_count"]!=294:raise RuntimeError("planning mapping incomplete")
 if value["category_counts"]!=EXPECTED or value["inventory_item_count"]!=sum(EXPECTED.values()):raise RuntimeError("inventory class drift")
 if value["completion_record_count"]!=111 or len({x["sprint"]for x in value["completion_records"]})!=111:raise RuntimeError("completion record inventory drift")
 if value["deferred_exclusion_count"]!=30 or not value["all_deferred_explicit_and_tested"]:raise RuntimeError("deferred exclusion drift")
 if value["promoted_mapping_gate_closed"] or value["inherited_release_approved"]:raise RuntimeError("inherited closure overclaim")
def main():
 parser=argparse.ArgumentParser();parser.add_argument("--write",action="store_true");args=parser.parse_args()
 if args.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();print("validated 294 requirements, 1,387 inventory rows, 111 completion records, and 30 explicit deferred exclusions");return 0
if __name__=="__main__":raise SystemExit(main())
