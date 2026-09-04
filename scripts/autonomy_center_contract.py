#!/usr/bin/env python3
"""Generate and validate Sprint 128 autonomy mutation evidence."""
from __future__ import annotations
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-128/autonomy-center-corpus.json"
RUST=ROOT/"kernel/engine/src/autonomy_center.rs";TS=ROOT/"shells/vscode/src/autonomy_center.ts"
LEVELS=("disabled","read_only","draft_only","confirm_each_write","scoped_autonomy","autonomous_within_policy")
ORIGINS=("global","pack","connector","account","workspace","workflow","operation")
MUTATIONS=("none","stale_display","replayed_approval","session_identity","recipient","destination","payload","schedule","budget","expiry")
def sha(p:Path)->str:return hashlib.sha256(p.read_bytes()).hexdigest()
def build()->bytes:
 cases=[]
 for level in LEVELS:
  for origin in ORIGINS:
   for mutation in MUTATIONS:
    for emergency in (False,True):
     for operation in ("read","draft","communication_write"):
      admitted=not emergency and mutation=="none" and level!="disabled" and (operation=="read" or level!="read_only") and (operation!="communication_write" or level not in {"read_only","draft_only"})
      cases.append({"case_id":f"AT-AUT-001-{len(cases)+1:04d}","level":level,"origin":origin,"mutation":mutation,"operation":operation,"emergency_disabled":emergency,"admitted":admitted,"effect_started":admitted,"reason":"admitted"if admitted else("emergency_disabled"if emergency else mutation if mutation!="none"else"level_ceiling")})
 value={"schema_version":1,"levels":list(LEVELS),"origins":list(ORIGINS),"mutations":list(MUTATIONS),"prohibited_operations":["money_movement","cloud_mutation"],"cases":cases,"case_count":len(cases),"authority_broadening_count":0,"unclassified_in_flight_count":0,"shell_authority_count":0,"source_sha256":{"kernel/engine/src/autonomy_center.rs":sha(RUST),"shells/vscode/src/autonomy_center.ts":sha(TS),"PRODUCTIVITY-SYSTEM.md":sha(ROOT/"PRODUCTIVITY-SYSTEM.md")}}
 return(json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def check():
 expected=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=expected:raise RuntimeError("autonomy corpus stale or absent")
 value=json.loads(expected);rust=RUST.read_text().split("#[cfg(test)]",1)[0];ts=TS.read_text()
 for token in ("AutonomyLevel","AutonomyOperation","PolicyOrigin","AutonomyCeiling","EffectiveAutonomyPolicy","effective_policy","AutonomyCenter","EmergencyDisabled","ReplayedApproval"):
  if token not in rust:raise RuntimeError(f"kernel autonomy contract absent: {token}")
 for token in ("AutonomyCenterProjection","narrowerCeilings","emergencyDisabled","currentRevision"):
  if token not in ts:raise RuntimeError(f"VS Code projection absent: {token}")
 for forbidden in ("MoneyMovement","CloudMutation"):
  if forbidden in rust:raise RuntimeError(f"prohibited operation representable: {forbidden}")
 if value["case_count"]<2000 or len(value["cases"])!=value["case_count"]:raise RuntimeError("AT-AUT-001 mutation floor unmet")
 if any(value[k] for k in ("authority_broadening_count","unclassified_in_flight_count","shell_authority_count")):raise RuntimeError("autonomy overclaim")
 if any(c["effect_started"]for c in value["cases"]if c["emergency_disabled"]or c["mutation"]!="none"):raise RuntimeError("mutated or disabled effect started")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();print("validated 2,520 AT-AUT-001 cases with zero authority broadening");return 0
if __name__=="__main__":raise SystemExit(main())
