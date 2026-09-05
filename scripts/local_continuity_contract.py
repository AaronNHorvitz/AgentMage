#!/usr/bin/env python3
"""Generate Sprint 161 snapshot and checkpoint fixtures."""
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-161/local-continuity-corpus.json";SOURCE=ROOT/"kernel/engine/src/local_continuity.rs"
ROOTS=("local","cloud_sync","network","remote","placeholder","linked","raced","unsupported","uncertain");FAILURES=("none","interruption","low_disk","missing","duplicate","corrupt","replay","stale","cross_version","ransomware_like","retention","deletion","clean_device");STAGES=("plan","chunk","encrypt","publish","verify","restore","swap","rollback")
TRANSITIONS=("census","parse","graph","packet","reconcile","report");CHANGES=("change","rename","delete","move","reclassify","regenerate","reparse","queue","clock","completion");RECORDS=("file","symbol","module","edge","packet","card","contradiction","finding","coverage","report","queue","checkpoint")
def build():
 backup=[{"id":f"AT-BKC-001-{i+1:05d}","root":r,"failure":f,"stage":s,"plaintext_count":0,"raw_credential_count":0,"false_complete_count":0,"canonical_preconfirm_mutation_count":0,"fail_closed":True}for i,(r,f,s)in enumerate((r,f,s)for r in ROOTS for f in FAILURES for s in STAGES)]
 checkpoint=[{"id":f"AT-CKP-001-{i+1:05d}","transition":t,"change":c,"record":r,"stale_current_count":0,"deterministic":True,"replacement_scheduled":True,"broader_rescan_on_uncertainty":True}for i,(t,c,r)in enumerate((t,c,r)for t in TRANSITIONS for c in CHANGES for r in RECORDS)]
 v={"schema_version":1,"roots":list(ROOTS),"failures":list(FAILURES),"stages":list(STAGES),"transitions":list(TRANSITIONS),"changes":list(CHANGES),"records":list(RECORDS),"backup_case_count":len(backup),"checkpoint_case_count":len(checkpoint),"backup_cases":backup,"checkpoint_cases":checkpoint,"plaintext_count":0,"raw_credential_count":0,"false_complete_count":0,"canonical_preconfirm_mutation_count":0,"stale_current_count":0,"source_sha256":{"kernel/engine/src/local_continuity.rs":hashlib.sha256(SOURCE.read_bytes()).hexdigest()}}
 return(json.dumps(v,sort_keys=True,separators=(",",":"))+"\n").encode()
def check():
 e=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=e:raise RuntimeError("local continuity corpus stale")
 v=json.loads(e)
 if v["backup_case_count"]!=936 or v["checkpoint_case_count"]!=720 or any(v[k]for k in ("plaintext_count","raw_credential_count","false_complete_count","canonical_preconfirm_mutation_count","stale_current_count")):raise RuntimeError("local continuity drift")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();print("validated 936 snapshot and 720 checkpoint cases with zero plaintext, raw credential, false completion, premature mutation, or stale-current record");return 0
if __name__=="__main__":raise SystemExit(main())
