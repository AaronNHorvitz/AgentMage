#!/usr/bin/env python3
"""Validate Sprint 87-88 inert connector mutation contracts."""
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]; SOURCE=ROOT/"capabilities/knowledge/src/connector_mutation.rs"; GUIDE=ROOT/"docs/guides/connector-governance-and-recovery.md"
CORPORA=((ROOT/"docs/verification/sprint-87-connector-isolation-corpus.json",56,{"manifest":8,"stage":8,"credential":8,"isolation":8,"scope":8,"authority":8,"failure":8}),(ROOT/"docs/verification/sprint-88-connector-write-corpus.json",56,{"write":8,"grant":8,"isolation":8,"result":8,"recovery":8,"receipt":8,"authority":8}))
REQUIRED=("ConnectorKind","ConnectorControlManifest","ConnectorOperation","ConnectorRemoteRefresh","ConnectorMutationRequest","ConnectorMutationPreview","ConnectorMutationReceipt","admit_connector_mutation","reconcile_connector_mutation","derived_credential_sha256","idempotency_key_sha256","RetryBlocked")
FORBIDDEN=("std::fs","std::process","Command::new","TcpStream","reqwest::","sqlx::","rusqlite::")
def validate():
 failures=[]; source=SOURCE.read_text(); production=source.split("#[cfg(test)]",1)[0]; guide=GUIDE.read_text()
 for token in REQUIRED:
  if token not in production: failures.append(f"connector mutation boundary absent: {token}")
 for token in FORBIDDEN:
  if token in production: failures.append(f"connector mutation contract acquired executor: {token}")
 for path,count,categories in CORPORA:
  corpus=json.loads(path.read_text());cases=corpus.get("cases",[])
  if corpus.get("case_count")!=count or len(cases)!=count or len(set(cases))!=count: failures.append(f"corpus count drifted: {path.name}")
  if corpus.get("categories")!=categories or sum(categories.values())!=count: failures.append(f"corpus categories drifted: {path.name}")
  if corpus.get("local_contract_passed") is not True or corpus.get("native_mutation_executed") is not False or corpus.get("independent_review_present") is not False or corpus.get("substitution_set")!=[]: failures.append(f"corpus truth state drifted: {path.name}")
 if "never blindly retried" not in guide or source.count("fn sprint_87_")!=3 or source.count("fn sprint_88_")!=3: failures.append("connector recovery/test disclosure drifted")
 return failures
if __name__=="__main__":
 errors=validate()
 if errors:print("\n".join(errors));raise SystemExit(1)
 print("validated 7 connector families, 7 write classes, and 112 local cases")
