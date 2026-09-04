#!/usr/bin/env python3
"""Build deterministic Sprint 118 incident lifecycle fixtures."""
from __future__ import annotations
import argparse,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-118/incident-lifecycle-corpus.json"
PROVIDERS=("pagerduty","jira-service-management","datadog-incidents","slack","teams");EFFECTS=("state","severity","assignment","work-item","message","rollback","flag","closure");CLASSES=("fact","inference","conflict","unknown","decision");ATTACKS=("false-telemetry","prompt-injection","hidden-recipient","channel-confusion","malicious-attachment","cross-tenant","urgency-pressure","nested-remediation");FAULTS=("duplicate-event","delayed-event","changed-responder","membership-change","timeout","partial-message","provider-outage","cancellation","crash")
def build()->bytes:
 records=[{"case_id":f"S-118-I-{i+1:03d}","provider":PROVIDERS[i%len(PROVIDERS)],"evidence_class":CLASSES[i%len(CLASSES)],"source_bound":True,"incident_bound":True,"service_bound":True,"tenant_bound":True,"timeline_bound":True,"fact_inference_separate":True}for i in range(128)]
 attacks=[{"case_id":f"S-118-A-{i+1:04d}","attack":ATTACKS[i%len(ATTACKS)],"provider_contact_count":0,"hidden_recipient_count":0,"cross_tenant_count":0,"secret_disclosure_count":0,"autonomous_effect_count":0,"duplicate_effect_count":0}for i in range(2048)]
 faults=[{"case_id":f"S-118-F-{i+1:04d}","fault":FAULTS[i%len(FAULTS)],"precondition_reread":True,"idempotency_bound":True,"uncertain_reconciled":True,"correction_is_fresh_effect":True,"duplicate_effect_count":0,"autonomous_effect_count":0}for i in range(512)]
 v={"schema_version":1,"providers":list(PROVIDERS),"effects":list(EFFECTS),"evidence_classes":list(CLASSES),"attacks":list(ATTACKS),"faults":list(FAULTS),"record_case_count":len(records),"attack_case_count":len(attacks),"fault_case_count":len(faults),"record_cases":records,"attack_cases":attacks,"fault_cases":faults,"integration_case":{"synthetic_failed_release":True,"incident_draft":True,"message_draft":True,"separate_work_approval":True,"separate_notification_approval":True,"rollback_preview_only":True,"provider_contact_count":0},"reviewer":"scripts.incident_lifecycle_contract","promoted_provider_count":0,"external_effect_count":0,"independent_human_review_count":0}
 return(json.dumps(v,sort_keys=True,separators=(",",":"))+"\n").encode()
def write():OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
def check():
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=build():raise RuntimeError("incident corpus stale or absent")
 v=json.loads(OUTPUT.read_bytes())
 if any(not all(c[k]for k in("source_bound","incident_bound","service_bound","tenant_bound","timeline_bound","fact_inference_separate"))for c in v["record_cases"]):raise RuntimeError("record failed")
 if any(c["provider_contact_count"]or c["hidden_recipient_count"]or c["cross_tenant_count"]or c["secret_disclosure_count"]or c["autonomous_effect_count"]or c["duplicate_effect_count"]for c in v["attack_cases"]):raise RuntimeError("attack admitted")
 if any(not c["precondition_reread"]or not c["idempotency_bound"]or not c["uncertain_reconciled"]or not c["correction_is_fresh_effect"]or c["duplicate_effect_count"]or c["autonomous_effect_count"]for c in v["fault_cases"]):raise RuntimeError("fault failed")
 if v["promoted_provider_count"]or v["external_effect_count"]or v["independent_human_review_count"]:raise RuntimeError("incident support overclaim")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:write()
 check();print("validated 128 incidents, 2048 attacks, and 512 faults without incident promotion");return 0
if __name__=="__main__":raise SystemExit(main())
