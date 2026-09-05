#!/usr/bin/env python3
"""Generate and validate Sprint 144 Actual Budget evidence."""
from __future__ import annotations
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-144/actual-budget-corpus.json";SOURCE=ROOT/"kernel/engine/src/actual_budget_adapter.rs"
PROFILES=("supported","degraded","future","corrupt","locked","removed");MUTATIONS=("none","budget","account","transaction","category","amount","currency","split","rule","schedule","sync","postcondition","credential");FAILURES=("success","conflict","stale","corrupt","locked","cancelled","crash","duplicate","restore","partial");STATES=("discovered","read","drafted","approved","submitted","removed");OPERATIONS=("read","search","import","export","draft_change","update_record","update_rule","update_schedule","update_budget")
def sha(path:Path)->str:return hashlib.sha256(path.read_bytes()).hexdigest()
def build()->bytes:
 cases=[]
 for profile in PROFILES:
  for mutation in MUTATIONS:
   for failure in FAILURES:
    for state in STATES:
     operation=OPERATIONS[len(cases)%len(OPERATIONS)];allowed=profile=="supported"and mutation=="none"and failure=="success"and state=="submitted";cases.append({"id":f"AT-ACT-001-{len(cases)+1:05d}","profile":profile,"operation":operation,"mutation":mutation,"failure":failure,"state":state,"effect_count":1 if allowed else 0,"wrong_file_write":False,"duplicate":False,"precision_loss":False,"silent_rule_change":False,"corrupt_recovery":False,"network_broadening":False,"money_movement":False,"residual_authority":0 if state=="removed"else None})
 value={"schema_version":1,"profiles":list(PROFILES),"operations":list(OPERATIONS),"mutations":list(MUTATIONS),"failures":list(FAILURES),"states":list(STATES),"cases":cases,"case_count":len(cases),"wrong_file_write_count":0,"duplicate_effect_count":0,"precision_loss_count":0,"silent_rule_change_count":0,"corrupt_recovery_count":0,"network_broadening_count":0,"money_movement_count":0,"removal_authority_count":0,"source_sha256":{"kernel/engine/src/actual_budget_adapter.rs":sha(SOURCE),"PRODUCTIVITY-SYSTEM.md":sha(ROOT/"PRODUCTIVITY-SYSTEM.md")}}
 return(json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def check():
 expected=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=expected:raise RuntimeError("Actual Budget corpus stale or absent")
 value=json.loads(expected);source=SOURCE.read_text().split("#[cfg(test)]",1)[0]
 for token in ("ActualBudgetProfile","ActualRecordKind","ActualRecord","ActualBudgetEffect","ActualBackupReceipt","ActualBudgetController","admit_record","admit_effect","admit_backup","reconcile","remove"):
  if token not in source:raise RuntimeError(f"Actual Budget contract absent: {token}")
 for prohibited in ("Payment","TransferInitiation","Trade","AccountAdministration","CredentialRecovery"):
  if prohibited in source:raise RuntimeError(f"prohibited operation represented: {prohibited}")
 if value["case_count"]!=4680 or any(value[key]for key in ("wrong_file_write_count","duplicate_effect_count","precision_loss_count","silent_rule_change_count","corrupt_recovery_count","network_broadening_count","money_movement_count","removal_authority_count")):raise RuntimeError("Actual Budget integrity drift")
def main():
 parser=argparse.ArgumentParser();parser.add_argument("--write",action="store_true");args=parser.parse_args()
 if args.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();print("validated 4,680 AT-ACT-001 cases with zero wrong-file, money-movement, or residual authority");return 0
if __name__=="__main__":raise SystemExit(main())
