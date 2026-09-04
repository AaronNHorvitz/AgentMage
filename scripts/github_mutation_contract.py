#!/usr/bin/env python3
"""Validate Sprint 85-86 inert GitHub mutation contracts."""
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
SOURCE=ROOT/"capabilities/knowledge/src/github_mutation.rs"
GUIDE=ROOT/"docs/guides/github-mutation-and-recovery.md"
CORPORA=((ROOT/"docs/verification/sprint-85-github-mutation-corpus.json",56,{"draft":8,"refresh":8,"grant":8,"capability":8,"signed_push":8,"prohibition":8,"authority":8}),(ROOT/"docs/verification/sprint-86-github-recovery-corpus.json",48,{"idempotency":8,"result":8,"recovery":8,"receipt":8,"attack":8,"retry":8}))
REQUIRED=("GithubMutationKind","GithubPushBinding","HostedRefresh","GithubMutationRequest","GithubMutationPreview","GithubMutationReceipt","ProhibitedGithubOperation","admit_github_mutation","reconcile_github_mutation","idempotency_key_sha256","expected_effect_sha256","commit_approval_sha256","push_approval_sha256","RetryBlocked")
FORBIDDEN=("std::fs","std::process","std::net","Command::new","TcpStream","reqwest::","git2::")
def validate():
 failures=[]; source=SOURCE.read_text(); production=source.split("#[cfg(test)]",1)[0]; guide=GUIDE.read_text()
 for token in REQUIRED:
  if token not in production: failures.append(f"GitHub mutation boundary absent: {token}")
 for token in FORBIDDEN:
  if token in production: failures.append(f"GitHub mutation contract acquired executor: {token}")
 for path,count,categories in CORPORA:
  corpus=json.loads(path.read_text()); cases=corpus.get("cases",[])
  if corpus.get("case_count")!=count or len(cases)!=count or len(set(cases))!=count: failures.append(f"corpus count drifted: {path.name}")
  if corpus.get("categories")!=categories or sum(categories.values())!=count: failures.append(f"corpus categories drifted: {path.name}")
  if corpus.get("local_contract_passed") is not True or corpus.get("native_mutation_executed") is not False or corpus.get("independent_review_present") is not False or corpus.get("substitution_set")!=[]: failures.append(f"corpus truth state drifted: {path.name}")
 if "never blindly retried" not in guide or source.count("fn sprint_85_")!=3 or source.count("fn sprint_86_")!=3: failures.append("GitHub mutation recovery/test disclosure drifted")
 return failures
if __name__=="__main__":
 errors=validate()
 if errors: print("\n".join(errors)); raise SystemExit(1)
 print("validated 14 GitHub mutation classes and 104 local cases")
