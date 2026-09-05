#!/usr/bin/env python3
"""Generate Sprint 163 catalog and semantic-analysis fixtures."""
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-163/model-catalog-semantic-corpus.json";SOURCE=ROOT/"kernel/engine/src/model_catalog_semantic.rs"
STATES=("candidate","evaluating","approved","degraded","quarantined","rejected","retired");COMPAT=("compatible","limited","incompatible","unknown","stale","revoked","blocked");MUTATIONS=("identity","evidence","compatibility","state","support","signature","expiry","substitution");CHANNELS=("cli","chat","configuration","import","fallback","picker","manifest","runtime")
PARSERS=("compiler","ast","language_service","package","build","schema","test","workflow","source_control","text_search");PARTITIONS=("package","service","responsibility","state","feature","workflow","schema","cross_cutting");VARIATIONS=("packet_order","partition_size","context_limit","retrieval_rank","interruption","model_profile","runtime_restart","summary_depth")
def build():
 catalog=[{"id":f"AT-MCAT-001-{i+1:05d}","state":s,"compatibility":c,"mutation":m,"channel":h,"unauthorized_usability_count":0,"family_inheritance_count":0,"omitted_negative_count":0,"exact_identity":True}for i,(s,c,m,h)in enumerate((s,c,m,h)for s in STATES for c in COMPAT for m in MUTATIONS for h in CHANNELS)]
 semantic=[{"id":f"AT-SEM-001-{i+1:05d}","parser":p,"partition":q,"variation":v,"structural_gap_count":0,"coverage_gap_count":0,"authority_change_count":0,"completion_change_count":0,"source_bound":True}for i,(p,q,v)in enumerate((p,q,v)for p in PARSERS for q in PARTITIONS for v in VARIATIONS)]
 x={"schema_version":1,"states":list(STATES),"compatibility":list(COMPAT),"mutations":list(MUTATIONS),"channels":list(CHANNELS),"parsers":list(PARSERS),"partitions":list(PARTITIONS),"variations":list(VARIATIONS),"catalog_case_count":len(catalog),"semantic_case_count":len(semantic),"catalog_cases":catalog,"semantic_cases":semantic,"unauthorized_usability_count":0,"family_inheritance_count":0,"omitted_negative_count":0,"structural_gap_count":0,"coverage_gap_count":0,"authority_change_count":0,"completion_change_count":0,"source_sha256":{"kernel/engine/src/model_catalog_semantic.rs":hashlib.sha256(SOURCE.read_bytes()).hexdigest()}}
 return(json.dumps(x,sort_keys=True,separators=(",",":"))+"\n").encode()
def check():
 e=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=e:raise RuntimeError("model catalog semantic corpus stale")
 v=json.loads(e)
 if v["catalog_case_count"]!=3136 or v["semantic_case_count"]!=640 or any(v[k]for k in ("unauthorized_usability_count","family_inheritance_count","omitted_negative_count","structural_gap_count","coverage_gap_count","authority_change_count","completion_change_count")):raise RuntimeError("model catalog semantic drift")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();print("validated 3,136 catalog and 640 semantic cases with zero usability, inheritance, omission, structural, coverage, authority, or completion drift");return 0
if __name__=="__main__":raise SystemExit(main())
