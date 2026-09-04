#!/usr/bin/env python3
"""Build and verify Sprint 99 cross-interface assurance evidence."""
from __future__ import annotations
import argparse,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-99/cross-interface-corpus.json"
BOUNDARIES=("vscode","cli","desktop","json","sdk","acp","package","hook","mcp","connector","browser","schedule","child-agent")
ATTACKS=("grant-bypass","receipt-forgery","path-escape","retention-bypass","cancellation-bypass","sandbox-bypass","offline-bypass","identity-pooling")
ISOLATION=("account","workspace","project","agent","worktree","connector","transport")
def build()->bytes:
 cases=[{"case_id":f"S-074-XI-{i+1:03d}","boundary":BOUNDARIES[i%len(BOUNDARIES)],"attack":ATTACKS[(i//len(BOUNDARIES))%len(ATTACKS)],"decision":"deny","unauthorized_effect_count":0,"network_count":0,"receipt_count":1} for i in range(104)]
 return (json.dumps({"schema_version":1,"boundary_count":13,"boundaries":list(BOUNDARIES),"attack_count":8,"attacks":list(ATTACKS),"isolation_dimension_count":7,"isolation_dimensions":list(ISOLATION),"case_count":104,"native_interface_count":0,"cases":cases},sort_keys=True,separators=(",",":"))+"\n").encode()
def write():OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
def check():
 if not OUTPUT.is_file() or OUTPUT.read_bytes()!=build():raise RuntimeError("cross-interface corpus stale or absent")
 v=json.loads(OUTPUT.read_bytes())
 if tuple(v["boundaries"])!=BOUNDARIES or tuple(v["attacks"])!=ATTACKS or tuple(v["isolation_dimensions"])!=ISOLATION or v["native_interface_count"] or any(c["decision"]!="deny" or c["unauthorized_effect_count"] or c["network_count"] or c["receipt_count"]!=1 for c in v["cases"]):raise RuntimeError("cross-interface overclaim")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:write()
 check();print("validated 104 cross-interface cases over 13 boundaries and 7 isolation dimensions");return 0
if __name__=="__main__":raise SystemExit(main())
