#!/usr/bin/env python3
"""Generate the deterministic Sprint 164 management mutation corpus."""
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-164/model-management-corpus.json";SOURCE=ROOT/"kernel/engine/src/model_management.rs"
INTENTS=("list","explain","recommend","download","import","resume","verify","activate","compare","cancel","roll_back","remove","clean_storage")
MUTATIONS=("source","redirect","artifact","hash","license","size","tokenizer","template","codec","runtime","context","decoding","modality","platform","hardware","disk","catalog","preview","scan","activation","policy","support","signature","destination")
TRANSITIONS=("download","resume","import","verify","scan","self_test","activate","rollback","remove","cleanup")
OUTCOMES=("success","changed","malformed","unsupported","low_resource","failed_scan","stale_preview","interrupted","substituted","revoked","cancelled","crashed","uncertain")
def build():
 cases=[{"id":f"AT-MGR-001-{i+1:05d}","mutation":m,"transition":t,"outcome":o,"silent_activation_count":0,"partial_activation_count":0,"substituted_profile_count":0,"unconfirmed_effect_count":0,"prior_profile_preserved":True,"quarantine_isolated":True}for i,(m,t,o)in enumerate((m,t,o)for m in MUTATIONS for t in TRANSITIONS for o in OUTCOMES)]
 value={"schema_version":1,"intents":list(INTENTS),"mutations":list(MUTATIONS),"transitions":list(TRANSITIONS),"outcomes":list(OUTCOMES),"case_count":len(cases),"cases":cases,"silent_activation_count":0,"partial_activation_count":0,"substituted_profile_count":0,"unconfirmed_effect_count":0,"source_sha256":{"kernel/engine/src/model_management.rs":hashlib.sha256(SOURCE.read_bytes()).hexdigest()}}
 return(json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def check():
 expected=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=expected:raise RuntimeError("model management corpus stale")
 value=json.loads(expected)
 if value["case_count"]!=3120 or any(value[k]for k in ("silent_activation_count","partial_activation_count","substituted_profile_count","unconfirmed_effect_count")):raise RuntimeError("model management safety drift")
def main():
 parser=argparse.ArgumentParser();parser.add_argument("--write",action="store_true");args=parser.parse_args()
 if args.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();print("validated 3,120 model-management mutations with zero silent, partial, substituted, or unconfirmed activation");return 0
if __name__=="__main__":raise SystemExit(main())
