#!/usr/bin/env python3
"""Build truthful Sprint 111 local supply-chain/finding evidence."""
from pathlib import Path
from typing import Final
try:from scripts.sprint_evidence_recorder import SprintEvidenceDefinition,main
except ModuleNotFoundError:from sprint_evidence_recorder import SprintEvidenceDefinition,main
ROOT:Final=Path(__file__).resolve().parents[1]
SOURCE_PATHS:Final=("Cargo.toml","Cargo.lock","kernel/engine/Cargo.toml","kernel/engine/src/lib.rs","kernel/engine/src/supply_chain_evidence.rs","SECURITY-REVIEW.md","supply-chain/dependency-provenance.json","supply-chain/sbom.cdx.json","supply-chain/dependency-hashes.sha256","artifacts/sprints/sprint-111/supply-chain-finding-corpus.json","scripts/supply_chain_finding_contract.py","tests/test_supply_chain_finding_contract.py","scripts/sprint_evidence_recorder.py","scripts/sprint_111_evidence.py","tests/test_sprint_111_evidence.py")
COMMANDS:Final=(("supply-chain-evidence-rust-contract",("cargo","test","-p","agentmage-kernel-engine","supply_chain_evidence::tests","--lib")),("supply-chain-finding-corpus-contract",("python3","-m","unittest","tests.test_supply_chain_finding_contract")),("dependency-boundary-contract",("python3","scripts/dependency_rules.py")),("supply-chain-currentness",("python3","scripts/supply_chain.py")),("product-ci-contract",("python3","scripts/product_ci.py","--check")),("evidence-tests",("python3","-m","unittest","tests.test_sprint_111_evidence")))
FOCUSED_COMMANDS:Final=tuple(i[0] for i in COMMANDS[:3]);IMPLEMENTED:Final={"document_kind_count":6,"finding_tool_fixture_count":7,"attack_case_count":2048,"hidden_blocking_finding_count":0,"accepted_stale_evidence_count":0,"assurance_overclaim_count":0,"promoted_tool_count":0}
BLOCKERS:Final=({"code":"UPSTREAM-SPRINT-110-BLOCKED","owner":"111"},{"code":"BLOCKED-EXTERNAL-SUPPLY-SECURITY-AT-SUP-001-AT-SEC-003","owner":"111"})
VERIFICATION:Final={"focused_local_contracts":True,"focused_blocking_skip_count":0,"conflicts_preserved":True,"release_state_change_count":0,"reviewer":"scripts.supply_chain_finding_contract","rv19_complete":False,"at_sup_001_complete":False,"at_sec_003_complete":False,"independent_human_review":False,"sprint_gate_closed":False}
SUMMARY:Final={"local_sprint_111_contract_passed":True,"sprint_status":"BLOCKED","upstream_sprint_110_closed":False,"live_security_tool_support":False,"release_approval":False}
DEFINITION:Final=SprintEvidenceDefinition(sprint=111,root=ROOT,output="artifacts/sprints/sprint-111/local-evidence-report.json",source_paths=SOURCE_PATHS,commands=COMMANDS,focused_commands=FOCUSED_COMMANDS,rust_focused_commands=frozenset(FOCUSED_COMMANDS[:1]),security_requirement_ids=("SR-SUP-*","SR-DEL-004","SR-DEL-011","SR-DEL-013","RV-19","RV-23","RV-27"),implemented_contracts=IMPLEMENTED,verification_evidence=VERIFICATION,blockers=BLOCKERS,summary=SUMMARY)
if __name__=="__main__":raise SystemExit(main(DEFINITION))
