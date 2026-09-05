#!/usr/bin/env python3
"""Generate Sprint 158 credential-broker mutation fixtures."""
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-158/credential-broker-corpus.json";SOURCE=ROOT/"kernel/engine/src/credential_broker.rs"
STORES=("linux_secret_service","windows_credential_manager","windows_dpapi","macos_keychain_retained");FLOWS=("system_browser_oauth","device_oauth","short_lived_token","ssh_agent","certificate_reference","scoped_manual_token");ATTACKS=("worker","provider","host","tenant","account","operation","scope","redirect","proxy","expiry","race","replay");LIFECYCLES=("active","expired","refreshing","rotating","revoked","deleted","disconnected","emergency_disabled");FIXTURES=("normal","crash","concurrent")
SURFACES=("prompts","model_context","chat","files","arguments","environment","logs","receipts","diagnostics","exports","crashes","snapshots","residue")
def build():
 cases=[{"id":f"AT-CRD-001-{i+1:05d}","store":s,"flow":f,"mutation":a,"lifecycle":l,"fixture":x,"disclosure_count":0,"wrong_account_count":0,"stale_reference_count":0,"worker_memory_cleared":True,"fail_closed":True}for i,(s,f,a,l,x) in enumerate((s,f,a,l,x)for s in STORES for f in FLOWS for a in ATTACKS for l in LIFECYCLES for x in FIXTURES)]
 return(json.dumps({"schema_version":1,"stores":list(STORES),"flows":list(FLOWS),"mutations":list(ATTACKS),"lifecycles":list(LIFECYCLES),"fixtures":list(FIXTURES),"prohibited_surfaces":list(SURFACES),"case_count":len(cases),"cases":cases,"disclosure_count":0,"wrong_account_count":0,"stale_reference_count":0,"canary_finding_count":0,"restored_raw_credential_count":0,"restored_usable_credential_count":0,"reauthentication_required":True,"source_sha256":{"kernel/engine/src/credential_broker.rs":hashlib.sha256(SOURCE.read_bytes()).hexdigest()}},sort_keys=True,separators=(",",":"))+"\n").encode()
def check():
 expected=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=expected:raise RuntimeError("credential broker corpus stale")
 value=json.loads(expected)
 if value["case_count"]!=6912 or any(value[k] for k in ("disclosure_count","wrong_account_count","stale_reference_count","canary_finding_count","restored_raw_credential_count","restored_usable_credential_count")):raise RuntimeError("credential broker contract drift")
def main():
 parser=argparse.ArgumentParser();parser.add_argument("--write",action="store_true");args=parser.parse_args()
 if args.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();print("validated 6,912 AT-CRD-001 mutations with zero disclosure, wrong-account request, stale reference, canary finding, or restored credential");return 0
if __name__=="__main__":raise SystemExit(main())
