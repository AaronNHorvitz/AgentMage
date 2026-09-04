#!/usr/bin/env python3
"""Build truthful Sprint 113 local GitOps evidence."""
from pathlib import Path
from typing import Final
try:from scripts.sprint_evidence_recorder import SprintEvidenceDefinition,main
except ModuleNotFoundError:from sprint_evidence_recorder import SprintEvidenceDefinition,main
ROOT:Final=Path(__file__).resolve().parents[1]
SOURCE_PATHS:Final=("Cargo.toml","Cargo.lock","kernel/engine/Cargo.toml","kernel/engine/src/lib.rs","kernel/engine/src/gitops_control.rs","SECURITY-REVIEW.md","DELIVERY-SYSTEM.md","supply-chain/dependency-provenance.json","supply-chain/sbom.cdx.json","supply-chain/dependency-hashes.sha256","artifacts/sprints/sprint-113/gitops-control-corpus.json","scripts/gitops_control_contract.py","tests/test_gitops_control_contract.py","scripts/sprint_evidence_recorder.py","scripts/sprint_113_evidence.py","tests/test_sprint_113_evidence.py")
COMMANDS:Final=(("gitops-rust-contract",("cargo","test","-p","agentmage-kernel-engine","gitops_control::tests","--lib")),("gitops-corpus-contract",("python3","-m","unittest","tests.test_gitops_control_contract")),("dependency-boundary-contract",("python3","scripts/dependency_rules.py")),("supply-chain-currentness",("python3","scripts/supply_chain.py")),("product-ci-contract",("python3","scripts/product_ci.py","--check")),("evidence-tests",("python3","-m","unittest","tests.test_sprint_113_evidence")))
FOCUSED_COMMANDS:Final=tuple(i[0] for i in COMMANDS[:3]);IMPLEMENTED:Final={"provider_fixture_count":2,"read_case_count":22,"plan_case_count":64,"attack_case_count":2048,"fault_case_count":512,"controller_contact_count":0,"authority_escape_count":0,"unsafe_retry_count":0,"promoted_provider_count":0}
BLOCKERS:Final=({"code":"UPSTREAM-SPRINT-112-BLOCKED","owner":"113"},{"code":"BLOCKED-EXTERNAL-GITOPS-AT-DEP-001-RV-28","owner":"113"})
VERIFICATION:Final={"focused_local_contracts":True,"focused_blocking_skip_count":0,"desired_sync_authority_separate":True,"strong_effects_default_disabled":True,"controller_race_reconciliation":True,"supported_version_matrix_complete":False,"at_dep_001_complete":False,"independent_human_review":False,"sprint_gate_closed":False}
SUMMARY:Final={"local_sprint_113_contract_passed":True,"sprint_status":"BLOCKED","upstream_sprint_112_closed":False,"live_gitops_support":False,"release_approval":False}
DEFINITION:Final=SprintEvidenceDefinition(sprint=113,root=ROOT,output="artifacts/sprints/sprint-113/local-evidence-report.json",source_paths=SOURCE_PATHS,commands=COMMANDS,focused_commands=FOCUSED_COMMANDS,rust_focused_commands=frozenset(FOCUSED_COMMANDS[:1]),security_requirement_ids=("AM-DEP-001","AT-DEP-001","SR-DEL-*","RV-25","RV-27","RV-28"),implemented_contracts=IMPLEMENTED,verification_evidence=VERIFICATION,blockers=BLOCKERS,summary=SUMMARY)
if __name__=="__main__":raise SystemExit(main(DEFINITION))
