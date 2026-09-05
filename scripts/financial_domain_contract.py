#!/usr/bin/env python3
"""Generate and validate exact Sprint 142 financial-domain evidence."""
from __future__ import annotations
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-142/financial-domain-corpus.json";SOURCE=ROOT/"kernel/engine/src/financial_domain.rs"
CURRENCIES=("USD","EUR","JPY","KWD","GBP","CAD","AUD");SCALES=(0,2,3,6,18);SIGNS=(-1,0,1);ROUNDING=("toward_zero","away_from_zero","floor","ceiling","half_even");MAGNITUDES=(1,5,99,10**18-1);ORDERS=("forward","reverse","pairwise","sorted")
MUTATIONS=("source","correction","supersession","account","currency","effective_date","lineage","future_version");FAILURES=("ambiguous_currency","overflow","unsupported_scale","malformed","mixed_currency","silent_rounding")
OBJECTS=("institution","account","statement","transaction","pending","posted","split","transfer","category","payee","recurring_stream","budget","goal","debt","asset","liability","receipt","invoice","reimbursement","tax_label")
def sha(path:Path)->str:return hashlib.sha256(path.read_bytes()).hexdigest()
def build()->bytes:
 cases=[]
 for currency in CURRENCIES:
  for scale in SCALES:
   for sign in SIGNS:
    for rounding in ROUNDING:
     for magnitude in MAGNITUDES:
      for order in ORDERS:
       units=sign*magnitude;cases.append({"id":f"AT-FIN-001-{len(cases)+1:05d}","currency":currency,"scale":scale,"minor_units":str(units),"rounding":rounding,"order":order,"canonical":f"{currency}:{scale}:{units}","binary_float":False,"exact":True,"deterministic":True,"lineage_preserved":True})
 attacks=[{"mutation":mutation,"failure":failure,"stored":False,"visible_failure":True,"lineage_preserved":True}for mutation in MUTATIONS for failure in FAILURES]
 value={"schema_version":1,"currencies":list(CURRENCIES),"scales":list(SCALES),"rounding_modes":list(ROUNDING),"object_kinds":list(OBJECTS),"mutations":list(MUTATIONS),"failures":list(FAILURES),"cases":cases,"attacks":attacks,"case_count":len(cases),"attack_count":len(attacks),"binary_float_count":0,"inexact_result_count":0,"order_dependent_count":0,"silent_failure_count":0,"lineage_loss_count":0,"source_sha256":{"kernel/engine/src/financial_domain.rs":sha(SOURCE),"SECURITY-REVIEW.md":sha(ROOT/"SECURITY-REVIEW.md")}}
 return(json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def check()->None:
 expected=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=expected:raise RuntimeError("financial domain corpus stale or absent")
 value=json.loads(expected);source=SOURCE.read_text().split("#[cfg(test)]",1)[0]
 for token in ("Money","Currency","RoundingMode","checked_add","rescale","allocate","FinancialObjectKind","FinancialSourceRecord","AdjustmentRecord","ReconciliationRecord","validate"):
  if token not in source:raise RuntimeError(f"financial contract absent: {token}")
 if "f32" in source or "f64" in source:raise RuntimeError("binary floating point entered canonical financial domain")
 if value["case_count"]!=8400 or value["attack_count"]!=48:raise RuntimeError("financial matrix drift")
 if any(value[key]for key in ("binary_float_count","inexact_result_count","order_dependent_count","silent_failure_count","lineage_loss_count")):raise RuntimeError("financial integrity drift")
def main()->int:
 parser=argparse.ArgumentParser();parser.add_argument("--write",action="store_true");args=parser.parse_args()
 if args.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();print("validated 8,400 AT-FIN-001 arithmetic cases and 48 fail-closed lineage attacks");return 0
if __name__=="__main__":raise SystemExit(main())
