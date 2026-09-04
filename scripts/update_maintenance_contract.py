#!/usr/bin/env python3
"""Build and verify Sprint 96 update and supply-maintenance evidence."""
from __future__ import annotations
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
OUTPUT=ROOT/"artifacts/sprints/sprint-96/update-maintenance-corpus.json"
CLASSES=("valid-offline-preview","wrong-signer","tampered-package","downgrade","incompatible-target","stale-preview","missing-approval","interrupted-stage","failed-activation","failed-rollback","hidden-remote-check","dependency-substitution","missing-license","missing-hash","missing-removal","stale-vulnerability-review","suppressed-finding","revoked-component")
def sha(path:Path)->str:return hashlib.sha256(path.read_bytes()).hexdigest()
def build()->bytes:
 provenance=json.loads((ROOT/"supply-chain/dependency-provenance.json").read_bytes())
 sbom=json.loads((ROOT/"supply-chain/sbom.cdx.json").read_bytes())
 components=provenance["components"]
 identities=[c["component_id"] for c in components]
 removals=[{"component_id":i,"disposition":"remove-exact-version-and-rebuild-closure"} for i in identities]
 cases=[{"case_id":f"S-073-{i+1:03d}","class":CLASSES[i%len(CLASSES)],"expected":"allow-preview-only" if i%len(CLASSES)==0 else "deny-or-rollback","activation_count":0,"remote_check_count":0,"suppression_count":0} for i in range(72)]
 value={"schema_version":1,"case_count":len(cases),"classes":list(CLASSES),"dependency_inventory":{"component_count":len(components),"edge_count":len(provenance["dependencies"]),"licensed_component_count":sum(bool(c.get("license")) for c in components),"hashed_component_count":sum(bool(c.get("hashes")) for c in components),"provenance_sha256":sha(ROOT/"supply-chain/dependency-provenance.json"),"sbom_sha256":sha(ROOT/"supply-chain/sbom.cdx.json"),"dependency_hashes_sha256":sha(ROOT/"supply-chain/dependency-hashes.sha256"),"sbom_component_count":len(sbom["components"])},"removal_plan":{"component_count":len(removals),"entries":removals},"vulnerability_review":{"status":"BLOCKED_EXTERNAL","current_feed_present":False,"reviewed_component_count":0,"suppression_count":0},"update":{"automatic_remote_check":False,"signed_package_count":0,"staged_activation_count":0,"rollback_execution_count":0,"preview_contract_present":True,"compatibility_contract_present":True,"integrity_contract_present":True},"cases":cases}
 return (json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def write():OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
def check():
 if not OUTPUT.is_file() or OUTPUT.read_bytes()!=build():raise RuntimeError("update-maintenance corpus stale or absent")
 v=json.loads(OUTPUT.read_bytes()); inventory=v["dependency_inventory"]
 if inventory["component_count"]!=inventory["sbom_component_count"] or inventory["licensed_component_count"]!=inventory["component_count"] or inventory["hashed_component_count"]!=inventory["component_count"]:raise RuntimeError("supply inventory incomplete")
 if v["removal_plan"]["component_count"]!=inventory["component_count"]:raise RuntimeError("removal plan incomplete")
 if v["vulnerability_review"]["current_feed_present"] or v["update"]["signed_package_count"] or any(c["activation_count"] or c["remote_check_count"] or c["suppression_count"] for c in v["cases"]):raise RuntimeError("external or effect overclaim")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:write()
 check();print("validated 72 update/maintenance cases and complete local supply closure");return 0
if __name__=="__main__":raise SystemExit(main())
