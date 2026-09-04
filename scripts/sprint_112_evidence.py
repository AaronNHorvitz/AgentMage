#!/usr/bin/env python3
"""Build truthful Sprint 112 local deployment-safety evidence."""
from pathlib import Path
from typing import Final
try:from scripts.sprint_evidence_recorder import SprintEvidenceDefinition,main
except ModuleNotFoundError:from sprint_evidence_recorder import SprintEvidenceDefinition,main
ROOT:Final=Path(__file__).resolve().parents[1]
SOURCE_PATHS:Final=("Cargo.toml","Cargo.lock","kernel/engine/Cargo.toml","kernel/engine/src/lib.rs","kernel/engine/src/deployment_safety.rs","SECURITY-REVIEW.md","DELIVERY-SYSTEM.md","supply-chain/dependency-provenance.json","supply-chain/sbom.cdx.json","supply-chain/dependency-hashes.sha256","artifacts/sprints/sprint-112/deployment-safety-corpus.json","scripts/deployment_safety_contract.py","tests/test_deployment_safety_contract.py","scripts/sprint_evidence_recorder.py","scripts/sprint_112_evidence.py","tests/test_sprint_112_evidence.py")
COMMANDS:Final=(("deployment-safety-rust-contract",("cargo","test","-p","agentmage-kernel-engine","deployment_safety::tests","--lib")),("deployment-safety-corpus-contract",("python3","-m","unittest","tests.test_deployment_safety_contract")),("dependency-boundary-contract",("python3","scripts/dependency_rules.py")),("supply-chain-currentness",("python3","scripts/supply_chain.py")),("product-ci-contract",("python3","scripts/product_ci.py","--check")),("evidence-tests",("python3","-m","unittest","tests.test_sprint_112_evidence")))
FOCUSED_COMMANDS:Final=tuple(i[0] for i in COMMANDS[:3]);IMPLEMENTED:Final={"read_case_count":14,"renderer_fixture_count":2,"plan_case_count":64,"attack_case_count":2048,"fault_case_count":512,"cluster_contact_count":0,"unpreviewed_effect_count":0,"unsafe_retry_count":0,"promoted_environment_count":0}
BLOCKERS:Final=({"code":"UPSTREAM-SPRINT-111-BLOCKED","owner":"112"},{"code":"BLOCKED-EXTERNAL-DEPLOYMENT-AT-DEP-001-RV-28","owner":"112"})
VERIFICATION:Final={"focused_local_contracts":True,"focused_blocking_skip_count":0,"offline_rendering":True,"exact_resource_state":True,"fresh_rollback_approval":True,"rv25_deployment_complete":False,"rv28_complete":False,"at_dep_001_complete":False,"independent_human_review":False,"sprint_gate_closed":False}
SUMMARY:Final={"local_sprint_112_contract_passed":True,"sprint_status":"BLOCKED","upstream_sprint_111_closed":False,"live_kubernetes_support":False,"release_approval":False}
DEFINITION:Final=SprintEvidenceDefinition(sprint=112,root=ROOT,output="artifacts/sprints/sprint-112/local-evidence-report.json",source_paths=SOURCE_PATHS,commands=COMMANDS,focused_commands=FOCUSED_COMMANDS,rust_focused_commands=frozenset(FOCUSED_COMMANDS[:1]),security_requirement_ids=("AM-DEP-001","AT-DEP-001","SR-DEL-002","SR-DEL-005..014","RV-25","RV-28"),implemented_contracts=IMPLEMENTED,verification_evidence=VERIFICATION,blockers=BLOCKERS,summary=SUMMARY)
if __name__=="__main__":raise SystemExit(main(DEFINITION))
