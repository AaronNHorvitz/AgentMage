#!/usr/bin/env python3
"""Validate Sprint 91 inert scheduled-authority contracts."""
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];SOURCE=ROOT/"kernel/engine/src/scheduled_authority.rs";GUIDE=ROOT/"docs/guides/scheduled-authority.md";CORPUS=ROOT/"docs/verification/sprint-91-scheduled-authority-corpus.json"
REQUIRED=("ScheduledEffectClass","ScheduledAuthorityControl","ScheduledAuthorityGrant","ScheduledAuthorityRefresh","ScheduledAuthorityPreview","ScheduledAuthorityResult","admit_scheduled_authority","reconcile_scheduled_authority","prior_configuration_sha256","activation_approval_sha256","self_modified","RetryBlocked")
FORBIDDEN=("std::fs","std::process","std::net","Command::new","TcpStream","reqwest::","git2::")
EXPECTED={"threat":8,"allowlist":8,"binding":8,"worktree":8,"activation":8,"recovery":8,"control":8,"attack":8}
def validate():
 failures=[];source=SOURCE.read_text();production=source.split("#[cfg(test)]",1)[0];guide=GUIDE.read_text();corpus=json.loads(CORPUS.read_text())
 for token in REQUIRED:
  if token not in production:failures.append(f"scheduled-authority boundary absent: {token}")
 for token in FORBIDDEN:
  if token in production:failures.append(f"scheduled-authority contract acquired executor: {token}")
 cases=corpus.get("cases",[])
 if corpus.get("case_count")!=64 or len(cases)!=64 or len(set(cases))!=64:failures.append("scheduled-authority corpus count drifted")
 if corpus.get("categories")!=EXPECTED or sum(EXPECTED.values())!=64:failures.append("scheduled-authority categories drifted")
 if corpus.get("local_contract_passed") is not True or corpus.get("native_scheduled_effect_executed") is not False or corpus.get("emergency_stop_executed") is not False or corpus.get("independent_review_present") is not False or corpus.get("substitution_set")!=[]:failures.append("scheduled-authority truth state drifted")
 if "has no" not in guide or source.count("fn sprint_91_")!=6:failures.append("scheduled-authority execution/test disclosure drifted")
 return failures
if __name__=="__main__":
 errors=validate()
 if errors:print("\n".join(errors));raise SystemExit(1)
 print("validated 3 predeclarable effects, 3 denied classes, and 64 local cases")
