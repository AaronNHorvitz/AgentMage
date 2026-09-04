#!/usr/bin/env python3
"""Build truthful Sprint 107 local work-management evidence."""
from pathlib import Path
from typing import Final
try:from scripts.sprint_evidence_recorder import SprintEvidenceDefinition,main
except ModuleNotFoundError:from sprint_evidence_recorder import SprintEvidenceDefinition,main
ROOT:Final=Path(__file__).resolve().parents[1]
SOURCE_PATHS:Final=("Cargo.toml","Cargo.lock","capabilities/knowledge/Cargo.toml","capabilities/knowledge/src/lib.rs","capabilities/knowledge/src/work_management.rs","DELIVERY-SYSTEM.md","docs/decisions/0041-standardized-planning-review-and-delivery-agent-profiles.md","artifacts/sprints/sprint-107/work-management-corpus.json","scripts/work_management_contract.py","tests/test_work_management_contract.py","supply-chain/dependency-hashes.sha256","supply-chain/dependency-provenance.json","supply-chain/sbom.cdx.json","scripts/sprint_evidence_recorder.py","scripts/sprint_107_evidence.py","tests/test_sprint_107_evidence.py")
COMMANDS:Final=(("work-management-rust-contract",("cargo","test","-p","agentmage-capability-knowledge","work_management::tests","--lib")),("work-management-artifact-contract",("python3","-m","unittest","tests.test_work_management_contract")),("dependency-boundary-contract",("python3","scripts/dependency_rules.py")),("supply-chain-currentness",("python3","scripts/supply_chain.py")),("product-ci-contract",("python3","scripts/product_ci.py","--check")),("evidence-tests",("python3","-m","unittest","tests.test_sprint_107_evidence")))
FOCUSED_COMMANDS:Final=tuple(i[0] for i in COMMANDS[:3])
IMPLEMENTED:Final={"provider_fixture_count":4,"object_case_count":80,"link_case_count":48,"effect_case_count":40,"workflow_case_count":56,"attack_case_count":1024,"recovery_case_count":512,"review_surface_count":10,"provider_request_count":0,"promoted_provider_count":0,"live_provider_count":0}
BLOCKERS:Final=({"code":"UPSTREAM-SPRINT-106-BLOCKED","owner":"107"},{"code":"BLOCKED-EXTERNAL-WORK-PROVIDERS-AT-WRK-001","owner":"107"})
VERIFICATION:Final={"focused_local_contracts":True,"focused_blocking_skip_count":0,"link_authority_broadening_count":0,"non_target_change_count":0,"hidden_recipient_count":0,"duplicate_effect_count":0,"reviewer":"scripts.work_management_contract","at_wrk_001_complete":False,"independent_human_review":False,"sprint_gate_closed":False}
SUMMARY:Final={"local_sprint_107_contract_passed":True,"sprint_status":"BLOCKED","upstream_sprint_106_closed":False,"live_work_provider_support":False,"release_approval":False}
DEFINITION:Final=SprintEvidenceDefinition(sprint=107,root=ROOT,output="artifacts/sprints/sprint-107/local-evidence-report.json",source_paths=SOURCE_PATHS,commands=COMMANDS,focused_commands=FOCUSED_COMMANDS,rust_focused_commands=frozenset(FOCUSED_COMMANDS[:1]),security_requirement_ids=tuple([f"SR-DEL-{i:03d}" for i in range(1,14)]+[f"RV-{i}" for i in range(23,28)]),implemented_contracts=IMPLEMENTED,verification_evidence=VERIFICATION,blockers=BLOCKERS,summary=SUMMARY)
if __name__=="__main__":raise SystemExit(main(DEFINITION))
