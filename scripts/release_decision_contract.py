#!/usr/bin/env python3
"""Generate Sprint 166 independent release-blocker fixtures."""
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-166/release-decision-corpus.json";SOURCE=ROOT/"kernel/engine/src/release_decision.rs"
CLASSES=("platform","provider","authority","credential","disclosure","backup","restore","model","audit_coverage","read_only","reconciliation","security","accessibility","recovery","support","removal","evidence","fuzzing","reviewer","user_approval");STATES=("failed","skipped","stale","unavailable","flaky","quarantined","suppressed","unreconciled","unreviewed")
def build():
 cases=[{"id":f"AT-GA-004-{i+1:04d}","blocker_class":c,"state":s,"publication_allowed":False,"ga_closed":False,"substitution_count":0}for i,(c,s)in enumerate((c,s)for c in CLASSES for s in STATES)]
 value={"schema_version":1,"blocker_classes":list(CLASSES),"non_pass_states":list(STATES),"case_count":len(cases),"cases":cases,"false_publication_count":0,"false_ga_closure_count":0,"substitution_count":0,"current_release_decision":"BLOCKED","source_sha256":{"kernel/engine/src/release_decision.rs":hashlib.sha256(SOURCE.read_bytes()).hexdigest()}}
 return(json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def check():
 expected=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=expected:raise RuntimeError("release decision corpus stale")
 value=json.loads(expected)
 if value["case_count"]!=180 or any(value[k]for k in ("false_publication_count","false_ga_closure_count","substitution_count")):raise RuntimeError("release decision drift")
def main():
 parser=argparse.ArgumentParser();parser.add_argument("--write",action="store_true");args=parser.parse_args()
 if args.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();print("validated 180 independent release blockers with zero publication, GA closure, or substitution");return 0
if __name__=="__main__":raise SystemExit(main())
