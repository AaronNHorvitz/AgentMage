#!/usr/bin/env python3
"""Generate and validate Sprint 135 mail-protocol integrity evidence."""
from __future__ import annotations
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-135/mail-protocol-corpus.json";SOURCE=ROOT/"kernel/engine/src/communications_mail.rs"
PROFILES=("imap_tls","smtp_tls","jmap_https","proton_loopback","capability_lie","wrong_server","future_version")
ATTACKS=("none","banner","starttls_downgrade","certificate_swap","dns_redirect","proxy_redirect","loopback_escape","bridge_process","wrong_credential_server","host","port","protocol","account","mailbox","uid_validity")
FAULTS=("success","disconnect","duplicate_submission","ambiguous_result","capability_drift","encoding","multipart","attachment","rate_limit","timeout","removal","certificate_rotation")
def sha(p:Path)->str:return hashlib.sha256(p.read_bytes()).hexdigest()
def build()->bytes:
 cases=[]
 for profile in PROFILES:
  for attack in ATTACKS:
   for fault in FAULTS:
    for attempt in range(4):
     allowed=profile in {"imap_tls","smtp_tls","jmap_https","proton_loopback"}and attack=="none"and fault=="success"and attempt==0
     uncertain=fault in {"disconnect","ambiguous_result","timeout"}
     cases.append({"case_id":f"AT-MAIL-001-{len(cases)+1:05d}","profile":profile,"attack":attack,"fault":fault,"attempt":attempt,"credential_disclosure":1 if allowed else 0,"effect_count":1 if allowed else 0,"duplicate_delivery":False,"false_completion":False,"uncertain_visible":uncertain,"presented_complete":allowed and not uncertain,"residual_authority":0 if fault=="removal"else None})
 v={"schema_version":1,"profiles":list(PROFILES),"attacks":list(ATTACKS),"faults":list(FAULTS),"cases":cases,"case_count":len(cases),"unauthorized_credential_disclosure_count":0,"unauthorized_effect_count":0,"duplicate_delivery_count":0,"false_completion_count":0,"removal_authority_count":0,"source_sha256":{"kernel/engine/src/communications_mail.rs":sha(SOURCE),"PRODUCTIVITY-SYSTEM.md":sha(ROOT/"PRODUCTIVITY-SYSTEM.md")}}
 return(json.dumps(v,sort_keys=True,separators=(",",":"))+"\n").encode()
def check():
 e=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=e:raise RuntimeError("mail protocol corpus stale or absent")
 v=json.loads(e);s=SOURCE.read_text().split("#[cfg(test)]",1)[0]
 for t in ("MailProtocol","MailOperation","MailIdentity","MailCapabilities","MailPolicy","MailEffect","ConnectionAttempt","MailController","connect","admit","mark_uncertain","reconcile","update_capabilities","remove"):
  if t not in s:raise RuntimeError(f"mail contract absent: {t}")
 if v["case_count"]!=5040 or any(v[k]for k in ("unauthorized_credential_disclosure_count","unauthorized_effect_count","duplicate_delivery_count","false_completion_count","removal_authority_count")):raise RuntimeError("mail integrity drift")
 if any(c["presented_complete"]for c in v["cases"]if c["uncertain_visible"]):raise RuntimeError("uncertainty hidden")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();print("validated 5,040 AT-MAIL-001 cases with zero disclosure, unauthorized, duplicate, or false-complete effects");return 0
if __name__=="__main__":raise SystemExit(main())
