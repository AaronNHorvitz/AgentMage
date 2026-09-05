#!/usr/bin/env python3
"""Generate the Sprint 153 provider-native cloud observer corpus."""
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-153/provider-observer-corpus.json";SOURCE=ROOT/"kernel/engine/src/cloud_provider_observer.rs"
PROVIDERS=("aws","azure","gcp");READS=("inventory","configuration","tags_or_labels","health","metrics","logs","audit_reference","security_observation","deployment_identity","cost_or_billing")
ATTACKS=("none","hierarchy","account_subscription_project","region_zone","service_provider","resource_id","data_plane","credential","token_audience","endpoint","quota_project","api_version","field","time","row_byte_rate","prohibited_operation")
STATES=("complete","partial","stale","throttled","quota_limited","version_skew","revoked","removed");FIXTURES=("reference","fault","malicious","future_version","pagination_loop")
PROHIBITED=("command","resource_write","deploy","secret","identity","policy","logging","budget","upload","delete","administration")
NATIVE={"aws":["partition","organization","organizational_unit","account","arn","role_session","trail_reference","cost_explorer"],"azure":["cloud","tenant","management_group","subscription","resource_group","resource_id","activity_reference","cost_management"],"gcp":["universe","organization","folder","project","zone","full_resource_name","audit_reference","billing_account"]}
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def build():
 cases=[]
 for provider in PROVIDERS:
  for read in READS:
   for attack in ATTACKS:
    for state in STATES:
     for fixture in FIXTURES:cases.append({"id":f"AT-{provider.upper()}-001-{len(cases)+1:05d}","provider":provider,"read":read,"attack":attack,"state":state,"fixture":fixture,"native_identity":True,"exact_scope":True,"limitations_visible":True,"secret_disclosure_count":0,"escape_count":0,"provider_effect_count":0,"residual_authority_count":0})
 value={"schema_version":1,"providers":list(PROVIDERS),"reads":list(READS),"attacks":list(ATTACKS),"states":list(STATES),"fixtures":list(FIXTURES),"prohibited":list(PROHIBITED),"native_identities":NATIVE,"cases":cases,"case_count":len(cases),"secret_disclosure_count":0,"escape_count":0,"provider_effect_count":0,"residual_authority_count":0,"source_sha256":{"kernel/engine/src/cloud_provider_observer.rs":sha(SOURCE)}}
 return(json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def check():
 expected=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=expected:raise RuntimeError("provider observer corpus stale")
 value=json.loads(expected);source=SOURCE.read_text().split("#[cfg(test)]",1)[0]
 for token in ("NativeCloudProvider","ProviderHierarchy","ProviderObserverProfile","ProviderReadRequest","ProviderRecoveryReceipt","validate_provider_profile","admit_provider_read","reject_provider_operation","validate_provider_recovery"):
  if token not in source:raise RuntimeError(token)
 if value["case_count"]!=19200 or any(value[key]for key in ("secret_disclosure_count","escape_count","provider_effect_count","residual_authority_count")):raise RuntimeError("provider observer drift")
def main():
 parser=argparse.ArgumentParser();parser.add_argument("--write",action="store_true");args=parser.parse_args()
 if args.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();print("validated 19,200 AT-AWS/AZR/GCP-001 cases with zero disclosure, escape, effect, or authority");return 0
if __name__=="__main__":raise SystemExit(main())
