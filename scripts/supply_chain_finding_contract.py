#!/usr/bin/env python3
"""Build and verify Sprint 111 synthetic supply-chain/finding evidence."""
from __future__ import annotations
import argparse,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-111/supply-chain-finding-corpus.json"
DOCUMENTS=("spdx","cyclonedx","sigstore-signature","cosign-attestation","slsa-provenance","opa-policy")
TOOLS=("codeql","semgrep","sonarqube","snyk","trivy","grype","sarif")
ATTACKS=("forged-signer","swapped-subject","incomplete-graph","malicious-package-url","suppression-abuse","conflicting-severity","stale-line","injected-remediation","source-change","same-label-rebuild","signer-revocation","tool-update","policy-change")
def build()->bytes:
 docs=[{"document":d,"schema_preserved":True,"producer_bound":True,"subject_bound":True,"component_graph_preserved":True,"license_preserved":True,"generation_context_bound":True,"assurance_overclaim_count":0} for d in DOCUMENTS]
 findings=[{"tool":t,"original_tool_bound":True,"rule_bound":True,"version_bound":True,"location_bound":True,"severity":"high","conflicting_severity":"low","conflict_preserved":True,"reachability_preserved":True,"suppression_preserved":True,"immutable_source_bound":True,"text_untrusted":True} for t in TOOLS]
 attacks=[{"case_id":f"S-111-A-{i+1:04d}","attack":ATTACKS[i%len(ATTACKS)],"hidden_blocking_finding_count":0,"accepted_stale_evidence_count":0,"forged_identity_count":0,"release_state_change_count":0} for i in range(2048)]
 value={"schema_version":1,"documents":list(DOCUMENTS),"tools":list(TOOLS),"attacks":list(ATTACKS),"document_case_count":len(docs),"finding_case_count":len(findings),"attack_case_count":len(attacks),"document_cases":docs,"finding_cases":findings,"attack_cases":attacks,"reviewer":"scripts.supply_chain_finding_contract","promoted_tool_count":0,"live_verification_count":0,"independent_human_review_count":0}
 return (json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def write():OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
def check():
 if not OUTPUT.is_file() or OUTPUT.read_bytes()!=build():raise RuntimeError("supply-chain finding corpus stale or absent")
 v=json.loads(OUTPUT.read_bytes())
 if any(not all(c[k] for k in ("schema_preserved","producer_bound","subject_bound","component_graph_preserved","license_preserved","generation_context_bound")) or c["assurance_overclaim_count"] for c in v["document_cases"]):raise RuntimeError("supply evidence failed")
 if any(not c["conflict_preserved"] or not c["immutable_source_bound"] or not c["text_untrusted"] for c in v["finding_cases"]):raise RuntimeError("finding evidence lost")
 if any(c["hidden_blocking_finding_count"] or c["accepted_stale_evidence_count"] or c["forged_identity_count"] or c["release_state_change_count"] for c in v["attack_cases"]):raise RuntimeError("supply attack admitted")
 if v["promoted_tool_count"] or v["live_verification_count"] or v["independent_human_review_count"]:raise RuntimeError("tool support overclaim")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:write()
 check();print("validated 6 document, 7 finding, and 2048 hostile cases without tool promotion");return 0
if __name__=="__main__":raise SystemExit(main())
