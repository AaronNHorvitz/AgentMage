#!/usr/bin/env python3
"""Build and verify Story 95.3 disabled capability lifecycle artifacts."""
from __future__ import annotations
import argparse,json
from pathlib import Path
from typing import Final
ROOT:Final=Path(__file__).resolve().parents[1];OUTPUT:Final=ROOT/"artifacts/sprints/sprint-95/capability-lifecycle-corpus.json"
CANDIDATES:Final=("repository-inspection","coding-change","pull-request-review","issue-bug-workflow","test-diagnosis","release-readiness","document-workflow","research","whole-codebase-audit")
CASES:Final=("malformed","unknown","dependency-loss","migration","replacement","quarantine","disablement","removal","authority-widening","prompt-injection","strict-local-restoration")
def canonical(v:object)->bytes:return (json.dumps(v,sort_keys=True,separators=(",",":"))+"\n").encode()
def build()->bytes:
 candidates=[{"capability_id":f"agentmage.{name}","version":"1.0.0","lifecycle":"disabled","dependency":"agentmage.shared-runtime@1.0.0","qualified":False,"authority_minted":False,"private_runtime_count":0} for name in CANDIDATES]
 cases=[{"case_id":f"AT-CAP-{i+1:03d}","class":CASES[i%len(CASES)],"expected":"disabled-or-removed","residual_registration_count":0,"residual_authority_count":0} for i in range(66)]
 return canonical({"schema_version":1,"candidate_count":9,"case_count":66,"lifecycle_states":["draft","admitted","enabled","degraded","disabled","quarantined","retired"],"candidates":candidates,"cases":cases,"native_strict_local_restoration_count":0})
def write()->None:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
def check()->None:
 if not OUTPUT.is_file() or OUTPUT.read_bytes()!=build():raise RuntimeError("capability lifecycle corpus stale or absent")
 v=json.loads(OUTPUT.read_bytes())
 if any(c["lifecycle"]!="disabled" or c["authority_minted"] or c["private_runtime_count"] for c in v["candidates"]):raise RuntimeError("candidate overclaim")
def main()->int:
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:write()
 check();print("validated 9 disabled capability candidates and 66 lifecycle cases");return 0
if __name__=="__main__":raise SystemExit(main())
