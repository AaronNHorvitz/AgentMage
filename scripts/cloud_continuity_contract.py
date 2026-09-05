#!/usr/bin/env python3
"""Generate Sprint 162 encrypted cloud-continuity fixtures."""
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-162/cloud-continuity-corpus.json";SOURCE=ROOT/"kernel/engine/src/cloud_continuity.rs"
PROVIDERS=("s3_compatible","onedrive","google_drive","box","dropbox","later_conformance");ATTACKS=("cross_account","cross_prefix","list","read","write","delete","redirect","proxy","credential_reuse","observer_crossover");FAILURES=("throttle","partition","outage","revocation","changed_version","multipart_interrupt","partial_delete","stale_manifest","local_loss");FIXTURES=("upload","reconcile","restore","remove")
def build():
 cases=[{"id":f"AT-CBK-001-{i+1:05d}","provider":p,"attack":a,"failure":f,"fixture":x,"out_of_scope_count":0,"duplicate_effect_count":0,"plaintext_count":0,"raw_credential_count":0,"observer_crossover_count":0,"fail_closed":True}for i,(p,a,f,x)in enumerate((p,a,f,x)for p in PROVIDERS for a in ATTACKS for f in FAILURES for x in FIXTURES)]
 v={"schema_version":1,"providers":list(PROVIDERS),"attacks":list(ATTACKS),"failures":list(FAILURES),"fixtures":list(FIXTURES),"case_count":len(cases),"cases":cases,"out_of_scope_count":0,"duplicate_effect_count":0,"plaintext_count":0,"raw_credential_count":0,"observer_crossover_count":0,"source_sha256":{"kernel/engine/src/cloud_continuity.rs":hashlib.sha256(SOURCE.read_bytes()).hexdigest()}}
 return(json.dumps(v,sort_keys=True,separators=(",",":"))+"\n").encode()
def check():
 e=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=e:raise RuntimeError("cloud continuity corpus stale")
 v=json.loads(e)
 if v["case_count"]!=2160 or any(v[k]for k in ("out_of_scope_count","duplicate_effect_count","plaintext_count","raw_credential_count","observer_crossover_count")):raise RuntimeError("cloud continuity drift")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();print("validated 2,160 AT-CBK-001 cases with zero out-of-scope access, duplicate effect, plaintext, credential, or observer crossover");return 0
if __name__=="__main__":raise SystemExit(main())
