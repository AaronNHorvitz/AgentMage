#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 66 evidence."""

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
    "capabilities/knowledge/src/additional_parsers.rs", "capabilities/knowledge/src/lib.rs",
    "supply-chain/dependency-hashes.sha256", "supply-chain/dependency-provenance.json",
    "supply-chain/sbom.cdx.json", "docs/architecture/safe-additional-file-parsers.md",
    "docs/guides/additional-parser-review.md", "docs/verification/sprint-66-parser-corpus.json",
    "docs/verification/sprint-66-parser-dependency-manifest.json",
    "docs/verification/sprint-66-local-results.md", "scripts/additional_parser_contract.py",
    "tests/test_additional_parser_contract.py", "scripts/sprint_evidence_recorder.py",
    "scripts/sprint_66_evidence.py", "tests/test_sprint_66_evidence.py",
)
COMMANDS: Final = (
    ("additional-parser-unit", ("cargo", "test", "-p", "agentmage-capability-knowledge", "additional_parsers::tests", "--lib", "--locked")),
    ("additional-parser-corpus", ("python3", "-m", "unittest", "tests.test_additional_parser_contract")),
    ("additional-parser-strict-clippy", ("cargo", "clippy", "-p", "agentmage-capability-knowledge", "--all-targets", "--locked", "--", "-D", "warnings")),
    ("additional-parser-format", ("cargo", "fmt", "--all", "--", "--check")),
    ("supply-chain-currentness", ("python3", "scripts/supply_chain.py")),
    ("product-ci-contract", ("python3", "scripts/product_ci.py", "--check")),
    ("evidence-tests", ("python3", "-m", "unittest", "tests.test_sprint_66_evidence")),
)
FOCUSED_COMMANDS: Final = tuple(item[0] for item in COMMANDS[:2])
SECURITY_REQUIREMENTS: Final = (
    "SR-SUP-003", "SR-SUP-006", "SR-SUP-008", "SR-SUP-009",
    "SR-AI-007", "SR-TST-002", "SR-TST-004", "SR-TST-006",
)
IMPLEMENTED: Final = {
    "parser_rust_fixture_count": 6, "review_corpus_case_count": 140,
    "saved_html_text_and_ranges": True, "bounded_xml_without_entities": True,
    "notebook_cell_output_and_execution_metadata": True, "notebook_execution_capability": False,
    "yaml_secret_redaction": True, "yaml_constructor_capability": False,
    "timestamp_json_lines_stack_log_sequence": True, "metadata_only_archive_inventory": True,
    "nested_archive_quarantine": True, "deferred_format_count": 6,
    "deferred_formats_enabled": False, "network_access_capability": False,
    "filesystem_mutation_capability": False, "content_execution_capability": False,
    "native_parser_parity": False, "transcription_metrics_present": False,
    "round_trip_authoring_complete": False, "independent_review": False,
    "manual_fuzzing": False,
}
BLOCKERS: Final = (
    {"code": "UPSTREAM-SPRINT-65-BLOCKED", "owner": "66.1"},
    {"code": "NATIVE-PARSER-PARITY-ABSENT", "owner": "66.1.3.4"},
    {"code": "AUDIO-TRANSCRIPTION-METRICS-OWNED-BY-SPRINT-67", "owner": "66.1.3.4"},
    {"code": "READ-ONLY-FORMAT-ROUND-TRIP-NOT-APPLICABLE", "owner": "66.1.3.4"},
    {"code": "INDEPENDENT-BOUNDARY-REVIEW-ABSENT", "owner": "66.1.3.4"},
    {"code": "MANUAL-FUZZING-DEFERRED", "owner": "SR-TST-004"},
)
VERIFICATION: Final = {
    "focused_local_contracts": True, "focused_blocking_skip_count": 0,
    "review_corpus_case_count": 140, "accepted_network_effect_count": 0,
    "accepted_execution_effect_count": 0, "accepted_filesystem_effect_count": 0,
    "source_ranges_verified": True, "notebook_inert_extraction_verified": True,
    "yaml_redaction_and_constructor_refusal_verified": True,
    "structured_log_sequence_verified": True, "archive_inventory_and_quarantine_verified": True,
    "deferred_format_refusal_verified": True, "native_parser_platform_count": 0,
    "transcription_metrics_present": False, "independent_review": False,
    "manual_fuzzing": False, "sprint_gate_closed": False,
}
SUMMARY: Final = {
    "local_sprint_66_contract_passed": True, "sprint_status": "BLOCKED",
    "upstream_sprint_65_closed": False, "native_parser_parity_complete": False,
    "transcription_metrics_complete": False, "round_trip_comparison_complete": False,
    "independent_review_present": False, "manual_fuzzing_complete": False,
    "release_approval": False,
}
DEFINITION: Final = SprintEvidenceDefinition(
    sprint=66, root=ROOT, output="artifacts/sprints/sprint-66/local-evidence-report.json",
    source_paths=SOURCE_PATHS, commands=COMMANDS, focused_commands=FOCUSED_COMMANDS,
    rust_focused_commands=frozenset(FOCUSED_COMMANDS[:1]),
    security_requirement_ids=SECURITY_REQUIREMENTS, implemented_contracts=IMPLEMENTED,
    verification_evidence=VERIFICATION, blockers=BLOCKERS, summary=SUMMARY,
)


if __name__ == "__main__":
    raise SystemExit(main(DEFINITION))
