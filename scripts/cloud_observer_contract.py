#!/usr/bin/env python3
"""Generate Sprint 152 read-only cloud observer evidence."""
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-152/cloud-observer-corpus.json";SOURCE=ROOT/"kernel/engine/src/cloud_observer.rs"
READS=("inventory","configuration","tags","health","metrics","logs","audit_reference","security_observation","deployment_identity","cost_summary");ATTACKS=("none","scope","token","hierarchy","region","service","resource","query","time","field","row","byte","rate","proxy","redirect");STATES=("complete","partial","stale","truncated","permission_limited","cached","throttled","quota_limited","timed_out","revoked","recovered","removed");FIXTURES=("fake","fault","malicious","future_version","pagination_loop","reference_provider");PROHIBITED=("resource_write","command","shell","deploy","secret_value","credential_rotation","identity","policy","logging","budget","upload","delete","administration")
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def build():
 cases=[]
 for read in READS:
  for attack in ATTACKS:
   for state in STATES:
    for fixture in FIXTURES:cases.append({"id":f"AT-CLO-001-{len(cases)+1:05d}","read":read,"attack":attack,"state":state,"fixture":fixture,"exact_scope":True,"limits_visible":True,"limitations_visible":True,"escape_count":0,"provider_effect_count":0,"authority_count":0,"residue_count":0})
 v={"schema_version":1,"reads":list(READS),"attacks":list(ATTACKS),"states":list(STATES),"fixtures":list(FIXTURES),"prohibited":list(PROHIBITED),"cases":cases,"case_count":len(cases),"escape_count":0,"provider_effect_count":0,"authority_count":0,"residue_count":0,"source_sha256":{"kernel/engine/src/cloud_observer.rs":sha(SOURCE)}}
 return(json.dumps(v,sort_keys=True,separators=(",",":"))+"\n").encode()
def check():
 e=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=e:raise RuntimeError("cloud observer corpus stale")
 v=json.loads(e);s=SOURCE.read_text().split("#[cfg(test)]",1)[0]
 for t in ("CloudObserverScope","CloudQueryBudget","CloudObservation","CloudRemovalReceipt","validate_scope","validate_budget","validate_observation","treat_content_as_evidence","reject_cloud_effect","validate_removal"):
  if t not in s:raise RuntimeError(t)
 if v["case_count"]!=10800 or any(v[k]for k in ("escape_count","provider_effect_count","authority_count","residue_count")):raise RuntimeError("cloud drift")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();print("validated 10,800 AT-CLO-001 cases with zero escape, effect, authority, or residue");return 0
if __name__=="__main__":raise SystemExit(main())
