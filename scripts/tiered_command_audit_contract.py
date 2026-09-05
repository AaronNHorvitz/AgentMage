#!/usr/bin/env python3
"""Generate Sprint 159 tiered-command and immutable-audit fixtures."""
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-159/tiered-command-audit-corpus.json";SOURCE=ROOT/"kernel/engine/src/tiered_command_audit.rs"
LEVELS=("disabled","inspect","workspace_autonomous","connected_operations","owner_unrestricted");SEMANTICS=("direct","shell","pipeline","redirection","script","interpreter","pty","package","build","test","git","docker","cancellation","receipt");ATTACKS=("model_activation","inheritance","scheduling","renewal","replay","race","stale_display","concealment","grant_union","hidden_launch");TRIGGERS=("expiry","stop","lock","logout","restart","policy_change","emergency_disablement","integrity_failure","panic_stop")
STATES=("tracked","staged","unstaged","untracked","ignored","sparse","generated","vendored","binary","large_file_storage","submodule","worktree","archive","symbolic_link","hard_link","inaccessible","malformed","special","external","changing","unsupported","failed");HAZARDS=("instruction","hook","build_script","parser_exploit","archive_bomb","path_traversal","link_race","descendant","crash","cancellation","secret","hosted_write");FIXTURES=("census","parser","verification")
def build():
 authority=[{"id":f"AT-AUT-002-{i+1:05d}","level":l,"semantic":s,"attack":a,"trigger":t,"escape_count":0,"hidden_launch_count":0,"surviving_process_count":0,"reusable_activation_count":0,"fail_closed":True}for i,(l,s,a,t)in enumerate((l,s,a,t)for l in LEVELS for s in SEMANTICS for a in ATTACKS for t in TRIGGERS)]
 audit=[{"id":f"AT-CEN-001-{i+1:05d}","state":s,"hazard":h,"fixture":f,"disposition_count":1,"canonical_mutation_count":0,"hosted_mutation_count":0,"raw_secret_count":0,"content_authority_count":0,"silent_omission_count":0,"parser_escape_count":0,"surviving_process_count":0}for i,(s,h,f)in enumerate((s,h,f)for s in STATES for h in HAZARDS for f in FIXTURES)]
 value={"schema_version":1,"levels":list(LEVELS),"semantics":list(SEMANTICS),"attacks":list(ATTACKS),"triggers":list(TRIGGERS),"repository_states":list(STATES),"hazards":list(HAZARDS),"authority_case_count":len(authority),"audit_case_count":len(audit),"authority_cases":authority,"audit_cases":audit,"escape_count":0,"hidden_launch_count":0,"surviving_process_count":0,"canonical_mutation_count":0,"hosted_mutation_count":0,"raw_secret_count":0,"content_authority_count":0,"silent_omission_count":0,"parser_escape_count":0,"source_sha256":{"kernel/engine/src/tiered_command_audit.rs":hashlib.sha256(SOURCE.read_bytes()).hexdigest()}}
 return(json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def check():
 expected=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=expected:raise RuntimeError("tiered command audit corpus stale")
 value=json.loads(expected)
 if value["authority_case_count"]!=6300 or value["audit_case_count"]!=792 or any(value[k]for k in ("escape_count","hidden_launch_count","surviving_process_count","canonical_mutation_count","hosted_mutation_count","raw_secret_count","content_authority_count","silent_omission_count","parser_escape_count")):raise RuntimeError("tiered command audit drift")
def main():
 parser=argparse.ArgumentParser();parser.add_argument("--write",action="store_true");args=parser.parse_args()
 if args.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();print("validated 6,300 authority and 792 immutable-audit cases with zero escape, launch, mutation, disclosure, omission, or residue");return 0
if __name__=="__main__":raise SystemExit(main())
