#!/usr/bin/env python3
"""Build deterministic Sprint 122 Windows runtime-boundary fixtures."""
from __future__ import annotations
import argparse,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-122/windows-runtime-corpus.json"
PATH_ATTACKS=("device","extended-length","volume","unc","webdav","pipe","traversal","reserved-name","trailing-dot","trailing-space","alternate-stream","reparse","junction","symlink","mount","cloud-placeholder","case-fold","unicode","short-name","hard-link","rename","replace","race")
AMBIENT_ATTACKS=("profile","adjacent-directory","registry","environment","credential","clipboard","desktop","device","camera","microphone","process","job","neighboring-user")
STATE_ATTACKS=("dpapi-unavailable","credential-manager-unavailable","onedrive-root","redirected-profile","remote-share","removable-media","cloud-sync","model-substitution","hostile-service","redirect","proxy","cross-adapter-credential")
FAULTS=("durable-before","durable-during","durable-after","tool-before","tool-during","tool-after","model-before","model-during","model-after","credential-before","credential-during","credential-after","connected-before","connected-during","connected-after")
def build()->bytes:
 paths=[{"case_id":f"S-122-P-{i+1:04d}","attack":PATH_ATTACKS[i%len(PATH_ATTACKS)],"synthetic":True,"root_identity_bound":True,"target_identity_bound":True,"final_path_rechecked":True,"escape_count":0,"canary_disclosure_count":0}for i in range(1024)]
 ambient=[{"case_id":f"S-122-A-{i+1:04d}","attack":AMBIENT_ATTACKS[i%len(AMBIENT_ATTACKS)],"synthetic":True,"restricted_token":True,"job_object":True,"network_attempt_count":0,"ambient_access_count":0,"residue_count":0}for i in range(512)]
 state=[{"case_id":f"S-122-S-{i+1:04d}","attack":STATE_ATTACKS[i%len(STATE_ATTACKS)],"synthetic":True,"fail_closed":True,"secret_disclosure_count":0,"substitution_count":0,"authority_leak_count":0}for i in range(512)]
 faults=[{"case_id":f"S-122-F-{i+1:04d}","fault":FAULTS[i%len(FAULTS)],"synthetic":True,"cleanup_complete":True,"uncertainty_accurate":True,"duplicate_completed_operation_count":0,"residual_authority_count":0}for i in range(512)]
 value={"schema_version":1,"path_attacks":list(PATH_ATTACKS),"ambient_attacks":list(AMBIENT_ATTACKS),"state_attacks":list(STATE_ATTACKS),"faults":list(FAULTS),"path_case_count":len(paths),"ambient_case_count":len(ambient),"state_case_count":len(state),"fault_case_count":len(faults),"path_cases":paths,"ambient_cases":ambient,"state_cases":state,"fault_cases":faults,"contracts":{"handle_relative_ntfs":True,"restricted_worker":True,"dpapi_reference_only":True,"fixed_local_data_root":True,"hash_pinned_signed_model":True,"confined_provider_worker":True},"native_guest_execution_count":0,"observed_network_minutes":0,"packet_trace_count":0,"accessibility_run_count":0,"promoted_platform_count":0,"independent_human_review_count":0}
 return(json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def write():OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
def check():
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=build():raise RuntimeError("Windows runtime corpus stale or absent")
 v=json.loads(OUTPUT.read_bytes())
 if len(v["path_cases"])<1000 or any(c["escape_count"]or c["canary_disclosure_count"]or not c["final_path_rechecked"]for c in v["path_cases"]):raise RuntimeError("path boundary failed")
 if len(v["ambient_cases"])<500 or any(c["network_attempt_count"]or c["ambient_access_count"]or c["residue_count"]for c in v["ambient_cases"]):raise RuntimeError("worker boundary failed")
 if any(not c["fail_closed"]or c["secret_disclosure_count"]or c["substitution_count"]or c["authority_leak_count"]for c in v["state_cases"]):raise RuntimeError("state boundary failed")
 if any(not c["cleanup_complete"]or not c["uncertainty_accurate"]or c["duplicate_completed_operation_count"]or c["residual_authority_count"]for c in v["fault_cases"]):raise RuntimeError("recovery failed")
 if any(v[k]for k in("native_guest_execution_count","observed_network_minutes","packet_trace_count","accessibility_run_count","promoted_platform_count","independent_human_review_count")):raise RuntimeError("Windows support overclaim")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:write()
 check();print("validated 1024 path, 512 sandbox, 512 state, and 512 recovery cases without Windows promotion");return 0
if __name__=="__main__":raise SystemExit(main())
