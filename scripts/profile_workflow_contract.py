#!/usr/bin/env python3
"""Build and verify inert Sprint 95 profile-workflow fixtures."""
from __future__ import annotations
import argparse,json
from pathlib import Path
from typing import Final
ROOT:Final=Path(__file__).resolve().parents[1]
OUTPUT:Final=ROOT/"artifacts/sprints/sprint-95/profile-workflow-corpus.json"
GRAPHS:Final={"idea-to-work":["AG-25","AG-26","AG-28","AG-05","AG-27","AG-30"],"bug-to-change":["AG-03","AG-06","AG-42","AG-07","AG-08","AG-09","AG-43","AG-14"],"pull-request-review":["AG-35","AG-36","AG-37","AG-38","AG-39","AG-40","AG-41","AG-11"],"roadmap-reconciliation":["AG-29","AG-30","AG-31","AG-32","AG-33","AG-34"]}
CASES:Final=("idea-refinement","incomplete-issue","ambiguous-criteria","duplicate-bug","irreproducible-failure","conflicting-review","stale-plan","failed-validation","cancellation","resource-exhaustion","authority-aggregation","sibling-state-access","implementer-self-review","reviewer-sharing","destructive-conflict","hidden-command","hidden-write","hidden-network","result-as-authority","false-completion")
def canonical(value:object)->bytes:return (json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def build()->bytes:
 graphs=[]
 for name,profiles in GRAPHS.items():
  graphs.append({"workflow_id":name,"schema_version":1,"profiles":profiles,"node_count":len(profiles),"shared_runtime":True,"parent_review_required":True,"provider_effect_count":0,"completion":"deterministic-predicate","cancellation":"descendant-stop-required","retry":"classified-fresh-attempt-only"})
 cases=[{"case_id":f"S-095-{index+1:03d}","class":CASES[index%len(CASES)],"workflow_id":tuple(GRAPHS)[index%4],"expected":"blocked-or-authority-free-proposal","real_effect_count":0,"approval_count":0} for index in range(80)]
 return canonical({"schema_version":1,"workflow_count":4,"case_count":80,"graphs":graphs,"cases":cases,"dissent_preserved":True,"native_descendant_cleanup_count":0,"native_worktree_pipeline_count":0,"independent_review":False})
def write()->None:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
def check()->None:
 if not OUTPUT.is_file() or OUTPUT.read_bytes()!=build():raise RuntimeError("profile workflow corpus stale or absent")
 value=json.loads(OUTPUT.read_bytes())
 if len({case["case_id"] for case in value["cases"]})!=80 or any(case["real_effect_count"] or case["approval_count"] for case in value["cases"]):raise RuntimeError("profile workflow corpus invalid")
def main()->int:
 parser=argparse.ArgumentParser();parser.add_argument("--write",action="store_true");args=parser.parse_args()
 if args.write:write()
 check();print("validated 4 profile workflows and 80 zero-effect cases");return 0
if __name__=="__main__":raise SystemExit(main())
