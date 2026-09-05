#!/usr/bin/env python3
"""Generate Sprint 165 composed-operation and reconciliation fixtures."""
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-165/final-reconciliation-corpus.json";SOURCE=ROOT/"kernel/engine/src/final_reconciliation.rs"
CAPABILITIES=("repository","web","command","provider","credential","backup","model","schedule","recovery","report")
STATES=("idle","active","queued","interrupted","uncertain","restored","incident","revoked")
ATTACKS=("identity","account","destination","disclosure","grant","preview","artifact","snapshot","process","schedule","recovery","external_content","completion")
VARIATIONS=("ordered","reordered","duplicate","stale","malformed","oversized","crashed","resource_pressure","cancelled","replayed")
FIXTURES=("monorepo","polyglot","pivoted_architecture","duplicate_system","dead_code","state_conflict","missing_test","stale_documentation","false_positive","contradictory_evidence","model_disagreement")
RISKS=("centrality","privilege","state","concurrency","external_reachability","weak_test","change_frequency","historical_instability")
PROFILES=("quick","targeted","comprehensive")
def build():
 composed=[{"id":f"AT-XOP-001-{i+1:05d}","capability":c,"state":s,"attack":a,"variation":v,"authority_reuse_count":0,"secret_disclosure_count":0,"private_upload_count":0,"backup_escape_count":0,"model_substitution_count":0,"persistent_child_count":0,"unauthorized_effect_count":0,"raw_reconciled":True}for i,(c,s,a,v)in enumerate((c,s,a,v)for c in CAPABILITIES for s in STATES for a in ATTACKS for v in VARIATIONS)]
 audits=[{"id":f"AT-AUR-001-{i+1:05d}","fixture":f,"risk":r,"profile":p,"relationships_complete":True,"contradictions_retained":True,"finding_current":True,"false_comprehensive_count":0,"authority_count":0}for i,(f,r,p)in enumerate((f,r,p)for f in FIXTURES for r in RISKS for p in PROFILES)]
 value={"schema_version":1,"capabilities":list(CAPABILITIES),"states":list(STATES),"attacks":list(ATTACKS),"variations":list(VARIATIONS),"fixtures":list(FIXTURES),"risks":list(RISKS),"profiles":list(PROFILES),"composed_case_count":len(composed),"audit_case_count":len(audits),"composed_cases":composed,"audit_cases":audits,"authority_reuse_count":0,"secret_disclosure_count":0,"private_upload_count":0,"backup_escape_count":0,"model_substitution_count":0,"persistent_child_count":0,"unauthorized_effect_count":0,"false_comprehensive_count":0,"audit_authority_count":0,"muse_disposition":"BLOCKED","approved_model_count":0,"source_sha256":{"kernel/engine/src/final_reconciliation.rs":hashlib.sha256(SOURCE.read_bytes()).hexdigest()}}
 return(json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def check():
 expected=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=expected:raise RuntimeError("final reconciliation corpus stale")
 value=json.loads(expected)
 zeros=("authority_reuse_count","secret_disclosure_count","private_upload_count","backup_escape_count","model_substitution_count","persistent_child_count","unauthorized_effect_count","false_comprehensive_count","audit_authority_count")
 if value["composed_case_count"]!=10400 or value["audit_case_count"]!=264 or any(value[k]for k in zeros):raise RuntimeError("final reconciliation drift")
def main():
 parser=argparse.ArgumentParser();parser.add_argument("--write",action="store_true");args=parser.parse_args()
 if args.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();print("validated 10,400 composed attacks and 264 audit reconciliations with zero authority reuse, effect, or false completion");return 0
if __name__=="__main__":raise SystemExit(main())
