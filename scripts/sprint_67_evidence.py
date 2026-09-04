#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 67 evidence."""

from __future__ import annotations

from pathlib import Path
from typing import Final

try:
    from scripts.sprint_evidence_recorder import SprintEvidenceDefinition, main
except ModuleNotFoundError:
    from sprint_evidence_recorder import SprintEvidenceDefinition, main


ROOT: Final = Path(__file__).resolve().parents[1]
SOURCE_PATHS: Final = (
    "Cargo.toml", "Cargo.lock", "capabilities/knowledge/Cargo.toml",
    "capabilities/knowledge/src/artifact_receipt.rs", "capabilities/knowledge/src/lib.rs",
    "schemas/runtime/common-artifact-receipt.schema.json",
    "schemas/runtime/examples/common-artifact-receipt.valid.json",
    "scripts/validate_planning_schemas.mjs", "tests/test_planning_schemas.mjs",
    "supply-chain/dependency-hashes.sha256", "supply-chain/dependency-provenance.json",
    "supply-chain/sbom.cdx.json",
    "docs/architecture/common-artifact-receipts-and-audio-observations.md",
    "docs/guides/common-artifact-receipt-review.md",
    "docs/verification/sprint-66-parser-corpus.json",
    "docs/verification/sprint-67-artifact-dependency-manifest.json",
    "docs/verification/sprint-67-fidelity-unsupported-matrix.json",
    "docs/verification/sprint-67-local-results.md",
    "scripts/common_artifact_receipt_contract.py",
    "tests/test_common_artifact_receipt_contract.py", "scripts/sprint_evidence_recorder.py",
    "scripts/sprint_67_evidence.py", "tests/test_sprint_67_evidence.py",
)
COMMANDS: Final = (
    ("common-receipt-unit", ("cargo", "test", "-p", "agentmage-capability-knowledge", "artifact_receipt::tests", "--lib", "--locked")),
    ("common-receipt-contract", ("python3", "-m", "unittest", "tests.test_common_artifact_receipt_contract")),
    ("runtime-schema-contract", ("npm", "run", "-s", "schemas:test")),
    ("common-receipt-strict-clippy", ("cargo", "clippy", "-p", "agentmage-capability-knowledge", "--all-targets", "--locked", "--", "-D", "warnings")),
    ("common-receipt-format", ("cargo", "fmt", "--all", "--", "--check")),
    ("supply-chain-currentness", ("python3", "scripts/supply_chain.py")),
    ("product-ci-contract", ("python3", "scripts/product_ci.py", "--check")),
    ("evidence-tests", ("python3", "-m", "unittest", "tests.test_sprint_67_evidence")),
)
FOCUSED_COMMANDS: Final = tuple(item[0] for item in COMMANDS[:3])
SECURITY_REQUIREMENTS: Final = (
    "SR-SUP-003", "SR-SUP-006", "SR-SUP-008", "SR-SUP-009",
    "SR-AI-007", "SR-TST-002", "SR-TST-004", "SR-TST-006",
)
IMPLEMENTED: Final = {
    "common_receipt_operation_class_count": 7, "receipt_rust_fixture_count": 4,
    "hostile_parser_corpus_case_count": 140, "common_receipt_schema_present": True,
    "fidelity_unsupported_matrix_present": True, "audio_observation_validation_present": True,
    "audio_engine_present": False, "audio_model_present": False,
    "transcription_execution_capability": False, "labeled_audio_metrics_present": False,
    "every_type_receipt_integration_complete": False, "network_access_capability": False,
    "filesystem_mutation_capability": False, "content_execution_capability": False,
    "native_platform_parity": False, "independent_review": False, "manual_fuzzing": False,
}
BLOCKERS: Final = (
    {"code": "UPSTREAM-SPRINT-66-BLOCKED", "owner": "67.1"},
    {"code": "ADMITTED-LOCAL-AUDIO-ENGINE-ABSENT", "owner": "67.1.1.1"},
    {"code": "LABELED-AUDIO-CAMPAIGN-ABSENT", "owner": "67.1.3.3"},
    {"code": "EVERY-TYPE-RECEIPT-INTEGRATION-INCOMPLETE", "owner": "67.1.3.4"},
    {"code": "NATIVE-PARSER-AND-AUDIO-PARITY-ABSENT", "owner": "67.1.3.5"},
    {"code": "INDEPENDENT-BOUNDARY-REVIEW-ABSENT", "owner": "67.1.3.5"},
    {"code": "MANUAL-FUZZING-DEFERRED", "owner": "SR-TST-004"},
)
VERIFICATION: Final = {
    "focused_local_contracts": True, "focused_blocking_skip_count": 0,
    "common_receipt_operation_class_count": 7, "hostile_parser_corpus_case_count": 140,
    "accepted_network_effect_count": 0, "accepted_execution_effect_count": 0,
    "accepted_filesystem_effect_count": 0, "canonical_reference_order_verified": True,
    "fidelity_disposition_verified": True, "audio_time_and_speaker_validation_verified": True,
    "audio_engine_run_count": 0, "labeled_audio_fixture_count": 0,
    "native_platform_count": 0, "independent_review": False, "manual_fuzzing": False,
    "sprint_gate_closed": False,
}
SUMMARY: Final = {
    "local_sprint_67_contract_passed": True, "sprint_status": "BLOCKED",
    "upstream_sprint_66_closed": False, "audio_transcription_complete": False,
    "labeled_accuracy_complete": False, "every_type_receipts_complete": False,
    "native_parity_complete": False, "independent_review_present": False,
    "manual_fuzzing_complete": False, "release_approval": False,
}
DEFINITION: Final = SprintEvidenceDefinition(
    sprint=67, root=ROOT, output="artifacts/sprints/sprint-67/local-evidence-report.json",
    source_paths=SOURCE_PATHS, commands=COMMANDS, focused_commands=FOCUSED_COMMANDS,
    rust_focused_commands=frozenset(FOCUSED_COMMANDS[:1]),
    security_requirement_ids=SECURITY_REQUIREMENTS, implemented_contracts=IMPLEMENTED,
    verification_evidence=VERIFICATION, blockers=BLOCKERS, summary=SUMMARY,
)


if __name__ == "__main__":
    raise SystemExit(main(DEFINITION))
