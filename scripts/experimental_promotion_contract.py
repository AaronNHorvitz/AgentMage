#!/usr/bin/env python3
"""Generate Sprint 168 promotion-bypass fixtures."""
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-168/experimental-promotion-corpus.json";SOURCE=ROOT/"experimental/model-lab/src/promotion.rs"
ROUTES=("chat","model_output","file","copied_manifest","catalog_edit","stale_approval","alias","hash","preference","lab_result");MUTATIONS=("identity","license","lineage","artifact","runtime","resource","quality","security","platform","review","signature","candidate","policy");STATES=("idle","importing","quarantined","parsing","loaded","evaluating","cancelling","crashed","removing","removed","reinstalled","stale","revoked","unsupported","malformed","duplicate")
def build():
 cases=[{"id":f"AT-EML-001-P-{i+1:05d}","route":r,"mutation":m,"state":s,"ordinary_activation_count":0,"direct_promotion_count":0,"approved_state_change_count":0,"canonical_damage_count":0}for i,(r,m,s)in enumerate((r,m,s)for r in ROUTES for m in MUTATIONS for s in STATES)]
 value={"schema_version":1,"routes":list(ROUTES),"mutations":list(MUTATIONS),"states":list(STATES),"case_count":len(cases),"cases":cases,"ordinary_activation_count":0,"direct_promotion_count":0,"approved_state_change_count":0,"canonical_damage_count":0,"source_sha256":{"experimental/model-lab/src/promotion.rs":hashlib.sha256(SOURCE.read_bytes()).hexdigest()}}
 return(json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def check():
 expected=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=expected:raise RuntimeError("experimental promotion corpus stale")
 value=json.loads(expected)
 if value["case_count"]!=2080 or any(value[k]for k in ("ordinary_activation_count","direct_promotion_count","approved_state_change_count","canonical_damage_count")):raise RuntimeError("experimental promotion drift")
def main():
 parser=argparse.ArgumentParser();parser.add_argument("--write",action="store_true");args=parser.parse_args()
 if args.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();print("validated 2,080 experimental promotion mutations with zero direct promotion, activation, or canonical damage");return 0
if __name__=="__main__":raise SystemExit(main())
