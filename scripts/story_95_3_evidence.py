#!/usr/bin/env python3
"""Build truthful Story 95.3 local capability lifecycle evidence."""
from pathlib import Path
from typing import Final
try:from scripts.sprint_evidence_recorder import SprintEvidenceDefinition,main
except ModuleNotFoundError:from sprint_evidence_recorder import SprintEvidenceDefinition,main
ROOT:Final=Path(__file__).resolve().parents[1]
SOURCE_PATHS:Final=("Cargo.toml","Cargo.lock","kernel/engine/Cargo.toml","kernel/engine/src/lib.rs","kernel/engine/src/capability_registry.rs","kernel/engine/src/capability_lifecycle.rs","artifacts/sprints/sprint-95/local-coordination-evidence-report.json","artifacts/sprints/sprint-95/capability-lifecycle-corpus.json","scripts/capability_lifecycle_contract.py","tests/test_capability_lifecycle_contract.py","supply-chain/dependency-hashes.sha256","supply-chain/dependency-provenance.json","supply-chain/sbom.cdx.json","scripts/sprint_evidence_recorder.py","scripts/story_95_3_evidence.py","tests/test_story_95_3_evidence.py")
COMMANDS:Final=(("capability-lifecycle-rust-contract",("cargo","test","-p","agentmage-kernel-engine","capability_lifecycle::tests","--lib")),("capability-lifecycle-artifact-contract",("python3","-m","unittest","tests.test_capability_lifecycle_contract")),("supply-chain-currentness",("python3","scripts/supply_chain.py")),("product-ci-contract",("python3","scripts/product_ci.py","--check")),("evidence-tests",("python3","-m","unittest","tests.test_story_95_3_evidence")))
FOCUSED_COMMANDS:Final=tuple(i[0] for i in COMMANDS[:2]);IMPLEMENTED:Final={"candidate_count":9,"lifecycle_state_count":7,"lifecycle_case_count":66,"shared_runtime_count":1,"enabled_capability_count":0,"native_strict_local_restoration_count":0}
BLOCKERS:Final=({"code":"UPSTREAM-STORY-95.2-BLOCKED","owner":"95.3"},{"code":"STRICT-LOCAL-NATIVE-RESTORATION-ABSENT","owner":"95.3-native"})
VERIFICATION:Final={"focused_local_contracts":True,"focused_blocking_skip_count":0,"lifecycle_case_count":66,"enabled_capability_count":0,"residual_authority_count":0,"native_strict_local_restoration_observed":False,"story_gate_closed":False}
SUMMARY:Final={"local_story_95_3_contract_passed":True,"sprint_status":"BLOCKED","upstream_story_95_2_closed":False,"native_strict_local_complete":False,"release_approval":False}
DEFINITION:Final=SprintEvidenceDefinition(sprint=95,root=ROOT,output="artifacts/sprints/sprint-95/story-95.3-local-evidence-report.json",source_paths=SOURCE_PATHS,commands=COMMANDS,focused_commands=FOCUSED_COMMANDS,rust_focused_commands=frozenset(FOCUSED_COMMANDS[:1]),security_requirement_ids=("SR-GOV-010","SR-ACC-001","SR-ACC-007","SR-ACC-008","SR-AI-003","SR-AI-004","SR-AI-005","SR-TST-004"),implemented_contracts=IMPLEMENTED,verification_evidence=VERIFICATION,blockers=BLOCKERS,summary=SUMMARY)
if __name__=="__main__":raise SystemExit(main(DEFINITION))
