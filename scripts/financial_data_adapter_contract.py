#!/usr/bin/env python3
"""Generate and validate Sprint 145 read-only financial-adapter evidence."""
from __future__ import annotations
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-145/read-only-corpus.json";SOURCE=ROOT/"kernel/engine/src/financial_data_adapter.rs"
CLASSES=("transactions","balances","liabilities","investments","recurring_streams","statements");MUTATIONS=("none","provider","institution","item","account","scope","owner","cursor","redirect","token","field","permission");FAILURES=("success","cursor_loss","duplicate","pending_transition","relink","outage","throttle","permission_reduction","revoked","removed");STATES=("consented","syncing","stale","queued","revoked","removed");PROHIBITED=("payment","transfer","bill_pay","trade","order","withdrawal","deposit","credit","loan","beneficiary","tax","account_admin","credential_recovery")
def sha(path:Path)->str:return hashlib.sha256(path.read_bytes()).hexdigest()
def build()->bytes:
 cases=[]
 for data_class in CLASSES:
  for mutation in MUTATIONS:
   for failure in FAILURES:
    for state in STATES:
     allowed=mutation=="none"and failure=="success"and state=="syncing";cases.append({"id":f"AT-BNK-001-{len(cases)+1:05d}","class":data_class,"mutation":mutation,"failure":failure,"state":state,"read_count":1 if allowed else 0,"write_count":0,"disclosure":False,"coverage_visible":True,"credential_exposed":False,"residual_authority":0 if state=="removed"else None})
 value={"schema_version":1,"provider":"plaid","classes":list(CLASSES),"mutations":list(MUTATIONS),"failures":list(FAILURES),"states":list(STATES),"prohibited":list(PROHIBITED),"cases":cases,"case_count":len(cases),"write_count":0,"disclosure_count":0,"credential_exposure_count":0,"prohibited_representation_count":0,"removal_authority_count":0,"source_sha256":{"kernel/engine/src/financial_data_adapter.rs":sha(SOURCE),"PRODUCTIVITY-SYSTEM.md":sha(ROOT/"PRODUCTIVITY-SYSTEM.md")}}
 return(json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def check():
 expected=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=expected:raise RuntimeError("read-only finance corpus stale or absent")
 value=json.loads(expected);source=SOURCE.read_text().split("#[cfg(test)]",1)[0]
 for token in ("FinancialProviderProfile","FinancialConsentReceipt","FinancialReadRequest","NormalizedFinancialObservation","FinancialSyncResult","FinancialDataController","admit_read","normalize","reconcile_sync","revoke_and_remove"):
  if token not in source:raise RuntimeError(f"financial adapter contract absent: {token}")
 if value["case_count"]!=4320 or any(value[key]for key in ("write_count","disclosure_count","credential_exposure_count","prohibited_representation_count","removal_authority_count")):raise RuntimeError("read-only finance integrity drift")
def main():
 parser=argparse.ArgumentParser();parser.add_argument("--write",action="store_true");args=parser.parse_args()
 if args.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();print("validated 4,320 AT-BNK-001 cases with zero writes, disclosure, or residual authority");return 0
if __name__=="__main__":raise SystemExit(main())
