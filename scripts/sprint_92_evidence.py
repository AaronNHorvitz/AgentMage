#!/usr/bin/env python3
"""Build truthful Sprint 92 local agent-profile catalog evidence."""
from pathlib import Path
from typing import Final
try:from scripts.sprint_evidence_recorder import SprintEvidenceDefinition,main
except ModuleNotFoundError:from sprint_evidence_recorder import SprintEvidenceDefinition,main
ROOT:Final=Path(__file__).resolve().parents[1]
SOURCE_PATHS:Final=("Cargo.toml","Cargo.lock","kernel/engine/Cargo.toml","kernel/engine/src/lib.rs","kernel/engine/src/agent_profiles.rs","docs/architecture/planning-review-and-delivery-agent-profiles.md","artifacts/sprints/sprint-92/agent-profile-catalog.json","artifacts/sprints/sprint-92/agent-profile-reference.md","artifacts/sprints/sprint-92/agent-profile-compatibility.json","artifacts/sprints/sprint-92/agent-profile-fixtures.json","scripts/agent_profile_contract.py","tests/test_agent_profile_contract.py","supply-chain/dependency-hashes.sha256","supply-chain/dependency-provenance.json","supply-chain/sbom.cdx.json","scripts/sprint_evidence_recorder.py","scripts/sprint_92_evidence.py","tests/test_sprint_92_evidence.py")
COMMANDS:Final=(("agent-profile-rust-contract",("cargo","test","-p","agentmage-kernel-engine","agent_profiles::tests","--lib")),("agent-profile-artifact-contract",("python3","-m","unittest","tests.test_agent_profile_contract")),("runtime-schema-contract",("npm","run","-s","schemas:test")),("supply-chain-currentness",("python3","scripts/supply_chain.py")),("product-ci-contract",("python3","scripts/product_ci.py","--check")),("evidence-tests",("python3","-m","unittest","tests.test_sprint_92_evidence")))
FOCUSED_COMMANDS:Final=tuple(i[0] for i in COMMANDS[:2]);IMPLEMENTED:Final={"profile_count":49,"template_count":10,"fixture_count":343,"independent_review_profile_count":9,"runtime_implementation_count":1,"enabled_profile_count":0,"provider_connection_count":0}
BLOCKERS:Final=({"code":"UPSTREAM-SPRINT-91-BLOCKED","owner":"92"},)
VERIFICATION:Final={"focused_local_contracts":True,"focused_blocking_skip_count":0,"catalog_profile_count":49,"fixture_count":343,"registration_grants_authority":False,"independent_review_contract_count":9,"sprint_gate_closed":False}
SUMMARY:Final={"local_sprint_92_contract_passed":True,"sprint_status":"BLOCKED","upstream_sprint_91_closed":False,"catalog_complete":True,"all_profiles_disabled":True,"runtime_reused":True,"release_approval":False}
DEFINITION:Final=SprintEvidenceDefinition(sprint=92,root=ROOT,output="artifacts/sprints/sprint-92/local-evidence-report.json",source_paths=SOURCE_PATHS,commands=COMMANDS,focused_commands=FOCUSED_COMMANDS,rust_focused_commands=frozenset(FOCUSED_COMMANDS[:1]),security_requirement_ids=("SR-GOV-010","SR-ACC-001","SR-ACC-007","SR-ACC-008","SR-SUP-005","SR-SUP-006","SR-AI-003","SR-AI-004","SR-AI-005","SR-AI-006","SR-TST-004"),implemented_contracts=IMPLEMENTED,verification_evidence=VERIFICATION,blockers=BLOCKERS,summary=SUMMARY)
if __name__=="__main__":raise SystemExit(main(DEFINITION))
