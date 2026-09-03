"""Mutation tests for the gate-owned Sprint 36 transaction review."""

from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import sprint_36_transaction_review as review


def report() -> dict[str, object]:
    all_tokens = b" ".join((
        " ".join(review.SECURITY_REQUIREMENTS).encode(),
        b"transition_matrix_and_receipt_tampering_fail_closed verify_write_receipts previous_receipt_sha256",
        b"s_029_ut02_every_post_preview_binding_mutation_is_inert s_029_st01_consumed_grant_and_approval_replay_never_reapply",
        b"preimage_sha256 postimage_sha256 consumed_grant_sha256",
        b"ordered_partial_failure_restores_changed_prefix_and_supersedes_later_work observations_match_preimages restore_after_failure",
        b"s_029_st01_native_descriptor_races_preserve_competing_state s_029_st01_parent_rename_at_every_boundary_restores_authorized_object s_029_rt01_process_stops_leave_only_reviewed_target_bytes",
        b"uncertain_malformed_and_failed_restoration_never_claim_commit_or_retry",
        b'"native_atomicity_proven": False "crash_durability_matrix_complete": False complete cross-platform race matrix',
    ))
    with patch.object(review, "git_bytes", return_value=all_tokens):
        return review.expected("a" * 40)


class Sprint36TransactionReviewTests(unittest.TestCase):
    def test_complete_review_passes(self) -> None:
        value = report()
        with patch.object(review, "expected", return_value=value):
            self.assertEqual(review.validate(value), [])

    def test_suppression_human_mapping_and_source_drift_fail(self) -> None:
        for mutate in (
            lambda value: value["checks"].update({"uncertain_results_never_claim_commit_or_retry": False}),
            lambda value: value.update({"independent_human_review_performed": True}),
            lambda value: value["security_requirement_ids"].pop(),
            lambda value: value["source_sha256"].pop(next(iter(value["source_sha256"]))),
        ):
            original = report()
            changed = copy.deepcopy(original)
            mutate(changed)
            with patch.object(review, "expected", return_value=original):
                self.assertTrue(review.validate(changed))


if __name__ == "__main__":
    unittest.main()
