#!/usr/bin/env python3
"""Validate Sprint 84 confirmed-computer-use contracts."""
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]; SOURCE=ROOT/"shells/host/src/computer_use.rs"; CORPUS=ROOT/"docs/verification/sprint-84-computer-use-corpus.json"; GUIDE=ROOT/"docs/guides/confirmed-computer-use.md"
REQUIRED=("StructuredToolDisposition","Unavailable","Unsupported","Available","ComputerUseAction","Click","Submit","Upload","Send","ExternalStateChange","ComputerUseRequest","ComputerUsePreview","ComputerUseReceipt","prepare_computer_use","verify_computer_use","foreground_window_sha256","before_screenshot_sha256","after_screenshot_sha256","confirmation_sha256","uncertain","terminated")
FORBIDDEN=("std::fs","std::process","std::net","Command::new","TcpStream","reqwest::")
EXPECTED={"fallback":5,"identity":5,"confirmation":5,"preview":5,"receipt":5,"recovery":5,"authority":5}
def validate():
 failures=[]; source=SOURCE.read_text(); production=source.split("#[cfg(test)]",1)[0]; corpus=json.loads(CORPUS.read_text()); guide=GUIDE.read_text()
 for token in REQUIRED:
  if token not in production: failures.append(f"computer-use boundary absent: {token}")
 for token in FORBIDDEN:
  if token in production: failures.append(f"computer-use contract acquired executor: {token}")
 cases=corpus.get("cases",[])
 if corpus.get("case_count")!=35 or len(cases)!=35 or len(set(cases))!=35: failures.append("computer-use corpus count drifted")
 if corpus.get("categories")!=EXPECTED or sum(EXPECTED.values())!=35: failures.append("computer-use corpus categories drifted")
 if corpus.get("local_contract_passed") is not True or corpus.get("native_action_executed") is not False or corpus.get("before_after_capture_complete") is not False or corpus.get("privacy_review_present") is not False or corpus.get("substitution_set")!=[]: failures.append("computer-use corpus truth state drifted")
 if "never blindly repeated" not in guide: failures.append("uncertain-outcome disclosure drifted")
 if source.count("fn sprint_84_")!=4: failures.append("Sprint 84 focused test inventory drifted")
 return failures
if __name__=="__main__":
 errors=validate()
 if errors: print("\n".join(errors)); raise SystemExit(1)
 print("validated 5 computer-use actions and 35 confirmation cases")
