#!/usr/bin/env python3
"""Build truthful Sprint 104 local connected-identity evidence."""
from pathlib import Path
from typing import Final
try:from scripts.sprint_evidence_recorder import SprintEvidenceDefinition,main
except ModuleNotFoundError:from sprint_evidence_recorder import SprintEvidenceDefinition,main
ROOT:Final=Path(__file__).resolve().parents[1]
SOURCE_PATHS:Final=("Cargo.toml","Cargo.lock","kernel/engine/Cargo.toml","kernel/engine/src/lib.rs","kernel/engine/src/connected_identity.rs","DELIVERY-SYSTEM.md","artifacts/sprints/sprint-104/connected-identity-corpus.json","scripts/connected_identity_contract.py","tests/test_connected_identity_contract.py","supply-chain/dependency-hashes.sha256","supply-chain/dependency-provenance.json","supply-chain/sbom.cdx.json","scripts/sprint_evidence_recorder.py","scripts/sprint_104_evidence.py","tests/test_sprint_104_evidence.py")
COMMANDS:Final=(("connected-identity-rust-contract",("cargo","test","-p","agentmage-kernel-engine","connected_identity::tests","--lib")),("connected-identity-artifact-contract",("python3","-m","unittest","tests.test_connected_identity_contract")),("dependency-boundary-contract",("python3","scripts/dependency_rules.py")),("supply-chain-currentness",("python3","scripts/supply_chain.py")),("product-ci-contract",("python3","scripts/product_ci.py","--check")),("evidence-tests",("python3","-m","unittest","tests.test_sprint_104_evidence")))
FOCUSED_COMMANDS:Final=tuple(item[0] for item in COMMANDS[:3]);IMPLEMENTED:Final={"identity_axis_count":16,"credential_state_count":6,"capability_class_count":8,"confusion_case_count":2048,"escalation_case_count":336,"canary_surface_count":8,"isolated_worker_pair_count":64,"native_identity_count":0,"live_provider_count":0}
BLOCKERS:Final=({"code":"UPSTREAM-SPRINT-103-BLOCKED","owner":"104"},{"code":"BLOCKED-EXTERNAL-RV24-NATIVE-IDENTITY-REVIEW","owner":"104.1.3.5"})
VERIFICATION:Final={"focused_local_contracts":True,"focused_blocking_skip_count":0,"wrong_domain_request_count":0,"secret_disclosure_count":0,"pairwise_escalation_admitted_count":0,"shared_worker_state_count":0,"reviewer":"scripts.connected_identity_contract","rv24_complete":False,"sprint_gate_closed":False}
SUMMARY:Final={"local_sprint_104_contract_passed":True,"sprint_status":"BLOCKED","upstream_sprint_103_closed":False,"native_identity_support":False,"release_approval":False}
DEFINITION:Final=SprintEvidenceDefinition(sprint=104,root=ROOT,output="artifacts/sprints/sprint-104/local-evidence-report.json",source_paths=SOURCE_PATHS,commands=COMMANDS,focused_commands=FOCUSED_COMMANDS,rust_focused_commands=frozenset(FOCUSED_COMMANDS[:1]),security_requirement_ids=("SR-DEL-002","SR-DEL-003","SR-DEL-004","RV-24"),implemented_contracts=IMPLEMENTED,verification_evidence=VERIFICATION,blockers=BLOCKERS,summary=SUMMARY)
if __name__=="__main__":raise SystemExit(main(DEFINITION))
