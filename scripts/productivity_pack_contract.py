#!/usr/bin/env python3
"""Generate and validate Sprint 127 productivity-pack support fixtures."""
from __future__ import annotations
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-127/productivity-pack-corpus.json";SOURCE=ROOT/"kernel/engine/src/productivity_pack.rs"
STATES=("supported","degraded","unsupported","disabled","revoked","removed");PACKS=("communications","finance","cloud_observer");CASE_TYPES=("valid","absent","omission","future_version","malformed","contradictory_support","undeclared_flow","duplicate")
INVENTORY=("components","processes","sockets","credential_references","caches","cursors","schedules","webhooks","indexes","retained_data","tools")
MATRIX={"communications":{"operations":["read","draft","communication_write"],"prohibited":["money_movement","cloud_mutation"]},"finance":{"operations":["read","draft","financial_read"],"prohibited":["money_movement","communication_write","cloud_mutation"]},"cloud_observer":{"operations":["read","cloud_read"],"prohibited":["cloud_mutation","money_movement","communication_write"]}}
def sha(path:Path)->str:return hashlib.sha256(path.read_bytes()).hexdigest()
def build()->bytes:
 cases=[]
 for state in STATES:
  for pack in PACKS:
   for kind in CASE_TYPES:
    inactive=state in {"unsupported","disabled","revoked","removed"}or kind!="valid"
    cases.append({"case_id":f"S-127-C-{len(cases)+1:03d}","state":state,"pack":pack,"case_type":kind,"registered_operations":[]if inactive else MATRIX[pack]["operations"],"authority_inventory":{field:[]for field in INVENTORY}if inactive else {field:([f"{pack}-{field}"]if field in {"components","tools"}else[])for field in INVENTORY},"admitted":not inactive,"visible_reason":None if not inactive else kind if kind!="valid"else state})
 strict={field:[]for field in INVENTORY}
 value={"schema_version":1,"states":list(STATES),"packs":list(PACKS),"case_types":list(CASE_TYPES),"inventory_fields":list(INVENTORY),"provider_operation_matrix":MATRIX,"lifecycle_actions":["install","enable","authenticate","synchronize","suspend","revoke","disable","remove","restore_strict_local"],"allowed_data_flows":[{"source":"communications","destination":"finance","purpose":"user-confirmed-receipt-match"},{"source":"communications","destination":"cloud_observer","purpose":"user-confirmed-incident-correlation"},{"source":"cloud_observer","destination":"communications","purpose":"user-confirmed-observation-draft"}],"absent_authority":strict,"disabled_authority":dict(strict),"strict_local_equivalent":True,"cases":cases,"case_count":len(cases),"source_sha256":{"PRODUCTIVITY-SYSTEM.md":sha(ROOT/"PRODUCTIVITY-SYSTEM.md"),"docs/decisions/0009-productivity-finance-and-cloud-observer-expansion.md":sha(ROOT/"docs/decisions/0009-productivity-finance-and-cloud-observer-expansion.md"),"kernel/engine/src/productivity_pack.rs":sha(SOURCE)},"installed_pack_count":0,"enabled_pack_count":0,"provider_account_count":0,"support_promotion_count":0}
 return(json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def check():
 expected=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=expected:raise RuntimeError("productivity pack corpus stale or absent")
 value=json.loads(expected);source=SOURCE.read_text().split("#[cfg(test)]",1)[0]
 for token in ("ProductivitySupportState","ProductivityPackManifest","ProductivityRuntimeInventory","ProductivityDataFlowEdge","validate_productivity_manifest","discover_productivity_operations","transition_productivity_lifecycle","admit_productivity_data_flow","deny_unknown_fields"):
  if token not in source:raise RuntimeError(f"productivity contract absent: {token}")
 for forbidden in ("MoneyMovement","CloudMutation","std::process","TcpStream","reqwest"):
  if forbidden in source:raise RuntimeError(f"forbidden pack authority admitted: {forbidden}")
 if value["case_count"]!=144 or len(value["cases"])!=144:raise RuntimeError("productivity corpus count drift")
 if value["absent_authority"]!=value["disabled_authority"]or not value["strict_local_equivalent"]:raise RuntimeError("strict-local equivalence drift")
 if any(case["admitted"]and case["case_type"]!="valid"for case in value["cases"]):raise RuntimeError("invalid manifest admitted")
 if any(value[key]for key in ("installed_pack_count","enabled_pack_count","provider_account_count","support_promotion_count")):raise RuntimeError("productivity support overclaim")
def main():
 parser=argparse.ArgumentParser();parser.add_argument("--write",action="store_true");args=parser.parse_args()
 if args.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();print("validated 144 productivity-pack cases, 6 support states, 3 pack matrices, and zero optional authority");return 0
if __name__=="__main__":raise SystemExit(main())
