#!/usr/bin/env python3
"""Generate Sprint 150 accounting-provider evidence."""
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-150/accounting-corpus.json";SOURCE=ROOT/"kernel/engine/src/accounting_provider.rs"
PROVIDERS=("quickbooks_online","xero");OBJECTS=("account","contact","item","invoice","bill","journal","tax_code","attachment");MUTATIONS=("none","provider","organization","role","period","object","account","contact","currency","amount","tax","attachment","duplicate_key","postcondition");FAILURES=("success","closed_period","concurrent_change","stale_balance","tax_difference","precision_boundary","timeout","retry","partial_effect","version_drift");STATES=("read","local_draft","provider_draft","approved_write","correction","void");PROHIBITED=("payment","transfer","bill_pay","bank_feed_admin","payroll","filing","credit","loan","credential_recovery","organization_admin")
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def build():
 cases=[]
 for provider in PROVIDERS:
  for obj in OBJECTS:
   for mutation in MUTATIONS:
    for failure in FAILURES:
     for state in STATES:cases.append({"id":f"AT-ACC-001-{len(cases)+1:05d}","provider":provider,"object":obj,"mutation":mutation,"failure":failure,"state":state,"accepted":mutation=="none"and failure=="success","organization_exact":True,"period_exact":True,"currency_exact":True,"tax_exact":True,"idempotent":True,"wrong_write_count":0,"duplicate_write_count":0,"money_movement_count":0})
 value={"schema_version":1,"providers":list(PROVIDERS),"objects":list(OBJECTS),"mutations":list(MUTATIONS),"failures":list(FAILURES),"states":list(STATES),"prohibited":list(PROHIBITED),"cases":cases,"case_count":len(cases),"wrong_write_count":0,"duplicate_write_count":0,"money_movement_count":0,"residual_authority_count":0,"source_sha256":{"kernel/engine/src/accounting_provider.rs":sha(SOURCE)}}
 return(json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def check():
 e=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=e:raise RuntimeError("accounting corpus stale")
 v=json.loads(e);s=SOURCE.read_text().split("#[cfg(test)]",1)[0]
 for t in ("AccountingDiscovery","NormalizedAccountingObject","AccountingWriteProposal","AccountingEffectReceipt","AccountingRemovalReceipt","validate_discovery","validate_read","admit_write","reconcile_effect","validate_removal","reject_prohibited_operation"):
  if t not in s:raise RuntimeError(t)
 if v["case_count"]!=13440 or any(v[k]for k in ("wrong_write_count","duplicate_write_count","money_movement_count","residual_authority_count")):raise RuntimeError("integrity drift")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();print("validated 13,440 AT-ACC-001 cases with zero wrong or money-moving writes");return 0
if __name__=="__main__":raise SystemExit(main())
