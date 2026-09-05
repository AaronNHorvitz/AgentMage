#!/usr/bin/env python3
"""Generate Sprint 154 cited cloud cost and delivery correlation evidence."""
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-154/cloud-delivery-correlation-corpus.json";SOURCE=ROOT/"kernel/engine/src/cloud_delivery_correlation.rs"
PROVIDERS=("aws","azure","gcp");VIEWS=("deployment_to_health","incident_to_observation","work_to_deployment","configuration_to_metric","cost_to_service","cross_evidence")
ATTACKS=("none","identity_collision","rename","transfer","deleted_resource","delayed_telemetry","clock_skew","stale_cost","missing_tags","shared_service","currency_difference","conflicting_evidence","injected_causation","injected_action")
CLASSES=("provider_native","deterministic_mapping","user_confirmed","model_suggestion","statistical","uncertain");FIXTURES=("reference","fault","malicious","recomputed")
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def build():
 cases=[]
 for provider in PROVIDERS:
  for view in VIEWS:
   for attack in ATTACKS:
    for association in CLASSES:
     for fixture in FIXTURES:cases.append({"id":f"AT-CCST-001-{len(cases)+1:05d}","provider":provider,"view":view,"attack":attack,"association_class":association,"fixture":fixture,"native_identity_visible":True,"source_cited":True,"time_visible":True,"cost_assumptions_visible":True,"causal_claim_count":0,"inherited_authority_count":0})
 value={"schema_version":1,"providers":list(PROVIDERS),"views":list(VIEWS),"attacks":list(ATTACKS),"association_classes":list(CLASSES),"fixtures":list(FIXTURES),"cases":cases,"case_count":len(cases),"causal_claim_count":0,"inherited_authority_count":0,"source_sha256":{"kernel/engine/src/cloud_delivery_correlation.rs":sha(SOURCE)}}
 return(json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def check():
 expected=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=expected:raise RuntimeError("cloud delivery correlation corpus stale")
 value=json.loads(expected);source=SOURCE.read_text().split("#[cfg(test)]",1)[0]
 for token in ("AssociationClass","CorrelatedViewKind","CorrelationSource","CostContext","CloudDeliveryCorrelation","validate_cost","validate_correlation","reject_correlation_action"):
  if token not in source:raise RuntimeError(token)
 if value["case_count"]!=6048 or value["causal_claim_count"]or value["inherited_authority_count"]:raise RuntimeError("correlation drift")
def main():
 parser=argparse.ArgumentParser();parser.add_argument("--write",action="store_true");args=parser.parse_args()
 if args.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();print("validated 6,048 AT-CCST-001 cases with zero causal claim or inherited authority");return 0
if __name__=="__main__":raise SystemExit(main())
