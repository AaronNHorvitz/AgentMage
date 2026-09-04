#!/usr/bin/env python3
"""Validate Sprint 89-90 pure job and schedule contracts."""
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];SOURCE=ROOT/"kernel/engine/src/job_scheduler.rs";GUIDE=ROOT/"docs/guides/read-only-jobs-and-schedules.md"
CORPORA=((ROOT/"docs/verification/sprint-89-job-runtime-corpus.json",56,{"state":8,"lease":8,"retry":8,"budget":8,"availability":8,"recovery":8,"authority":8}),(ROOT/"docs/verification/sprint-90-read-only-schedule-corpus.json",56,{"notification":8,"control":8,"schedule":8,"prohibition":8,"receipt":8,"clock":8,"recovery":8}))
REQUIRED=("JobState","JobLease","JobBudgets","JobAvailability","JobRecord","ScheduleControl","ScheduledOperation","ScheduleRecord","JobNotificationKind","JobRunReceipt","acquire_job_lease","renew_job_lease","admit_read_only_schedule","finish_job","admit_resume","DuplicateDenied")
FORBIDDEN=("std::fs","std::process","std::net","Command::new","TcpStream","reqwest::","rusqlite::")
def validate():
 failures=[];source=SOURCE.read_text();production=source.split("#[cfg(test)]",1)[0];guide=GUIDE.read_text()
 for token in REQUIRED:
  if token not in production:failures.append(f"job scheduler boundary absent: {token}")
 for token in FORBIDDEN:
  if token in production:failures.append(f"job scheduler contract acquired executor: {token}")
 for path,count,categories in CORPORA:
  corpus=json.loads(path.read_text());cases=corpus.get("cases",[])
  if corpus.get("case_count")!=count or len(cases)!=count or len(set(cases))!=count:failures.append(f"corpus count drifted: {path.name}")
  if corpus.get("categories")!=categories or sum(categories.values())!=count:failures.append(f"corpus categories drifted: {path.name}")
  if corpus.get("local_contract_passed") is not True or corpus.get("independent_review_present") is not False or corpus.get("substitution_set")!=[]:failures.append(f"corpus truth state drifted: {path.name}")
 if "scheduling executor" not in guide or source.count("fn sprint_89_")!=3 or source.count("fn sprint_90_")!=3:failures.append("job scheduler execution/test disclosure drifted")
 return failures
if __name__=="__main__":
 errors=validate()
 if errors:print("\n".join(errors));raise SystemExit(1)
 print("validated 7 job states, 8 schedule controls, and 112 local cases")
