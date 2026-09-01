from __future__ import annotations

import copy

from scripts.story_50_3_security_fault_evidence import expected_report, validate_report


def test_local_security_fault_truth_is_closed() -> None:
    report = expected_report()
    assert validate_report(report) == []
    assert len(report["crash_boundary_matrix"]) == 14
    assert report["product_truth"]["false_completions"] == 0


def test_security_false_success_and_authority_mutations_fail() -> None:
    for field, value in (
        ("secret_canary_disclosures", 1),
        ("duplicate_guarded_effects", 1),
        ("unsafe_automatic_retries", 1),
        ("false_completions", 1),
        ("active_or_archive_bytes_executed", True),
        ("approval_or_grant_bypass", True),
        ("installed_active_document_parser_campaign_complete", True),
    ):
        changed = copy.deepcopy(expected_report())
        changed["product_truth"][field] = value
        assert validate_report(changed)


def test_crash_matrix_reordering_or_omission_fails() -> None:
    changed = copy.deepcopy(expected_report())
    changed["crash_boundary_matrix"].pop()
    assert validate_report(changed)
    changed = copy.deepcopy(expected_report())
    changed["crash_boundary_matrix"].reverse()
    assert validate_report(changed)
