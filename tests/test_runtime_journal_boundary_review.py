from __future__ import annotations

import copy
import unittest

from scripts.runtime_journal_boundary_review import (
    EVENT_FAMILIES,
    LIMITATIONS,
    REPORT_PATH,
    build_report,
    git_revision,
    read_report,
    seal_report,
    validate_report,
)


class RuntimeJournalBoundaryReviewTests(unittest.TestCase):
    def test_current_committed_boundary_passes_every_independent_check(self) -> None:
        report = seal_report(build_report(git_revision("HEAD")))
        self.assertEqual(len(EVENT_FAMILIES), 21)
        self.assertEqual(len(report["checks"]), 12)
        self.assertTrue(all(check["passed"] for check in report["checks"]))
        self.assertEqual(
            report["status"], "pass-independent-automated-source-boundary-review"
        )

    def test_review_seal_and_every_overclaim_are_rejected(self) -> None:
        report = seal_report(build_report(git_revision("HEAD")))
        self.assertEqual(validate_report(report), [])
        for field in (
            "independent_human_review_performed",
            "manual_fuzzing_executed",
            "release_approved",
        ):
            changed = copy.deepcopy(report)
            changed[field] = True
            self.assertNotEqual(validate_report(changed), [], field)

    def test_check_result_source_digest_and_report_digest_mutations_fail(self) -> None:
        report = seal_report(build_report(git_revision("HEAD")))
        for mutate in (
            lambda value: value["checks"][0].update(passed=False),
            lambda value: value["sources"][0].update(sha256="f" * 64),
            lambda value: value.update(report_sha256="f" * 64),
        ):
            changed = copy.deepcopy(report)
            mutate(changed)
            self.assertNotEqual(validate_report(changed), [])

    def test_limitations_preserve_external_and_physical_review_boundaries(self) -> None:
        combined = " ".join(LIMITATIONS)
        self.assertIn("not an independent human review", combined)
        self.assertIn("Physical filesystem/device", combined)
        self.assertIn("Manual fuzzing", combined)
        self.assertIn("No release approval", combined)

    @unittest.skipUnless(REPORT_PATH.is_file(), "retained review follows source commit")
    def test_retained_review_matches_its_committed_source(self) -> None:
        self.assertEqual(validate_report(read_report()), [])


if __name__ == "__main__":
    unittest.main()
