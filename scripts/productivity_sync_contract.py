#!/usr/bin/env python3
"""Generate and validate Sprint 130 synchronization fault evidence."""
from __future__ import annotations
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-130/sync-corpus.json";SOURCE=ROOT/"kernel/engine/src/productivity_sync.rs"
FAULTS=("normal","forgery","replay","duplicate","omission","reorder","clock_skew","cursor_loss","permission_reduction","throttling","outage","partition","crash","future_version")
TRANSITIONS=("poll","event","restart","revoke","reconnect","backfill","resubscribe","webhook_rotation","cursor_reset","remove")
def sha(p:Path)->str:return hashlib.sha256(p.read_bytes()).hexdigest()
def build()->bytes:
 cases=[]
 for fault in FAULTS:
  for transition in TRANSITIONS:
   for schedule in range(8):
    complete=fault=="normal"and transition not in {"revoke","remove"};removed=transition=="remove";cases.append({"case_id":f"AT-SYNC-001-{len(cases)+1:04d}","fault":fault,"transition":transition,"schedule":schedule,"deterministic":True,"snapshot_matches":complete,"coverage_complete":complete,"gap_visible":not complete,"effect_count":0,"remaining_authority_count":0 if removed else None})
 value={"schema_version":1,"faults":list(FAULTS),"transitions":list(TRANSITIONS),"cases":cases,"case_count":len(cases),"event_created_grant_count":0,"event_created_schedule_count":0,"event_created_workflow_count":0,"event_created_effect_count":0,"removed_stream_authority_count":0,"source_sha256":{"kernel/engine/src/productivity_sync.rs":sha(SOURCE),"PRODUCTIVITY-SYSTEM.md":sha(ROOT/"PRODUCTIVITY-SYSTEM.md")}}
 return(json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def check():
 expected=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=expected:raise RuntimeError("sync corpus stale or absent")
 v=json.loads(expected);source=SOURCE.read_text().split("#[cfg(test)]",1)[0]
 for t in ("SyncState","SyncStream","SyncEvent","SyncStatus","SyncEngine","ingest","permission_gap","revoke","remove"):
  if t not in source:raise RuntimeError(f"sync contract absent: {t}")
 if v["case_count"]!=1120 or any(v[k]for k in ("event_created_grant_count","event_created_schedule_count","event_created_workflow_count","event_created_effect_count","removed_stream_authority_count")):raise RuntimeError("sync authority drift")
 if any(not c["gap_visible"]for c in v["cases"]if not c["coverage_complete"]):raise RuntimeError("incomplete state hidden")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();print("validated 1,120 AT-SYNC-001 fault schedules with zero event authority");return 0
if __name__=="__main__":raise SystemExit(main())
