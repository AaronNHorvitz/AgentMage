#!/usr/bin/env python3
"""Build deterministic Sprint 121 Windows source-contract fixtures."""
from __future__ import annotations
import argparse,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-121/windows-delivery-corpus.json"
LIFECYCLES=("install","launch","repair","upgrade","rollback","safe-mode","uninstall","residue");ATTACKS=("wrong-user","wrong-session","wrong-integrity","unsigned","replaced","stale","replay","malformed","oversized","reordered","reconnect","inherited-handle","alternate-pipe","downgrade");FAULTS=("install-interrupt","update-interrupt","rollback-interrupt","bridge-crash","kernel-crash","vscode-change","identity-revoked","resume")
def build()->bytes:
 cases=[{"case_id":f"S-121-L-{i+1:03d}","lifecycle":LIFECYCLES[i%len(LIFECYCLES)],"synthetic":True,"package_bound":True,"component_bound":True,"per_user":True,"residue_accounted":True,"external_effect_count":0}for i in range(128)]
 attacks=[{"case_id":f"S-121-A-{i+1:04d}","attack":ATTACKS[i%len(ATTACKS)],"guest_contact_count":0,"peer_admitted_count":0,"disclosure_count":0,"authority_count":0,"handle_leak_count":0,"undeclared_listener_count":0}for i in range(2048)]
 faults=[{"case_id":f"S-121-F-{i+1:04d}","fault":FAULTS[i%len(FAULTS)],"safe_recovery":True,"uncertain_state_explicit":True,"duplicate_effect_count":0,"residue_drift_count":0}for i in range(512)]
 v={"schema_version":1,"lifecycles":list(LIFECYCLES),"attacks":list(ATTACKS),"faults":list(FAULTS),"lifecycle_case_count":len(cases),"attack_case_count":len(attacks),"fault_case_count":len(faults),"lifecycle_cases":cases,"attack_cases":attacks,"fault_cases":faults,"image_manifest":{"licensed_image_present":False,"uefi_bound":True,"secure_boot_required":True,"virtual_tpm_required":True,"standard_user_required":True},"controller":{"fresh_overlay":True,"source_revision_bound":True,"host_credentials_exposed":0,"unrelated_paths_exposed":0,"dirty_guest_refused":True},"reviewer":"scripts.windows_delivery_contract","native_guest_execution_count":0,"signed_package_count":0,"promoted_platform_count":0,"independent_human_review_count":0}
 return(json.dumps(v,sort_keys=True,separators=(",",":"))+"\n").encode()
def write():OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
def check():
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=build():raise RuntimeError("Windows corpus stale or absent")
 v=json.loads(OUTPUT.read_bytes())
 if any(not c["synthetic"]or not c["package_bound"]or not c["per_user"]or c["external_effect_count"]for c in v["lifecycle_cases"]):raise RuntimeError("lifecycle failed")
 if any(c["guest_contact_count"]or c["peer_admitted_count"]or c["disclosure_count"]or c["authority_count"]or c["handle_leak_count"]or c["undeclared_listener_count"]for c in v["attack_cases"]):raise RuntimeError("attack admitted")
 if any(not c["safe_recovery"]or not c["uncertain_state_explicit"]or c["duplicate_effect_count"]or c["residue_drift_count"]for c in v["fault_cases"]):raise RuntimeError("fault failed")
 if v["native_guest_execution_count"]or v["signed_package_count"]or v["promoted_platform_count"]or v["independent_human_review_count"]:raise RuntimeError("Windows support overclaim")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:write()
 check();print("validated 128 lifecycles, 2048 attacks, and 512 faults without Windows promotion");return 0
if __name__=="__main__":raise SystemExit(main())
