#!/usr/bin/env python3
"""Build truthful Sprint 82 local public-research evidence."""

from pathlib import Path
from typing import Final

try:
    from scripts.sprint_evidence_recorder import SprintEvidenceDefinition, main
except ModuleNotFoundError:
    from sprint_evidence_recorder import SprintEvidenceDefinition, main


ROOT: Final = Path(__file__).resolve().parents[1]
SOURCE_PATHS: Final = (
    "Cargo.toml", "Cargo.lock", "shells/host/Cargo.toml", "shells/host/src/lib.rs",
    "shells/host/src/public_research.rs", "docs/guides/public-research.md",
    "docs/verification/sprint-82-public-research-corpus.json",
    "scripts/public_research_contract.py", "tests/test_public_research_contract.py",
    "supply-chain/dependency-hashes.sha256", "supply-chain/dependency-provenance.json",
    "supply-chain/sbom.cdx.json", "scripts/sprint_evidence_recorder.py",
    "scripts/sprint_82_evidence.py", "tests/test_sprint_82_evidence.py",
)
COMMANDS: Final = (
    ("public-research-rust-contract", ("cargo", "test", "-p", "agentmage-host", "public_research::tests", "--lib")),
    ("public-research-artifact-contract", ("python3", "-m", "unittest", "tests.test_public_research_contract")),
    ("runtime-schema-contract", ("npm", "run", "-s", "schemas:test")),
    ("supply-chain-currentness", ("python3", "scripts/supply_chain.py")),
    ("product-ci-contract", ("python3", "scripts/product_ci.py", "--check")),
    ("evidence-tests", ("python3", "-m", "unittest", "tests.test_sprint_82_evidence")),
)
FOCUSED_COMMANDS: Final = tuple(item[0] for item in COMMANDS[:2])
IMPLEMENTED: Final = {
    "source_type_count": 4, "focused_rust_test_count": 4, "corpus_case_count": 30,
    "quotation_word_limit": 25, "network_executor_count": 0,
    "live_search_count": 0, "browser_action_count": 0,
}
BLOCKERS: Final = (
    {"code": "UPSTREAM-SPRINT-81-BLOCKED", "owner": "82"},
    {"code": "LIVE-SEARCH-BROWSER-SECURITY-AND-PRIVACY-REVIEW-ABSENT", "owner": "82.1.3.4"},
)
VERIFICATION: Final = {
    "focused_local_contracts": True, "focused_blocking_skip_count": 0,
    "rust_test_count": 4, "corpus_case_count": 30, "citation_content_minimized": True,
    "live_public_search_executed": False, "browser_campaign_complete": False,
    "privacy_review": False, "sprint_gate_closed": False,
}
SUMMARY: Final = {
    "local_sprint_82_contract_passed": True, "sprint_status": "BLOCKED",
    "upstream_sprint_81_closed": False, "live_research_complete": False,
    "native_security_evidence_complete": False, "privacy_review_present": False,
    "release_approval": False,
}
DEFINITION: Final = SprintEvidenceDefinition(
    sprint=82, root=ROOT, output="artifacts/sprints/sprint-82/local-evidence-report.json",
    source_paths=SOURCE_PATHS, commands=COMMANDS, focused_commands=FOCUSED_COMMANDS,
    rust_focused_commands=frozenset(FOCUSED_COMMANDS[:1]),
    security_requirement_ids=(
        "SR-ACC-002", "SR-ACC-007", "SR-ACC-008", "SR-DAT-002", "SR-DAT-003",
        "SR-NET-003", "SR-NET-004", "SR-NET-005", "SR-NET-006", "SR-AI-004", "SR-AI-005",
    ),
    implemented_contracts=IMPLEMENTED, verification_evidence=VERIFICATION,
    blockers=BLOCKERS, summary=SUMMARY,
)


if __name__ == "__main__":
    raise SystemExit(main(DEFINITION))
