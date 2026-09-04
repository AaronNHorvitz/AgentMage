#!/usr/bin/env python3
"""Build and verify Sprint 98 safe-mode and recovery evidence."""
from __future__ import annotations
import argparse,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-98/maintenance-recovery-corpus.json"
DISABLED=("browser","child-agent","connector","executable","mcp","network","package","schedule")
FAILURES=("full-disk","corrupt-state","missing-secret-store","revoked-credential","missing-model","broken-package","lost-worktree","interrupted-update","failed-rollback")
SUBSYSTEMS=("desktop","connector","mcp","schedule","package","child-agent")
def build()->bytes:
 cases=[{"case_id":f"S-073-REC-{i+1:03d}","failure":FAILURES[i%len(FAILURES)],"subsystem":SUBSYSTEMS[i%len(SUBSYSTEMS)],"expected":"prior-or-complete-new-valid-state","optional_authority_count":0,"raw_content_count":0,"secret_count":0,"network_count":0} for i in range(72)]
 return (json.dumps({"schema_version":1,"case_count":72,"failure_classes":list(FAILURES),"diagnostic_subsystems":list(SUBSYSTEMS),"safe_mode_disabled":list(DISABLED),"safe_mode_disabled_count":8,"diagnostics_redacted":True,"last_known_good_is_proposal_only":True,"native_recovery_execution_count":0,"independent_review":False,"cases":cases},sort_keys=True,separators=(",",":"))+"\n").encode()
def write():OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
def check():
 if not OUTPUT.is_file() or OUTPUT.read_bytes()!=build():raise RuntimeError("maintenance recovery corpus stale or absent")
 v=json.loads(OUTPUT.read_bytes())
 if tuple(v["safe_mode_disabled"])!=DISABLED or any(c["optional_authority_count"] or c["raw_content_count"] or c["secret_count"] or c["network_count"] for c in v["cases"]) or v["native_recovery_execution_count"] or v["independent_review"]:raise RuntimeError("recovery overclaim")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:write()
 check();print("validated 72 recovery cases across 9 failure classes and 6 subsystems");return 0
if __name__=="__main__":raise SystemExit(main())
