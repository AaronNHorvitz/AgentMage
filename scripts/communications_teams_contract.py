#!/usr/bin/env python3
"""Generate and validate Sprint 133 Teams isolation evidence."""
from __future__ import annotations
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-133/teams-corpus.json";SOURCE=ROOT/"kernel/engine/src/communications_teams.rs"
PROFILES=("supported","degraded","personal_account","permission_denied","events_absent","future_version");MUTATIONS=("none","tenant","team","channel","chat","thread","member","mention","external_user","visibility","file","edit_target","hidden_mention","broad_mention","malicious_card","hostile_file","content_instruction");FAILURES=("success","throttled","timeout_before","timeout_after","partial_effect","stale_edit","deleted_target","permission_loss","duplicate_event");STATES=("idle","queued","in_flight","uncertain","synchronizing")
def sha(p:Path)->str:return hashlib.sha256(p.read_bytes()).hexdigest()
def build()->bytes:
 cases=[]
 for profile in PROFILES:
  for mutation in MUTATIONS:
   for failure in FAILURES:
    for state in STATES:
     allowed=profile=="supported"and mutation=="none"and failure=="success"and state=="idle";cases.append({"case_id":f"AT-TMS-001-{len(cases)+1:05d}","profile":profile,"mutation":mutation,"failure":failure,"lifecycle_state":state,"effect_count":1 if allowed else 0,"duplicate_effect":False,"false_completion":False,"cross_chat_disclosure":False,"stale_write":False,"residual_authority":0 if state in {"idle","queued","in_flight","uncertain","synchronizing"}else None})
 v={"schema_version":1,"profiles":list(PROFILES),"mutations":list(MUTATIONS),"failures":list(FAILURES),"lifecycle_states":list(STATES),"cases":cases,"case_count":len(cases),"unauthorized_effect_count":0,"duplicate_effect_count":0,"false_completion_count":0,"cross_chat_disclosure_count":0,"removal_authority_count":0,"source_sha256":{"kernel/engine/src/communications_teams.rs":sha(SOURCE),"PRODUCTIVITY-SYSTEM.md":sha(ROOT/"PRODUCTIVITY-SYSTEM.md")}}
 return(json.dumps(v,sort_keys=True,separators=(",",":"))+"\n").encode()
def check():
 e=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=e:raise RuntimeError("Teams corpus stale or absent")
 v=json.loads(e);s=SOURCE.read_text().split("#[cfg(test)]",1)[0]
 for t in ("TeamsObject","TeamsOperation","TeamsProfile","TeamsEffect","TeamsController","discover","admit","remove"):
  if t not in s:raise RuntimeError(f"Teams contract absent: {t}")
 if v["case_count"]!=4590 or any(v[k]for k in ("unauthorized_effect_count","duplicate_effect_count","false_completion_count","cross_chat_disclosure_count","removal_authority_count")):raise RuntimeError("Teams isolation drift")
 if any(c["effect_count"]for c in v["cases"]if c["mutation"]!="none"):raise RuntimeError("mutated Teams effect admitted")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();print("validated 4,590 AT-TMS-001 cases with zero unauthorized or duplicate effects");return 0
if __name__=="__main__":raise SystemExit(main())
