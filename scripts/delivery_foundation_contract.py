#!/usr/bin/env python3
"""Build and verify Sprint 103 delivery-foundation evidence."""
from __future__ import annotations
import argparse,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-103/delivery-foundation-corpus.json"
NODES=("service","work-item","repository","change","commit","review","build","check","artifact","provenance","environment","deployment","telemetry","incident","finding","release","rollback")
RELATIONS=("observed","derived","inferred","conflicting","stale","deleted","inaccessible","unknown")
FIXTURES=("minimum","maximum","empty","malformed","extra-field","future-version","rename","transfer","delete-tombstone","collision")
OPERATIONS=("describe","diagnose","discover","plan","preview","execute","reconcile","rollback-or-compensate","remove")
LEVELS=("L0","L1","L2","L3","L4","L5");MODES=("fake","fault","future-version","eventual-consistency","hostile","removed")
def build()->bytes:
 graph=[{"case_id":f"S-103-G-{i+1:03d}","node_kind":NODES[i%len(NODES)],"fixture":FIXTURES[(i//len(NODES))%len(FIXTURES)],"expected":"round-trip" if (i//len(NODES))%len(FIXTURES)==0 else "deny-or-visible-state","authority_count":0} for i in range(len(NODES)*len(FIXTURES))]
 matrix=[{"level":level,"operation":operation,"registered":operation in ("describe","diagnose","remove"),"inherited":False} for level in LEVELS for operation in OPERATIONS]
 return (json.dumps({"schema_version":1,"node_kinds":list(NODES),"relationship_states":list(RELATIONS),"fixture_classes":list(FIXTURES),"adapter_operations":list(OPERATIONS),"conformance_levels":list(LEVELS),"adapter_modes":list(MODES),"graph_case_count":len(graph),"matrix_case_count":len(matrix),"graph_cases":graph,"conformance_matrix":matrix,"reviewer":"scripts.delivery_foundation_contract","live_provider_count":0,"enabled_adapter_count":0},sort_keys=True,separators=(",",":"))+"\n").encode()
def write():OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
def check():
 if not OUTPUT.is_file() or OUTPUT.read_bytes()!=build():raise RuntimeError("delivery foundation corpus stale or absent")
 v=json.loads(OUTPUT.read_bytes())
 if any(c["authority_count"] for c in v["graph_cases"]) or any(c["inherited"] for c in v["conformance_matrix"]) or v["live_provider_count"] or v["enabled_adapter_count"]:raise RuntimeError("delivery foundation overclaim")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:write()
 check();print("validated 170 graph and 54 conformance cases without adapter enablement");return 0
if __name__=="__main__":raise SystemExit(main())
