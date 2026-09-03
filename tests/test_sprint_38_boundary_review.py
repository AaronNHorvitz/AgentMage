"""Mutation tests for the gate-owned Sprint 38 boundary review."""

from __future__ import annotations

import copy
import json
import unittest
from unittest.mock import patch

from scripts import sprint_38_boundary_review as review


def report() -> dict[str, object]:
    local = {
        "security_requirement_ids": review.REQUIREMENTS,
        "implemented_contracts": {key: True for key in (
            "byte_preserving_markdown_parser", "closed_structure_preserving_edits",
            "raw_notes_protected", "plain_folder_obsidian_domain_parity",
            "exact_create_and_update_previews", "stable_identity_and_source_hash_binding",
            "case_and_unicode_namespace_collision_denial", "bulk_reorganization_unrepresentable",
            "canonical_first_index_publication", "namespace_compare_and_swap",
            "native_end_to_end_knowledge_transaction",
        )},
        "verification_evidence": {
            "native_source_index_process_stop_matrix": True,
            **{key: False for key in (
                "upstream_sprint_37_gate", "complete_native_crash_and_race_matrix",
                "trusted_package_launcher_test_environment", "non_fedora_native_evidence",
                "independent_review", "manual_fuzzing",
            )},
        },
    }
    payloads = {path: b'"generic_shell": false "network_access": false "external_delivery": false' for path in review.SOURCES}
    payloads[review.SOURCES[5]] = json.dumps(local).encode()
    with patch.object(review, "git_bytes", side_effect=lambda _revision, path: payloads[path]):
        return review.expected("a" * 40)


class Sprint38BoundaryReviewTests(unittest.TestCase):
    def test_complete_review_passes(self) -> None:
        value = report()
        with patch.object(review, "expected", return_value=value): self.assertEqual(review.validate(value), [])

    def test_mutations_fail(self) -> None:
        for mutate in (
            lambda value: value["checks"].update({"recovery_evidence_is_retained": False}),
            lambda value: value.update({"independent_human_review_performed": True}),
            lambda value: value["security_requirement_ids"].pop(),
            lambda value: value["source_sha256"].pop(next(iter(value["source_sha256"]))),
        ):
            original = report(); changed = copy.deepcopy(original); mutate(changed)
            with patch.object(review, "expected", return_value=original): self.assertTrue(review.validate(changed))


if __name__ == "__main__": unittest.main()
