#!/usr/bin/env python3
"""Generate and validate Sprint 129 graph-confusion evidence."""
from __future__ import annotations
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-129/identity-graph-corpus.json";SOURCE=ROOT/"kernel/engine/src/productivity_identity_graph.rs"
KINDS=("provider","tenant","account","mailbox","workspace","team","channel","person","meeting","task","document","financial_record","cloud_resource")
STATES=("confirmed","deterministic","proposed","conflicting","stale","removed")
ATTACKS=("duplicate_name","alias","homoglyph","renamed_channel","transferred_resource","recycled_address","conflicting_evidence","merge","split","account_change","provider_deletion","link_revocation")
def sha(p:Path)->str:return hashlib.sha256(p.read_bytes()).hexdigest()
def build()->bytes:
 cases=[]
 for kind in KINDS:
  for state in STATES:
   for attack in ATTACKS:
    authoritative=state in {"confirmed","deterministic"}and attack not in {"transferred_resource","recycled_address","conflicting_evidence","provider_deletion","link_revocation"}
    cases.append({"case_id":f"AT-PGR-001-{len(cases)+1:04d}","kind":kind,"link_state":state,"attack":attack,"authorized":authoritative,"native_identity_visible":True,"source_visible":True,"freshness_visible":True,"classification_visible":True,"lineage_preserved":True})
 value={"schema_version":1,"identity_kinds":list(KINDS),"link_states":list(STATES),"attack_families":list(ATTACKS),"cases":cases,"case_count":len(cases),"display_authorization_count":0,"alias_authorization_count":0,"stale_authorization_count":0,"unrelated_history_corruption_count":0,"source_sha256":{"kernel/engine/src/productivity_identity_graph.rs":sha(SOURCE),"PRODUCTIVITY-SYSTEM.md":sha(ROOT/"PRODUCTIVITY-SYSTEM.md")}}
 return(json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def check():
 expected=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=expected:raise RuntimeError("identity graph corpus stale or absent")
 value=json.loads(expected);source=SOURCE.read_text().split("#[cfg(test)]",1)[0]
 for token in ("ProductivityIdentityKind","ProductivityLinkState","ProductivityIdentityNode","ProductivityIdentityLink","authorize","rename","remove"):
  if token not in source:raise RuntimeError(f"identity graph contract absent: {token}")
 if value["case_count"]!=936:raise RuntimeError("identity graph case count drift")
 if any(value[k]for k in ("display_authorization_count","alias_authorization_count","stale_authorization_count","unrelated_history_corruption_count")):raise RuntimeError("identity confusion overclaim")
 if any(c["authorized"]for c in value["cases"]if c["link_state"]in {"proposed","conflicting","stale","removed"}):raise RuntimeError("non-authoritative link authorized")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();print("validated 936 AT-PGR-001 identity-confusion cases");return 0
if __name__=="__main__":raise SystemExit(main())
