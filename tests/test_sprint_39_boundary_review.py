"""Mutation tests for the gate-owned Sprint 39 boundary review."""

from __future__ import annotations

import copy
import json
import unittest
from unittest.mock import patch

from scripts import sprint_39_boundary_review as review


def report() -> dict[str, object]:
    local = {
        "security_requirement_ids": review.REQUIREMENTS,
        "implemented_contracts": {
            **{key: True for key in (
                "eleven_boundary_privacy_gate", "removed_content_not_retained",
                "native_write_producer_privacy_gate", "content_free_staging_diagnostics",
                "separately_receipted_cleanup", "all_runtime_roots_scanned",
                "deterministic_recovery_precedence", "hash_chained_checkpoint_validation",
                "redacted_human_audit", "formal_runtime_checkpoint_schema",
            )},
            "completed_write_replay_allowed": False,
        },
        "verification_evidence": {
            **{key: True for key in (
                "native_end_to_end_recovery", "native_derived_index_recovery",
                "native_internal_process_stop_matrices", "complete_native_crash_concurrency_matrix",
            )},
            **{key: False for key in (
                "upstream_sprint_38_gate", "trusted_package_launcher_environment",
                "non_fedora_native_evidence", "independent_review", "manual_fuzzing",
                "physical_enospc_executed",
            )},
        },
    }
    payloads = {path: b"retained" for path in review.SOURCES}
    payloads[review.SOURCES[5]] = json.dumps(local).encode()
    with patch.object(review, "git_bytes", side_effect=lambda _revision, path: payloads[path]):
        return review.expected("a" * 40)


class Sprint39BoundaryReviewTests(unittest.TestCase):
    def test_complete_review_passes(self) -> None:
        value = report()
        with patch.object(review, "expected", return_value=value): self.assertEqual(review.validate(value), [])

    def test_mutations_fail(self) -> None:
        for mutate in (
            lambda value: value["checks"].update({"checkpoint_and_recovery_matrix_is_bound": False}),
            lambda value: value.update({"independent_human_review_performed": True}),
            lambda value: value["security_requirement_ids"].pop(),
            lambda value: value["source_sha256"].pop(next(iter(value["source_sha256"]))),
        ):
            original = report(); changed = copy.deepcopy(original); mutate(changed)
            with patch.object(review, "expected", return_value=original): self.assertTrue(review.validate(changed))


if __name__ == "__main__": unittest.main()
