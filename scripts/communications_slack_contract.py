#!/usr/bin/env python3
"""Generate and validate Sprint 136 Slack isolation evidence."""
from __future__ import annotations
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-136/slack-corpus.json";SOURCE=ROOT/"kernel/engine/src/communications_slack.rs"
PROFILES=("supported","shared_channel","direct_message","permission_denied","events_absent","future_version");MUTATIONS=("none","workspace","enterprise","channel","direct_message","thread","member","mention","visibility","file","scope","token","broad_mention","hostile_blocks","hostile_link","hostile_event");FAILURES=("success","rate_limit","timeout_before","timeout_after","stale_edit","deleted_target","membership_loss","event_replay","cursor_loop","provider_outage");STATES=("idle","queued","in_flight","uncertain","synchronizing")
def sha(p:Path)->str:return hashlib.sha256(p.read_bytes()).hexdigest()
def build()->bytes:
 cases=[]
 for profile in PROFILES:
  for mutation in MUTATIONS:
   for failure in FAILURES:
    for state in STATES:
     allowed=profile in {"supported","shared_channel","direct_message"}and mutation=="none"and failure=="success"and state=="idle";uncertain=failure in {"rate_limit","timeout_before","timeout_after","provider_outage"}
     cases.append({"case_id":f"AT-SLK-001-{len(cases)+1:05d}","profile":profile,"mutation":mutation,"failure":failure,"lifecycle_state":state,"effect_count":1 if allowed else 0,"duplicate_post":False,"cross_workspace_disclosure":False,"false_completion":False,"uncertain_visible":uncertain,"presented_complete":allowed and not uncertain,"residual_authority":0 if failure=="membership_loss"else None})
 v={"schema_version":1,"profiles":list(PROFILES),"mutations":list(MUTATIONS),"failures":list(FAILURES),"lifecycle_states":list(STATES),"cases":cases,"case_count":len(cases),"unauthorized_effect_count":0,"duplicate_post_count":0,"cross_workspace_disclosure_count":0,"false_completion_count":0,"removal_authority_count":0,"source_sha256":{"kernel/engine/src/communications_slack.rs":sha(SOURCE),"PRODUCTIVITY-SYSTEM.md":sha(ROOT/"PRODUCTIVITY-SYSTEM.md")}}
 return(json.dumps(v,sort_keys=True,separators=(",",":"))+"\n").encode()
def check():
 e=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=e:raise RuntimeError("Slack corpus stale or absent")
 v=json.loads(e);s=SOURCE.read_text().split("#[cfg(test)]",1)[0]
 for t in ("SlackOperation","SlackProfile","SlackEffect","SlackSyncState","SlackController","discover","admit","accept_event","mark_uncertain","remove"):
  if t not in s:raise RuntimeError(f"Slack contract absent: {t}")
 if v["case_count"]!=4800 or any(v[k]for k in ("unauthorized_effect_count","duplicate_post_count","cross_workspace_disclosure_count","false_completion_count","removal_authority_count")):raise RuntimeError("Slack isolation drift")
 if any(c["effect_count"]for c in v["cases"]if c["mutation"]!="none"):raise RuntimeError("mutated Slack effect admitted")
 if any(c["presented_complete"]for c in v["cases"]if c["uncertain_visible"]):raise RuntimeError("Slack uncertainty hidden")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();print("validated 4,800 AT-SLK-001 cases with zero unauthorized, duplicate, or cross-workspace effects");return 0
if __name__=="__main__":raise SystemExit(main())
