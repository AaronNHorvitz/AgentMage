#!/usr/bin/env python3
"""Build truthful Sprint 110 local artifact-promotion evidence."""
from pathlib import Path
from typing import Final
try:from scripts.sprint_evidence_recorder import SprintEvidenceDefinition,main
except ModuleNotFoundError:from sprint_evidence_recorder import SprintEvidenceDefinition,main
ROOT:Final=Path(__file__).resolve().parents[1]
SOURCE_PATHS:Final=("Cargo.toml","Cargo.lock","kernel/engine/Cargo.toml","kernel/engine/src/lib.rs","kernel/engine/src/artifact_promotion.rs","DELIVERY-SYSTEM.md","artifacts/sprints/sprint-110/artifact-promotion-corpus.json","scripts/artifact_promotion_contract.py","tests/test_artifact_promotion_contract.py","supply-chain/dependency-hashes.sha256","supply-chain/dependency-provenance.json","supply-chain/sbom.cdx.json","scripts/sprint_evidence_recorder.py","scripts/sprint_110_evidence.py","tests/test_sprint_110_evidence.py")
COMMANDS:Final=(("artifact-promotion-rust-contract",("cargo","test","-p","agentmage-kernel-engine","artifact_promotion::tests","--lib")),("artifact-promotion-corpus-contract",("python3","-m","unittest","tests.test_artifact_promotion_contract")),("dependency-boundary-contract",("python3","scripts/dependency_rules.py")),("supply-chain-currentness",("python3","scripts/supply_chain.py")),("product-ci-contract",("python3","scripts/product_ci.py","--check")),("evidence-tests",("python3","-m","unittest","tests.test_sprint_110_evidence")))
FOCUSED_COMMANDS:Final=tuple(i[0] for i in COMMANDS[:3]);IMPLEMENTED:Final={"provider_fixture_count":5,"read_case_count":65,"transfer_case_count":5,"attack_case_count":2048,"mutable_label_authority_count":0,"corrupt_promotion_count":0,"provider_request_count":0,"promoted_provider_count":0}
BLOCKERS:Final=({"code":"UPSTREAM-SPRINT-109-BLOCKED","owner":"110"},{"code":"BLOCKED-EXTERNAL-ARTIFACT-REGISTRIES-AT-ART-001","owner":"110"})
VERIFICATION:Final={"focused_local_contracts":True,"focused_blocking_skip_count":0,"unsafe_retry_count":0,"residual_partial_count":0,"hidden_effect_count":0,"reviewer":"scripts.artifact_promotion_contract","at_art_001_complete":False,"independent_human_review":False,"sprint_gate_closed":False}
SUMMARY:Final={"local_sprint_110_contract_passed":True,"sprint_status":"BLOCKED","upstream_sprint_109_closed":False,"live_artifact_provider_support":False,"release_approval":False}
DEFINITION:Final=SprintEvidenceDefinition(sprint=110,root=ROOT,output="artifacts/sprints/sprint-110/local-evidence-report.json",source_paths=SOURCE_PATHS,commands=COMMANDS,focused_commands=FOCUSED_COMMANDS,rust_focused_commands=frozenset(FOCUSED_COMMANDS[:1]),security_requirement_ids=tuple([f"SR-DEL-{i:03d}" for i in range(1,15)]+[f"RV-{i}" for i in range(23,28)]),implemented_contracts=IMPLEMENTED,verification_evidence=VERIFICATION,blockers=BLOCKERS,summary=SUMMARY)
if __name__=="__main__":raise SystemExit(main(DEFINITION))
