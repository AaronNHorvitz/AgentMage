"""Mutation tests for the gate-owned Sprint 35 transaction review."""

from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import sprint_35_transaction_review as review


def report() -> dict[str, object]:
    transaction = b" ".join((
        b"shadow_change_preview_and_grant_bind_every_exact_input",
        b"fresh_preimages_validate_without_consuming_or_writing",
        b"stale_partial_excluded_generated_duplicate_and_invalid_proposals_fail_closed",
        b"changed_preview_decision_expiry_and_policy_cannot_issue_authority",
        b"changed_preimage_invalidates_the_single_use_grant",
        b"preimage_sha256 expected_postimage_sha256 operation_sha256",
        b"preimage_bytes rollback Restore the reviewed preimage",
        b"MAX_WRITE_GRANT_LIFETIME_MS single-use operation grant invalidate_issued_grant",
        b"Success is not write authority Sprint 36 must",
    ))
    requirements = " ".join(review.SECURITY_REQUIREMENTS).encode()
    with patch.object(
        review, "git_bytes",
        side_effect=lambda _revision, path: transaction if path.endswith("write_approval.rs") else requirements,
    ):
        return review.expected("a" * 40)


class Sprint35TransactionReviewTests(unittest.TestCase):
    def test_complete_review_passes(self) -> None:
        value = report()
        with patch.object(review, "expected", return_value=value):
            self.assertEqual(review.validate(value), [])

    def test_suppression_human_claim_mapping_and_source_drift_fail(self) -> None:
        for mutate in (
            lambda value: value["checks"].update({"attack_traces_fail_closed": False}),
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
