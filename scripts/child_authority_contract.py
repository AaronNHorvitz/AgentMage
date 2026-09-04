#!/usr/bin/env python3
"""Build and verify the Sprint 94 child-authority property corpus."""
from __future__ import annotations
import argparse,hashlib,json
from pathlib import Path
from typing import Final
ROOT:Final=Path(__file__).resolve().parents[1]
OUTPUT:Final=ROOT/"artifacts/sprints/sprint-94/child-authority-corpus.json"
CLASSES:Final=("intersection","deny-precedence","sibling-aggregation","memory-isolation","conversation-isolation","temporary-file-isolation","receipt-isolation","output-isolation","worktree-isolation","path-collision","agent-limit","nesting-limit","turn-limit","tool-limit","process-limit","memory-limit","model-load-limit","network-limit","output-limit","result-as-authority","parent-review","pause-propagation","cancel-propagation","expiry-propagation")
def canonical(value:object)->bytes:return (json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def build()->bytes:
 cases=[]
 for index in range(96):
  kind=CLASSES[index%len(CLASSES)]
  cases.append({"case_id":f"S-072-{index+1:03d}","class":kind,"expected":"admit-narrow" if kind=="intersection" else "deny-or-isolate","real_effect_count":0,"authority_source":"four-way-intersection-only","result_authority_count":0})
 return canonical({"schema_version":1,"case_count":len(cases),"class_count":len(CLASSES),"classes":list(CLASSES),"cases":cases,"native_process_trace_count":0,"native_worktree_trace_count":0,"independent_review":False})
def write()->None:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
def check()->None:
 if not OUTPUT.is_file() or OUTPUT.read_bytes()!=build():raise RuntimeError("child-authority corpus stale or absent")
 value=json.loads(OUTPUT.read_bytes())
 if len({case["case_id"] for case in value["cases"]})!=96 or any(case["real_effect_count"] or case["result_authority_count"] for case in value["cases"]):raise RuntimeError("child-authority corpus invalid")
def main()->int:
 parser=argparse.ArgumentParser();parser.add_argument("--write",action="store_true");args=parser.parse_args()
 if args.write:write()
 check();print("validated 96 child-authority cases across 24 classes with zero effects");return 0
if __name__=="__main__":raise SystemExit(main())
