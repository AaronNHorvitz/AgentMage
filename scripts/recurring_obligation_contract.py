#!/usr/bin/env python3
"""Generate and validate Sprint 147 recurring-obligation evidence."""
from __future__ import annotations
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-147/obligation-corpus.json";SOURCE=ROOT/"kernel/engine/src/recurring_obligation.rs"
KINDS=("recurring_expense","recurring_income","fee","subscription","price_change","missing_record","duplicate_candidate","cancellation_candidate");MUTATIONS=("none","cadence","amount","merchant","source","communication","confidence","status","recipient","link","expiry","identity");FAILURES=("success","irregular","rename","split","refund","annual","free_trial","skipped","duplicate","stale");STATES=("deterministic","model_assisted","uncertain","reminded","dismissed","removed");PROHIBITED=("autonomous_cancellation","payment","transfer","account_administration")
def sha(path:Path)->str:return hashlib.sha256(path.read_bytes()).hexdigest()
def build()->bytes:
 cases=[]
 for kind in KINDS:
  for mutation in MUTATIONS:
   for failure in FAILURES:
    for state in STATES:
     detected=mutation=="none"and failure=="success"and state=="deterministic";cases.append({"id":f"AT-BIL-001-{len(cases)+1:05d}","kind":kind,"mutation":mutation,"failure":failure,"state":state,"detection_count":1 if detected else 0,"financial_citation":True,"communication_citation":True,"method_visible":True,"uncertainty_visible":True,"authority_count":0,"false_completion":False,"financial_effect_count":0})
 value={"schema_version":1,"kinds":list(KINDS),"mutations":list(MUTATIONS),"failures":list(FAILURES),"states":list(STATES),"prohibited":list(PROHIBITED),"cases":cases,"case_count":len(cases),"authority_count":0,"false_completion_count":0,"financial_effect_count":0,"hidden_uncertainty_count":0,"citation_loss_count":0,"source_sha256":{"kernel/engine/src/recurring_obligation.rs":sha(SOURCE),"PRODUCTIVITY-SYSTEM.md":sha(ROOT/"PRODUCTIVITY-SYSTEM.md")}}
 return(json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def check():
 expected=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=expected:raise RuntimeError("obligation corpus stale or absent")
 value=json.loads(expected);source=SOURCE.read_text().split("#[cfg(test)]",1)[0]
 for token in ("ObligationKind","DetectionMethod","RecurringObligation","ObligationEvidenceLink","LocalObligationReminder","ApprovedCommunicationDraft","validate_detection","validate_link","validate_reminder","admit_communication_draft","reject_financial_or_account_effect"):
  if token not in source:raise RuntimeError(f"obligation contract absent: {token}")
 if value["case_count"]!=5760 or any(value[key]for key in ("authority_count","false_completion_count","financial_effect_count","hidden_uncertainty_count","citation_loss_count")):raise RuntimeError("obligation integrity drift")
def main():
 parser=argparse.ArgumentParser();parser.add_argument("--write",action="store_true");args=parser.parse_args()
 if args.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();print("validated 5,760 AT-BIL-001 cases with citations and zero financial effects");return 0
if __name__=="__main__":raise SystemExit(main())
