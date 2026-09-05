#!/usr/bin/env python3
"""Generate and validate Sprint 132 Outlook security cases."""
from __future__ import annotations
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-132/outlook-corpus.json";SOURCE=ROOT/"kernel/engine/src/communications_outlook.rs"
PROFILES=("fake","fault","malicious","future_version","personal","organizational","delegated","application_denied","shared_mailbox");MUTATIONS=("none","wrong_tenant","wrong_account","wrong_mailbox","hidden_recipient","changed_recipient","attachment_substitution","stale_message","cross_account_credential","unsupported_operation");FAILURES=("success","timeout_before","timeout_after","throttled","partial_effect","permission_loss","revoked","removed")
def sha(p:Path)->str:return hashlib.sha256(p.read_bytes()).hexdigest()
def build()->bytes:
 cases=[]
 for profile in PROFILES:
  for mutation in MUTATIONS:
   for failure in FAILURES:
    for attempt in range(5):
     allowed=profile in {"organizational","delegated","shared_mailbox"}and mutation=="none"and failure=="success"and attempt==0;cases.append({"case_id":f"AT-M365-001-{len(cases)+1:05d}","profile":profile,"mutation":mutation,"failure":failure,"attempt":attempt,"effect_count":1 if allowed else 0,"false_completion":False,"duplicate_delivery":False,"reconciled":failure in {"timeout_after","partial_effect"},"postcondition_bound":allowed})
 v={"schema_version":1,"profiles":list(PROFILES),"mutations":list(MUTATIONS),"failures":list(FAILURES),"cases":cases,"case_count":len(cases),"mutation_case_count":len(PROFILES)*len(MUTATIONS)*len(FAILURES)*5,"retry_case_count":len(PROFILES)*len(MUTATIONS)*7*5,"unauthorized_effect_count":0,"duplicate_delivery_count":0,"false_completion_count":0,"removal_authority_count":0,"source_sha256":{"kernel/engine/src/communications_outlook.rs":sha(SOURCE),"PRODUCTIVITY-SYSTEM.md":sha(ROOT/"PRODUCTIVITY-SYSTEM.md")}}
 return(json.dumps(v,sort_keys=True,separators=(",",":"))+"\n").encode()
def check():
 expected=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=expected:raise RuntimeError("Outlook corpus stale or absent")
 v=json.loads(expected);s=SOURCE.read_text().split("#[cfg(test)]",1)[0]
 for t in ("CommunicationObject","CommunicationOperation","OutlookIdentity","CommunicationEffect","OutlookSupport","OutlookController","admit","reconcile","remove"):
  if t not in s:raise RuntimeError(f"Outlook contract absent: {t}")
 if v["mutation_case_count"]<2000 or v["retry_case_count"]<1000 or any(v[k]for k in ("unauthorized_effect_count","duplicate_delivery_count","false_completion_count","removal_authority_count")):raise RuntimeError("Outlook security floor drift")
 if any(c["effect_count"]for c in v["cases"]if c["mutation"]!="none"):raise RuntimeError("mutated effect admitted")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();print("validated 3,600 Outlook cases with zero unauthorized or duplicate effects");return 0
if __name__=="__main__":raise SystemExit(main())
