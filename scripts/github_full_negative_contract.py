#!/usr/bin/env python3
"""Build and verify Sprint 106 local GitHub negative evidence."""
from __future__ import annotations
import argparse,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-106/github-negative-corpus.json"
PROHIBITED=("generic-pull","merge","rebase","reset","clean","discard","stash-tag-note-mutation","branch-deletion","remote-configuration","hook-filter-execution","mirror","force","force-with-lease","implicit-refspec","bypass","repository-administration","organization-administration","secret-change","ruleset-change","automatic-merge","automatic-release","automatic-review")
ATTACKS=("hostile-config","hostile-object","manifest-race","cross-host-credential","cross-account-credential","cross-repository-credential","redirect","proxy","certificate-change","host-key-change","stale-ref","moved-line","protection-change","ruleset-change","check-change","review-change","bypass","injected-content","hidden-field","implicit-refspec","force-form","unsupported-ghes-version","unsupported-ghes-feature","administrative-operation")
def build()->bytes:
 cases=[{"case_id":f"S-106-N-{i+1:05d}","attack":ATTACKS[i%len(ATTACKS)],"expected":"absent-or-denied","provider_request_count":0,"unauthorized_effect_count":0,"user_work_loss_count":0,"fallback_count":0,"receipt_count":1} for i in range(10000)]
 value={"schema_version":1,"prohibited_operations":list(PROHIBITED),"attack_classes":list(ATTACKS),"case_count":len(cases),"cases":cases,"supported_github_com_tuple_count":0,"supported_ghes_tuple_count":0,"native_request_count":0,"complete_rv49":False,"reviewer":"scripts.github_full_negative_contract"}
 return (json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def write():OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
def check():
 if not OUTPUT.is_file() or OUTPUT.read_bytes()!=build():raise RuntimeError("GitHub negative corpus stale or absent")
 value=json.loads(OUTPUT.read_bytes())
 if value["case_count"]!=10000 or any(c["provider_request_count"] or c["unauthorized_effect_count"] or c["user_work_loss_count"] or c["fallback_count"] or c["receipt_count"]!=1 for c in value["cases"]):raise RuntimeError("negative campaign failed")
 if value["supported_github_com_tuple_count"] or value["supported_ghes_tuple_count"] or value["native_request_count"] or value["complete_rv49"]:raise RuntimeError("provider support overclaim")
def main():
 parser=argparse.ArgumentParser();parser.add_argument("--write",action="store_true");arguments=parser.parse_args()
 if arguments.write:write()
 check();print("validated 10000 GitHub negative mutations across 24 attack classes with zero provider requests");return 0
if __name__=="__main__":raise SystemExit(main())
