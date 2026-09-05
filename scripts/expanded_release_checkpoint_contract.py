#!/usr/bin/env python3
"""Build the truthful Sprint 156 expanded first-GA checkpoint corpus."""
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-156/expanded-release-checkpoint-corpus.json"
DOCUMENTS=("README.md","PRD.md","Agent-Scaffolding-Inventory.md","TASKS.md","SECURITY-REVIEW.md","RUNTIME-BOUNDARIES.md","DELIVERY-SYSTEM.md","PRODUCTIVITY-SYSTEM.md","WINDOWS-BOUNDARIES.md","SECURITY.md")
BLOCKERS=("failed","skipped","stale","unavailable","flaky","quarantined","suppressed","unreconciled","unreviewed","unsigned")
DOMAINS=("platform","provider","account","operation","autonomy","finance","cloud","privacy","security","accessibility","recovery","removal","support","evidence","package","documentation")
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def build():
 reports=[]
 for path in sorted((ROOT/"artifacts/sprints").glob("sprint-*/local-evidence-report.json"),key=lambda item:int(item.parent.name.split("-")[1])):
  if path.parent.name=="sprint-156":continue
  value=json.loads(path.read_text());reports.append({"sprint":int(path.parent.name.split("-")[1]),"status":value["summary"]["sprint_status"],"sha256":sha(path)})
 cases=[]
 for index in range(512):
  cases.append({"id":f"AT-GA-002-{index+1:04d}","domain":DOMAINS[index%len(DOMAINS)],"blocker":BLOCKERS[(index//len(DOMAINS))%len(BLOCKERS)],"visible":True,"checkpoint_closed":False,"g_ga_closed":False,"package_produced":False,"publication_allowed":False,"support_promoted":False})
 status=json.loads((ROOT/"architecture/status-model.json").read_text())
 value={"schema_version":1,"documents":[{"path":path,"sha256":sha(ROOT/path),"support_claim":"unsupported-pre-release"}for path in DOCUMENTS],"local_evidence_reports":reports,"local_evidence_report_count":len(reports),"blockers":list(BLOCKERS),"domains":list(DOMAINS),"cases":cases,"case_count":len(cases),"requirement_count":len(json.loads((ROOT/"requirements/registry.json").read_text())["requirements"]),"traceability_count":len(json.loads((ROOT/"requirements/traceability-report.json").read_text())["requirements"]),"current_product_lifecycle":status["current_product"]["lifecycle_status"],"supported_platform_count":sum(item["support_status"]=="supported" for item in status["platforms"]),"enabled_model_count":sum(item["enabled"] for item in status["models"]),"signed_manifest_count":0,"native_reproduction_count":0,"reviewer_signature_count":0,"user_approval_count":0,"release_package_count":0,"checkpoint_closure_count":0,"promotion_count":0}
 return(json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def check():
 expected=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=expected:raise RuntimeError("expanded checkpoint corpus stale")
 value=json.loads(expected)
 if value["requirement_count"]!=294 or value["traceability_count"]!=294 or value["local_evidence_report_count"]!=142:raise RuntimeError("checkpoint inventory drift")
 if len(value["documents"])!=10 or any(item["support_claim"]!="unsupported-pre-release" for item in value["documents"]):raise RuntimeError("documentation truth drift")
 if any(not case["visible"]or case["checkpoint_closed"]or case["g_ga_closed"]or case["package_produced"]or case["publication_allowed"]or case["support_promoted"] for case in value["cases"]):raise RuntimeError("release blocker drift")
 for key in ("supported_platform_count","enabled_model_count","signed_manifest_count","native_reproduction_count","reviewer_signature_count","user_approval_count","release_package_count","checkpoint_closure_count","promotion_count"):
  if value[key]!=0:raise RuntimeError(f"checkpoint overclaim: {key}")
def main():
 parser=argparse.ArgumentParser();parser.add_argument("--write",action="store_true");args=parser.parse_args()
 if args.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();print("validated 142 local reports and 512 AT-GA-002 blocker cases without checkpoint or release authority");return 0
if __name__=="__main__":raise SystemExit(main())
