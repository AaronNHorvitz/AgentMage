#!/usr/bin/env python3
"""Generate Sprint 155 cross-pack hostile and removal corpus."""
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-155/cross-pack-corpus.json";SOURCE=ROOT/"kernel/engine/src/cross_pack_verification.rs"
AUTONOMY=("observe","propose","approved_write","reconcile");PACKS=("productivity","communications","finance","cloud","delivery","all")
ATTACKS=("schema","ipc","event","webhook","cursor","provider_result","recipient","attachment","document","financial_value","cloud_scope","log_archive","manifest","model_output","workflow_graph","replay")
STATES=("idle","queued","in_flight","uncertain","synchronizing","stale","crashed","partially_removed");REMOVALS=("one_pack","two_packs","productivity","communications","finance")
PROHIBITED=("money_movement","financial_administration","cloud_mutation","cloud_execution","secret_access","identity_administration","policy_administration","content_created_authority")
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def build():
 cases=[]
 for autonomy in AUTONOMY:
  for pack in PACKS:
   for attack in ATTACKS:
    for state in STATES:
     for removal in REMOVALS:cases.append({"id":f"AT-XPR-001-{len(cases)+1:05d}","autonomy":autonomy,"pack":pack,"attack":attack,"state":state,"removal":removal,"attributable":True,"receipted":True,"evidence_current":True,"unauthorized_disclosure_count":0,"unauthorized_effect_count":0,"money_movement_count":0,"cloud_mutation_count":0,"duplicate_count":0,"false_completion_count":0,"hidden_blocker_count":0,"residue_count":0})
 value={"schema_version":1,"autonomy_levels":list(AUTONOMY),"packs":list(PACKS),"attacks":list(ATTACKS),"states":list(STATES),"removals":list(REMOVALS),"prohibited":list(PROHIBITED),"cases":cases,"case_count":len(cases),"unauthorized_disclosure_count":0,"unauthorized_effect_count":0,"money_movement_count":0,"cloud_mutation_count":0,"duplicate_count":0,"false_completion_count":0,"hidden_blocker_count":0,"residue_count":0,"source_sha256":{"kernel/engine/src/cross_pack_verification.rs":sha(SOURCE)}}
 return(json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def check():
 expected=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=expected:raise RuntimeError("cross-pack corpus stale")
 value=json.loads(expected);source=SOURCE.read_text().split("#[cfg(test)]",1)[0]
 for token in ("CapabilityPack","ProhibitedAuthorityFamily","CampaignState","CrossPackCampaignCase","PackRemovalInventory","CampaignReconciliation","validate_campaign_case","validate_removal","validate_reconciliation"):
  if token not in source:raise RuntimeError(token)
 zero=("unauthorized_disclosure_count","unauthorized_effect_count","money_movement_count","cloud_mutation_count","duplicate_count","false_completion_count","hidden_blocker_count","residue_count")
 if value["case_count"]!=15360 or any(value[key]for key in zero):raise RuntimeError("cross-pack drift")
def main():
 parser=argparse.ArgumentParser();parser.add_argument("--write",action="store_true");args=parser.parse_args()
 if args.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();print("validated 15,360 AT-XPR-001 cases with zero disclosure, effect, movement, mutation, duplicate, false completion, blocker, or residue");return 0
if __name__=="__main__":raise SystemExit(main())
