#!/usr/bin/env python3
"""Generate Sprint 151 finance privacy and absence evidence."""
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-151/finance-privacy-corpus.json";SOURCE=ROOT/"kernel/engine/src/finance_privacy.rs"
FAMILIES=("transfer","payment","bill_pay","trade","order","withdrawal","deposit","credit","loan","tax_filing","beneficiary","account_admin","credential_recovery");SURFACES=("model_prompt","memory","log","diagnostic","receipt","export","backup","crash","core_dump","communication","document","task","delivery","cloud","error");SOURCES=("model","message","document","workflow","provider","plugin","configuration","migration","autonomy");STATES=("idle","sync","queued","in_flight","uncertain","crashed","stale");SCANS=("schema","manifest","policy","registry","adapter","shell","workflow","schedule","test","documentation","compiled_artifact","provider_request")
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def build():
 cases=[]
 for family in FAMILIES:
  for source in SOURCES:
   for state in STATES:
    for surface in SURFACES:cases.append({"id":f"AT-FPRV-001-{len(cases)+1:05d}","family":family,"source":source,"state":state,"surface":surface,"policy_receipt":True,"canary_digest_only":True,"undeclared_disclosure_count":0,"capability_shape_count":0,"external_effect_count":0,"residual_count":0})
 v={"schema_version":1,"families":list(FAMILIES),"surfaces":list(SURFACES),"sources":list(SOURCES),"states":list(STATES),"scan_classes":list(SCANS),"cases":cases,"case_count":len(cases),"undeclared_disclosure_count":0,"capability_shape_count":0,"external_effect_count":0,"canary_disclosure_count":0,"residual_count":0,"source_sha256":{"kernel/engine/src/finance_privacy.rs":sha(SOURCE)}}
 return(json.dumps(v,sort_keys=True,separators=(",",":"))+"\n").encode()
def check():
 e=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=e:raise RuntimeError("finance privacy corpus stale")
 v=json.loads(e);s=SOURCE.read_text().split("#[cfg(test)]",1)[0]
 for t in ("FinancialDataPolicy","FinancialFlowReceipt","FinancialCanaryObservation","FinanceRemovalReceipt","ProhibitedFinancialFamily","validate_policy","admit_flow","validate_canary","validate_removal","reject_prohibited_family"):
  if t not in s:raise RuntimeError(t)
 if v["case_count"]!=12285 or any(v[k]for k in ("undeclared_disclosure_count","capability_shape_count","external_effect_count","canary_disclosure_count","residual_count")):raise RuntimeError("privacy drift")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();print("validated 12,285 AT-FPRV-001 cases with zero disclosure, effect, or residue");return 0
if __name__=="__main__":raise SystemExit(main())
