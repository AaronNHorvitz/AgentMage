#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 69 aggregate evidence."""

from __future__ import annotations
from pathlib import Path
from typing import Final
try:
    from scripts.sprint_evidence_recorder import SprintEvidenceDefinition, main
except ModuleNotFoundError:
    from sprint_evidence_recorder import SprintEvidenceDefinition, main

ROOT: Final = Path(__file__).resolve().parents[1]
SOURCE_PATHS: Final = (
    "docs/verification/v0.6-cross-format-local-matrix.json",
    "docs/verification/v0.6-administrative-local-bundle.json",
    "docs/guides/v0.6-administrative-artifact-review.md",
    "docs/releases/v0.6-local-candidate-notes.md", "scripts/v0_6_admin_release_contract.py",
    "tests/test_v0_6_admin_release_contract.py", "scripts/sprint_evidence_recorder.py",
    "scripts/sprint_69_evidence.py", "tests/test_sprint_69_evidence.py",
    *(f"artifacts/sprints/sprint-{number}/local-evidence-report.json" for number in range(54, 69)),
)
COMMANDS: Final = (
    ("v0.6-admin-release-contract", ("python3", "-m", "unittest", "tests.test_v0_6_admin_release_contract")),
    ("product-ci-contract", ("python3", "scripts/product_ci.py", "--check")),
    ("supply-chain-currentness", ("python3", "scripts/supply_chain.py")),
    ("evidence-tests", ("python3", "-m", "unittest", "tests.test_sprint_69_evidence")),
)
FOCUSED_COMMANDS: Final = (COMMANDS[0][0],)
SECURITY_REQUIREMENTS: Final = ("SR-DAT-001", "SR-DAT-002", "SR-DAT-003", "SR-SUP-008", "SR-SUP-009", "SR-AI-007", "SR-OPS-001", "SR-OPS-002", "SR-TST-002", "SR-TST-004", "SR-CIV-006", "SR-CIV-007")
IMPLEMENTED: Final = {
    "source_sprint_count": 15, "administrative_workflow_class_count": 8,
    "format_class_count": 10, "review_guide_count": 14,
    "all_source_reports_blocked": True, "sending_enabled": False,
    "live_calendar_changes_enabled": False, "messaging_enabled": False,
    "external_database_access_enabled": False, "automatic_recipient_selection_enabled": False,
    "unattended_disposition_enabled": False, "supported_format_count": 0,
    "supported_platform_count": 0, "native_format_matrix_complete": False,
    "accessibility_matrix_complete": False, "independent_review": False, "manual_fuzzing": False,
}
BLOCKERS: Final = (
    {"code": "UPSTREAM-SPRINTS-54-68-BLOCKED", "owner": "69.1"},
    {"code": "NATIVE-CROSS-FORMAT-MATRIX-INCOMPLETE", "owner": "69.1.1.2"},
    {"code": "HOSTILE-CROSS-FORMAT-MATRIX-INCOMPLETE", "owner": "69.1.1.3"},
    {"code": "EVERY-OUTPUT-CONTROLLED-WRITE-MATRIX-INCOMPLETE", "owner": "69.1.1.4"},
    {"code": "RECOVERY-MATRIX-INCOMPLETE", "owner": "69.1.3.3"},
    {"code": "INDEPENDENT-ARTIFACT-REVIEW-ABSENT", "owner": "69.1.3.4"},
    {"code": "SIGNED-GATE-DECISION-ABSENT", "owner": "69.1.3.5"},
)
VERIFICATION: Final = {
    "focused_local_contracts": True, "focused_blocking_skip_count": 0,
    "source_sprint_count": 15, "source_report_command_failures": 0,
    "source_release_approval_count": 0, "supported_format_count": 0,
    "supported_platform_count": 0, "disabled_external_effect_count": 6,
    "native_format_matrix_complete": False, "accessibility_matrix_complete": False,
    "independent_review": False, "signed_gate_decision": False, "sprint_gate_closed": False,
}
SUMMARY: Final = {
    "local_sprint_69_aggregate_passed": True, "sprint_status": "BLOCKED",
    "upstream_sprints_closed": False, "native_cross_format_complete": False,
    "accessibility_complete": False, "recovery_complete": False,
    "independent_review_present": False, "signed_gate_decision_present": False,
    "release_approval": False,
}
DEFINITION: Final = SprintEvidenceDefinition(
    sprint=69, root=ROOT, output="artifacts/sprints/sprint-69/local-evidence-report.json",
    source_paths=SOURCE_PATHS, commands=COMMANDS, focused_commands=FOCUSED_COMMANDS,
    rust_focused_commands=frozenset(), security_requirement_ids=SECURITY_REQUIREMENTS,
    implemented_contracts=IMPLEMENTED, verification_evidence=VERIFICATION,
    blockers=BLOCKERS, summary=SUMMARY,
)
if __name__ == "__main__": raise SystemExit(main(DEFINITION))
