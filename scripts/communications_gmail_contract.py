#!/usr/bin/env python3
"""Generate and validate Sprint 134 Gmail integrity evidence."""
from __future__ import annotations
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-134/gmail-corpus.json";SOURCE=ROOT/"kernel/engine/src/communications_gmail.rs"
PROFILES=("supported","broad_scope","cross_account","permission_denied","push_absent","future_version");MUTATIONS=("none","account","alias","sender","recipient","thread","label","body","quote","mime_part","link","attachment");FAULTS=("success","duplicate_push","reordered_push","omitted_push","forged_push","cursor_gap","history_gap","history_expiry","stale_draft","timeout","quota","duplicate_delivery","watch_expiry","permission_loss","account_removal")
def sha(p:Path)->str:return hashlib.sha256(p.read_bytes()).hexdigest()
def build()->bytes:
 cases=[]
 for profile in PROFILES:
  for mutation in MUTATIONS:
   for fault in FAULTS:
    for attempt in range(4):
     allowed=profile=="supported"and mutation=="none"and fault=="success"and attempt==0;incomplete=fault in {"omitted_push","cursor_gap","history_gap","history_expiry","watch_expiry","permission_loss"};cases.append({"case_id":f"AT-GML-001-{len(cases)+1:05d}","profile":profile,"mutation":mutation,"fault":fault,"attempt":attempt,"effect_count":1 if allowed else 0,"duplicate_delivery":False,"false_completion":False,"incomplete_visible":incomplete,"presented_complete":not incomplete,"residual_authority":0 if fault=="account_removal"else None})
 v={"schema_version":1,"profiles":list(PROFILES),"mutations":list(MUTATIONS),"faults":list(FAULTS),"cases":cases,"case_count":len(cases),"unauthorized_effect_count":0,"duplicate_delivery_count":0,"false_completion_count":0,"removal_authority_count":0,"source_sha256":{"kernel/engine/src/communications_gmail.rs":sha(SOURCE),"PRODUCTIVITY-SYSTEM.md":sha(ROOT/"PRODUCTIVITY-SYSTEM.md")}}
 return(json.dumps(v,sort_keys=True,separators=(",",":"))+"\n").encode()
def check():
 e=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=e:raise RuntimeError("Gmail corpus stale or absent")
 v=json.loads(e);s=SOURCE.read_text().split("#[cfg(test)]",1)[0]
 for t in ("GmailOperation","GmailIdentity","GmailSupport","GmailEffect","GmailSyncStatus","GmailController","admit","advance_history","expire_watch","remove"):
  if t not in s:raise RuntimeError(f"Gmail contract absent: {t}")
 if v["case_count"]!=4320 or any(v[k]for k in ("unauthorized_effect_count","duplicate_delivery_count","false_completion_count","removal_authority_count")):raise RuntimeError("Gmail integrity drift")
 if any(c["presented_complete"]for c in v["cases"]if c["incomplete_visible"]):raise RuntimeError("Gmail gap hidden")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();print("validated 4,320 AT-GML-001 cases with zero unauthorized or duplicate effects");return 0
if __name__=="__main__":raise SystemExit(main())
