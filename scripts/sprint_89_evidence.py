#!/usr/bin/env python3
"""Build truthful Sprint 89 local job-runtime evidence."""
from pathlib import Path
from typing import Final
try:from scripts.sprint_evidence_recorder import SprintEvidenceDefinition,main
except ModuleNotFoundError:from sprint_evidence_recorder import SprintEvidenceDefinition,main
ROOT:Final=Path(__file__).resolve().parents[1]
SOURCE_PATHS:Final=("Cargo.toml","Cargo.lock","kernel/engine/Cargo.toml","kernel/engine/src/lib.rs","kernel/engine/src/job_scheduler.rs","docs/guides/read-only-jobs-and-schedules.md","docs/verification/sprint-89-job-runtime-corpus.json","artifacts/sprints/sprint-88/local-evidence-report.json","scripts/job_scheduler_contract.py","tests/test_job_scheduler_contract.py","supply-chain/dependency-hashes.sha256","supply-chain/dependency-provenance.json","supply-chain/sbom.cdx.json","scripts/sprint_evidence_recorder.py","scripts/sprint_89_evidence.py","tests/test_sprint_89_evidence.py")
COMMANDS:Final=(("job-scheduler-rust-contract",("cargo","test","-p","agentmage-kernel-engine","job_scheduler::tests","--lib")),("job-scheduler-artifact-contract",("python3","-m","unittest","tests.test_job_scheduler_contract")),("runtime-schema-contract",("npm","run","-s","schemas:test")),("supply-chain-currentness",("python3","scripts/supply_chain.py")),("product-ci-contract",("python3","scripts/product_ci.py","--check")),("evidence-tests",("python3","-m","unittest","tests.test_sprint_89_evidence")))
FOCUSED_COMMANDS:Final=tuple(i[0] for i in COMMANDS[:2]);IMPLEMENTED:Final={"job_state_count":7,"schedule_control_count":8,"rust_test_count":6,"corpus_case_count":56,"effect_executor_count":0,"native_run_count":0}
BLOCKERS:Final=({"code":"UPSTREAM-SPRINT-88-BLOCKED","owner":"89"},{"code":"NATIVE-MULTI-RUNNER-JOB-RECOVERY-AND-REVIEW-ABSENT","owner":"89.1.3.4"})
VERIFICATION:Final={"focused_local_contracts":True,"focused_blocking_skip_count":0,"rust_test_count":6,"corpus_case_count":56,"native_multi_runner_executed":False,"durable_restart_executed":False,"post_run_receipt_captured":False,"independent_review":False,"sprint_gate_closed":False}
SUMMARY:Final={"local_sprint_89_contract_passed":True,"sprint_status":"BLOCKED","upstream_sprint_88_closed":False,"native_job_runtime_complete":False,"multi_runner_recovery_complete":False,"independent_review_present":False,"release_approval":False}
DEFINITION:Final=SprintEvidenceDefinition(sprint=89,root=ROOT,output="artifacts/sprints/sprint-89/local-evidence-report.json",source_paths=SOURCE_PATHS,commands=COMMANDS,focused_commands=FOCUSED_COMMANDS,rust_focused_commands=frozenset(FOCUSED_COMMANDS[:1]),security_requirement_ids=("SR-ACC-001","SR-ACC-007","SR-DAT-010","SR-AI-004","SR-AI-009","SR-OPS-001","SR-OPS-002","SR-TST-005","SR-TST-006"),implemented_contracts=IMPLEMENTED,verification_evidence=VERIFICATION,blockers=BLOCKERS,summary=SUMMARY)
if __name__=="__main__":raise SystemExit(main(DEFINITION))
