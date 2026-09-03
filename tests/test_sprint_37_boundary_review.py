"""Mutation tests for the gate-owned Sprint 37 boundary review."""

from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import sprint_37_boundary_review as review


def report() -> dict[str, object]:
    tokens = b" ".join((
        " ".join(review.REQUIREMENTS).encode(),
        b"create patch copy move trash preview_sha256 preimage_sha256 postimage_sha256",
        b'"schema_version" "cases" "record_type"',
        b"restoration native_process_stop_matrix terminal_uncertain_no_replay",
        b'"ubuntu_native_fixture_evidence": False "macos_native_driver": False "windows_native_driver": False',
        b'"isolated_write_worker_proven": False "complete_crash_durability_matrix": False',
        b'"generic_shell": False "network_access": False "external_delivery": False',
    ))
    with patch.object(review, "git_bytes", return_value=tokens):
        return review.expected("a" * 40)


class Sprint37BoundaryReviewTests(unittest.TestCase):
    def test_complete_review_passes(self) -> None:
        value = report()
        with patch.object(review, "expected", return_value=value):
            self.assertEqual(review.validate(value), [])

    def test_mutations_fail(self) -> None:
        for mutate in (
            lambda value: value["checks"].update({"operation_matrix_is_retained": False}),
            lambda value: value.update({"independent_human_review_performed": True}),
            lambda value: value["security_requirement_ids"].pop(),
            lambda value: value["source_sha256"].pop(next(iter(value["source_sha256"]))),
        ):
            original = report(); changed = copy.deepcopy(original); mutate(changed)
            with patch.object(review, "expected", return_value=original):
                self.assertTrue(review.validate(changed))


if __name__ == "__main__": unittest.main()
