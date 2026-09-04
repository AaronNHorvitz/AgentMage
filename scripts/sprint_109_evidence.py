#!/usr/bin/env python3
"""Build truthful Sprint 109 local CI evidence."""
from pathlib import Path
from typing import Final
try:from scripts.sprint_evidence_recorder import SprintEvidenceDefinition,main
except ModuleNotFoundError:from sprint_evidence_recorder import SprintEvidenceDefinition,main
ROOT:Final=Path(__file__).resolve().parents[1]
SOURCE_PATHS:Final=("Cargo.toml","Cargo.lock","kernel/engine/Cargo.toml","kernel/engine/src/lib.rs","kernel/engine/src/ci_control.rs","DELIVERY-SYSTEM.md","artifacts/sprints/sprint-109/ci-control-corpus.json","scripts/ci_control_contract.py","tests/test_ci_control_contract.py","supply-chain/dependency-hashes.sha256","supply-chain/dependency-provenance.json","supply-chain/sbom.cdx.json","scripts/sprint_evidence_recorder.py","scripts/sprint_109_evidence.py","tests/test_sprint_109_evidence.py")
COMMANDS:Final=(("ci-control-rust-contract",("cargo","test","-p","agentmage-kernel-engine","ci_control::tests","--lib")),("ci-control-artifact-contract",("python3","-m","unittest","tests.test_ci_control_contract")),("dependency-boundary-contract",("python3","scripts/dependency_rules.py")),("supply-chain-currentness",("python3","scripts/supply_chain.py")),("product-ci-contract",("python3","scripts/product_ci.py","--check")),("evidence-tests",("python3","-m","unittest","tests.test_sprint_109_evidence")))
FOCUSED_COMMANDS:Final=tuple(i[0] for i in COMMANDS[:3]);IMPLEMENTED:Final={"provider_fixture_count":4,"observation_case_count":44,"effect_case_count":16,"fault_schedule_count":1024,"attack_case_count":2048,"duplicate_run_count":0,"secret_disclosure_count":0,"provider_request_count":0,"promoted_provider_count":0}
BLOCKERS:Final=({"code":"UPSTREAM-SPRINT-108-BLOCKED","owner":"109"},{"code":"BLOCKED-EXTERNAL-CI-PROVIDERS-AT-CIC-001","owner":"109"})
VERIFICATION:Final={"focused_local_contracts":True,"focused_blocking_skip_count":0,"duplicate_run_count":0,"stronger_capability_count":0,"unsafe_output_count":0,"reviewer":"scripts.ci_control_contract","at_cic_001_complete":False,"independent_human_review":False,"sprint_gate_closed":False}
SUMMARY:Final={"local_sprint_109_contract_passed":True,"sprint_status":"BLOCKED","upstream_sprint_108_closed":False,"live_ci_provider_support":False,"release_approval":False}
DEFINITION:Final=SprintEvidenceDefinition(sprint=109,root=ROOT,output="artifacts/sprints/sprint-109/local-evidence-report.json",source_paths=SOURCE_PATHS,commands=COMMANDS,focused_commands=FOCUSED_COMMANDS,rust_focused_commands=frozenset(FOCUSED_COMMANDS[:1]),security_requirement_ids=tuple([f"SR-DEL-{i:03d}" for i in range(5,15)]+[f"RV-{i}" for i in range(23,27)]+["RV-29"]),implemented_contracts=IMPLEMENTED,verification_evidence=VERIFICATION,blockers=BLOCKERS,summary=SUMMARY)
if __name__=="__main__":raise SystemExit(main(DEFINITION))
