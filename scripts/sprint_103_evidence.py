#!/usr/bin/env python3
"""Build truthful Sprint 103 local delivery-foundation evidence."""
from pathlib import Path
from typing import Final
try:from scripts.sprint_evidence_recorder import SprintEvidenceDefinition,main
except ModuleNotFoundError:from sprint_evidence_recorder import SprintEvidenceDefinition,main
ROOT:Final=Path(__file__).resolve().parents[1]
SOURCE_PATHS:Final=("Cargo.toml","Cargo.lock","kernel/engine/Cargo.toml","kernel/engine/src/lib.rs","kernel/engine/src/delivery_graph.rs","DELIVERY-SYSTEM.md","artifacts/sprints/sprint-103/delivery-foundation-corpus.json","scripts/delivery_foundation_contract.py","tests/test_delivery_foundation_contract.py","supply-chain/dependency-hashes.sha256","supply-chain/dependency-provenance.json","supply-chain/sbom.cdx.json","scripts/sprint_evidence_recorder.py","scripts/sprint_103_evidence.py","tests/test_sprint_103_evidence.py")
COMMANDS:Final=(("delivery-foundation-rust-contract",("cargo","test","-p","agentmage-kernel-engine","delivery_graph::tests","--lib")),("delivery-foundation-artifact-contract",("python3","-m","unittest","tests.test_delivery_foundation_contract")),("dependency-boundary-contract",("python3","scripts/dependency_rules.py")),("supply-chain-currentness",("python3","scripts/supply_chain.py")),("product-ci-contract",("python3","scripts/product_ci.py","--check")),("evidence-tests",("python3","-m","unittest","tests.test_sprint_103_evidence")))
FOCUSED_COMMANDS:Final=tuple(i[0] for i in COMMANDS[:3]);IMPLEMENTED:Final={"node_kind_count":17,"relationship_state_count":8,"graph_case_count":170,"adapter_operation_count":9,"conformance_level_count":6,"matrix_case_count":54,"fake_adapter_mode_count":6,"enabled_adapter_count":0}
BLOCKERS:Final=({"code":"UPSTREAM-SPRINT-102-BLOCKED","owner":"103"},)
VERIFICATION:Final={"focused_local_contracts":True,"focused_blocking_skip_count":0,"graph_case_count":170,"matrix_case_count":54,"provider_logic_in_kernel_count":0,"live_provider_count":0,"reviewer":"scripts.delivery_foundation_contract","sprint_gate_closed":False}
SUMMARY:Final={"local_sprint_103_contract_passed":True,"sprint_status":"BLOCKED","upstream_sprint_102_closed":False,"live_provider_support":False,"release_approval":False}
DEFINITION:Final=SprintEvidenceDefinition(sprint=103,root=ROOT,output="artifacts/sprints/sprint-103/local-evidence-report.json",source_paths=SOURCE_PATHS,commands=COMMANDS,focused_commands=FOCUSED_COMMANDS,rust_focused_commands=frozenset(FOCUSED_COMMANDS[:1]),security_requirement_ids=("SR-DEL-001","SR-DEL-002","RV-23","RV-27"),implemented_contracts=IMPLEMENTED,verification_evidence=VERIFICATION,blockers=BLOCKERS,summary=SUMMARY)
if __name__=="__main__":raise SystemExit(main(DEFINITION))
