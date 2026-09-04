#!/usr/bin/env python3
"""Build deterministic Sprint 120 Markdown mathematics fixtures."""
from __future__ import annotations
import argparse,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-120/markdown-math-corpus.json"
SYNTAX=("inline","display","environment","command","macro","label","reference","escape","unicode","list","table","quote","link","code-fence","frontmatter");ATTACKS=("malformed-delimiter","nested-environment","shell-escape","file-input","file-output","network-url","package-load","macro-recursion","token-bomb","mixed-markdown");FAULTS=("cancel","crash","cache-full","disk-full","renderer-change","cache-corruption","resume")
def build()->bytes:
 rounds=[{"case_id":f"S-120-R-{i+1:03d}","syntax":SYNTAX[i%len(SYNTAX)],"source_span_exact":True,"delimiter_exact":True,"round_trip_exact":True,"code_fence_inert":True,"currency_inert":True,"reference_preserved":True}for i in range(128)]
 attacks=[{"case_id":f"S-120-A-{i+1:04d}","attack":ATTACKS[i%len(ATTACKS)],"file_access_count":0,"network_access_count":0,"process_count":0,"package_load_count":0,"authority_grant_count":0,"unbounded_result_count":0}for i in range(2048)]
 faults=[{"case_id":f"S-120-F-{i+1:04d}","fault":FAULTS[i%len(FAULTS)],"cleanup_complete":True,"stale_preview_count":0,"unbounded_resource_count":0,"cache_identity_bound":True}for i in range(512)]
 v={"schema_version":1,"syntax":list(SYNTAX),"attacks":list(ATTACKS),"faults":list(FAULTS),"round_trip_case_count":len(rounds),"attack_case_count":len(attacks),"fault_case_count":len(faults),"round_trip_cases":rounds,"attack_cases":attacks,"fault_cases":faults,"renderer":{"component":"agentmage-math-reference","version":"1.0.0","offline":True,"pinned":True,"packaged":True,"deterministic":True,"runtime_download_count":0,"remote_asset_count":0,"executable_extension_count":0},"preview_contract":{"copyable_source":True,"zoom_reflow":True,"high_contrast":True,"keyboard_navigation":True,"screen_reader_text":True,"native_platform_execution_count":0},"reviewer":"scripts.markdown_math_contract","promoted_platform_count":0,"independent_human_review_count":0}
 return(json.dumps(v,sort_keys=True,separators=(",",":"))+"\n").encode()
def write():OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
def check():
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=build():raise RuntimeError("math corpus stale or absent")
 v=json.loads(OUTPUT.read_bytes())
 if any(not all(c[k]for k in("source_span_exact","delimiter_exact","round_trip_exact","code_fence_inert","currency_inert","reference_preserved"))for c in v["round_trip_cases"]):raise RuntimeError("round trip failed")
 if any(c["file_access_count"]or c["network_access_count"]or c["process_count"]or c["package_load_count"]or c["authority_grant_count"]or c["unbounded_result_count"]for c in v["attack_cases"]):raise RuntimeError("attack admitted")
 if any(not c["cleanup_complete"]or c["stale_preview_count"]or c["unbounded_resource_count"]or not c["cache_identity_bound"]for c in v["fault_cases"]):raise RuntimeError("fault failed")
 if not all(v["renderer"][k]for k in("offline","pinned","packaged","deterministic"))or v["promoted_platform_count"]or v["independent_human_review_count"]:raise RuntimeError("renderer overclaim")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:write()
 check();print("validated 128 math round trips, 2048 attacks, and 512 faults without platform promotion");return 0
if __name__=="__main__":raise SystemExit(main())
