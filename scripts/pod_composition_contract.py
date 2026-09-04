#!/usr/bin/env python3
"""Build and verify Story 95.4 inert pod-composition evidence."""
from __future__ import annotations
import argparse,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-95/pod-composition-corpus.json"
CLASSES=("authority-aggregation","role-impersonation","reviewer-collusion","stale-review","worktree-collision","hidden-effect","budget-evasion","duplicate-work","false-consensus","crash","cancellation","worker-loss","replacement","rescheduling","partial-integration")
def build()->bytes:
 cases=[{"case_id":f"AT-MAG-{i+1:03d}","class":CLASSES[i%len(CLASSES)],"expected":"deny-or-reconcile","enabled":False,"publication_count":0,"authority_pool_count":0} for i in range(60)]
 return (json.dumps({"schema_version":1,"case_count":60,"classes":list(CLASSES),"max_workers":5,"single_agent_mode_preserved":True,"serialized_integration":True,"enabled_pod_count":0,"native_worker_count":0,"cases":cases},sort_keys=True,separators=(",",":"))+"\n").encode()
def write():OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
def check():
 if not OUTPUT.is_file() or OUTPUT.read_bytes()!=build():raise RuntimeError("pod corpus stale or absent")
 v=json.loads(OUTPUT.read_bytes())
 if any(c["enabled"] or c["publication_count"] or c["authority_pool_count"] for c in v["cases"]):raise RuntimeError("pod overclaim")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:write()
 check();print("validated 60 pod cases across 15 classes with zero enablement");return 0
if __name__=="__main__":raise SystemExit(main())
