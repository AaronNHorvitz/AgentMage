#!/usr/bin/env python3
"""Build truthful Sprint 78 local capability package evidence."""
from pathlib import Path
from typing import Final
try: from scripts.sprint_evidence_recorder import SprintEvidenceDefinition,main
except ModuleNotFoundError: from sprint_evidence_recorder import SprintEvidenceDefinition,main
ROOT: Final=Path(__file__).resolve().parents[1]
SOURCE_PATHS: Final=("Cargo.toml","Cargo.lock","shells/host/Cargo.toml","shells/host/src/lib.rs","shells/host/src/capability_package.rs","docs/guides/capability-package-trust.md","docs/verification/sprint-78-capability-package-corpus.json","scripts/capability_package_contract.py","tests/test_capability_package_contract.py","supply-chain/dependency-hashes.sha256","supply-chain/dependency-provenance.json","supply-chain/sbom.cdx.json","scripts/sprint_evidence_recorder.py","scripts/sprint_78_evidence.py","tests/test_sprint_78_evidence.py")
COMMANDS: Final=(("capability-package-rust-contract",("cargo","test","-p","agentmage-host","capability_package::tests","--all-features")),("capability-package-artifact-contract",("python3","-m","unittest","tests.test_capability_package_contract")),("runtime-schema-contract",("npm","run","-s","schemas:test")),("supply-chain-currentness",("python3","scripts/supply_chain.py")),("product-ci-contract",("python3","scripts/product_ci.py","--check")),("evidence-tests",("python3","-m","unittest","tests.test_sprint_78_evidence")))
FOCUSED_COMMANDS: Final=tuple(item[0] for item in COMMANDS[:2])
IMPLEMENTED: Final={"manifest_field_group_count":16,"compatibility_version_count":7,"scope_dimension_count":9,"lifecycle_action_count":6,"corpus_case_count":44,"cryptographic_signature_verification":True,"installed_package_count":0,"enabled_package_count":0,"native_lifecycle_complete":False}
BLOCKERS: Final=({"code":"UPSTREAM-SPRINT-77-BLOCKED","owner":"78.1"},{"code":"NATIVE-PACKAGE-LIFECYCLE-AND-INDEPENDENT-REVIEW-ABSENT","owner":"78.1.3.4"})
VERIFICATION: Final={"focused_local_contracts":True,"focused_blocking_skip_count":0,"rust_test_count":5,"corpus_case_count":44,"installed_package_count":0,"enabled_package_count":0,"native_lifecycle_complete":False,"capability_inventory_diff_complete":False,"independent_review":False,"sprint_gate_closed":False}
SUMMARY: Final={"local_sprint_78_contract_passed":True,"sprint_status":"BLOCKED","upstream_sprint_77_closed":False,"native_package_lifecycle_complete":False,"native_security_evidence_complete":False,"independent_review_present":False,"release_approval":False}
DEFINITION: Final=SprintEvidenceDefinition(sprint=78,root=ROOT,output="artifacts/sprints/sprint-78/local-evidence-report.json",source_paths=SOURCE_PATHS,commands=COMMANDS,focused_commands=FOCUSED_COMMANDS,rust_focused_commands=frozenset(FOCUSED_COMMANDS[:1]),security_requirement_ids=("SR-GOV-010","SR-PLT-011","SR-ACC-001","SR-SUP-002","SR-SUP-003","SR-SUP-004","SR-SUP-005","SR-SUP-006","SR-SUP-007","SR-SUP-008","SR-SUP-009","SR-SUP-010","SR-SUP-011","SR-SUP-012","SR-SUP-013","SR-OPS-008","SR-OPS-010","SR-TST-011"),implemented_contracts=IMPLEMENTED,verification_evidence=VERIFICATION,blockers=BLOCKERS,summary=SUMMARY)
if __name__=="__main__":raise SystemExit(main(DEFINITION))
