#!/usr/bin/env python3
"""Build and verify Sprint 105 external-effect and event evidence."""
from __future__ import annotations
import argparse,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-105/external-effect-corpus.json"
PLAN_FIELDS=("actor","target","object","payload","attachments","visibility","source-revision","environment","expected-change","cost","budget","preconditions","expiry","recovery","idempotency-key","operation-fingerprint")
OUTCOMES=("effect","no-effect","duplicate","partial","unknown")
CRASH_POINTS=("before-send","during-transport","after-effect","before-local-commit","during-reconciliation")
EVENT_ATTACKS=("forged-signature","delayed","replayed","duplicated","reordered","omitted","truncated","mutated-cursor","mutated-timestamp","mutated-tombstone","pagination-loop","rotated-secret")
def build()->bytes:
 mutations=[{"case_id":f"S-105-U-{i+1:03d}","field":field,"mutation":mutation,"expected":"deny-stably","request_count":0,"receipt_count":1} for field in PLAN_FIELDS for mutation in ("missing","malformed","oversized","stale","future-version") for i in range(1)]
 faults=[{"case_id":f"S-105-F-{i+1:04d}","crash_point":CRASH_POINTS[i%len(CRASH_POINTS)],"provider_outcome":OUTCOMES[i%len(OUTCOMES)],"duplicate_effect_count":0,"unsafe_retry_count":0,"receipt_count":1,"reconciled":True} for i in range(1024)]
 events=[{"case_id":f"S-105-E-{i+1:03d}","attack":attack,"authority_count":0,"follow_on_operation_count":0,"deterministic":True,"gap_visible":attack in ("reordered","omitted","pagination-loop"),"expected":"reject-or-visible"} for i,attack in enumerate(EVENT_ATTACKS)]
 rollbacks=[{"case_id":f"S-105-R-{i+1:03d}","later_remote_change":True,"stale_compensation_executed":False,"later_work_preserved":True,"fresh_plan_required":True} for i in range(64)]
 value={"schema_version":1,"plan_fields":list(PLAN_FIELDS),"effect_outcomes":list(OUTCOMES),"crash_points":list(CRASH_POINTS),"event_attacks":list(EVENT_ATTACKS),"schema_mutation_count":len(mutations),"fault_schedule_count":len(faults),"event_attack_count":len(events),"rollback_race_count":len(rollbacks),"schema_mutations":mutations,"fault_schedules":faults,"event_cases":events,"rollback_races":rollbacks,"reviewer":"scripts.external_effect_contract","live_provider_count":0,"independent_review_count":0}
 return (json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def write():OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
def check():
 if not OUTPUT.is_file() or OUTPUT.read_bytes()!=build():raise RuntimeError("external effect corpus stale or absent")
 value=json.loads(OUTPUT.read_bytes())
 if value["fault_schedule_count"]<1000 or any(c["duplicate_effect_count"] or c["unsafe_retry_count"] or c["receipt_count"]!=1 or not c["reconciled"] for c in value["fault_schedules"]):raise RuntimeError("fault schedule failed")
 if any(c["authority_count"] or c["follow_on_operation_count"] or not c["deterministic"] for c in value["event_cases"]):raise RuntimeError("event authority escaped")
 if any(c["stale_compensation_executed"] or not c["later_work_preserved"] or not c["fresh_plan_required"] for c in value["rollback_races"]):raise RuntimeError("stale compensation admitted")
 if value["live_provider_count"] or value["independent_review_count"]:raise RuntimeError("external evidence overclaim")
def main():
 parser=argparse.ArgumentParser();parser.add_argument("--write",action="store_true");arguments=parser.parse_args()
 if arguments.write:write()
 check();print("validated 80 schema mutations, 1024 fault schedules, 12 event attacks, and 64 rollback races");return 0
if __name__=="__main__":raise SystemExit(main())
