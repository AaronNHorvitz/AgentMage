#!/usr/bin/env python3
"""Build truthful Sprint 105 local external-effect evidence."""
from pathlib import Path
from typing import Final
try:from scripts.sprint_evidence_recorder import SprintEvidenceDefinition,main
except ModuleNotFoundError:from sprint_evidence_recorder import SprintEvidenceDefinition,main
ROOT:Final=Path(__file__).resolve().parents[1]
SOURCE_PATHS:Final=("Cargo.toml","Cargo.lock","kernel/engine/Cargo.toml","kernel/engine/src/lib.rs","kernel/engine/src/external_effect.rs","DELIVERY-SYSTEM.md","artifacts/sprints/sprint-105/external-effect-corpus.json","scripts/external_effect_contract.py","tests/test_external_effect_contract.py","supply-chain/dependency-hashes.sha256","supply-chain/dependency-provenance.json","supply-chain/sbom.cdx.json","scripts/sprint_evidence_recorder.py","scripts/sprint_105_evidence.py","tests/test_sprint_105_evidence.py")
COMMANDS:Final=(("external-effect-rust-contract",("cargo","test","-p","agentmage-kernel-engine","external_effect::tests","--lib")),("external-effect-artifact-contract",("python3","-m","unittest","tests.test_external_effect_contract")),("dependency-boundary-contract",("python3","scripts/dependency_rules.py")),("supply-chain-currentness",("python3","scripts/supply_chain.py")),("product-ci-contract",("python3","scripts/product_ci.py","--check")),("evidence-tests",("python3","-m","unittest","tests.test_sprint_105_evidence")))
FOCUSED_COMMANDS:Final=tuple(i[0] for i in COMMANDS[:3]);IMPLEMENTED:Final={"plan_field_count":16,"effect_outcome_count":5,"schema_mutation_count":80,"fault_schedule_count":1024,"event_attack_count":12,"rollback_race_count":64,"duplicate_effect_count":0,"unsafe_retry_count":0,"live_provider_count":0}
BLOCKERS:Final=({"code":"UPSTREAM-SPRINT-104-BLOCKED","owner":"105"},{"code":"BLOCKED-EXTERNAL-RV25-RV26-NATIVE-EFFECT-EVENT-REVIEW","owner":"105.1.3.5"})
VERIFICATION:Final={"focused_local_contracts":True,"focused_blocking_skip_count":0,"event_authority_count":0,"stale_compensation_count":0,"reviewer":"scripts.external_effect_contract","rv25_complete":False,"rv26_complete":False,"sprint_gate_closed":False}
SUMMARY:Final={"local_sprint_105_contract_passed":True,"sprint_status":"BLOCKED","upstream_sprint_104_closed":False,"live_effect_support":False,"release_approval":False}
DEFINITION:Final=SprintEvidenceDefinition(sprint=105,root=ROOT,output="artifacts/sprints/sprint-105/local-evidence-report.json",source_paths=SOURCE_PATHS,commands=COMMANDS,focused_commands=FOCUSED_COMMANDS,rust_focused_commands=frozenset(FOCUSED_COMMANDS[:1]),security_requirement_ids=("SR-DEL-005","SR-DEL-006","SR-DEL-007","SR-DEL-008","SR-DEL-009","RV-25","RV-26"),implemented_contracts=IMPLEMENTED,verification_evidence=VERIFICATION,blockers=BLOCKERS,summary=SUMMARY)
if __name__=="__main__":raise SystemExit(main(DEFINITION))
