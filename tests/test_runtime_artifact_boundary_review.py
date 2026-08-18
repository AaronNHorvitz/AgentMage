from __future__ import annotations

import copy
import unittest

from scripts.runtime_artifact_boundary_review import (
    LIMITATIONS,
    REPORT_PATH,
    build_report,
    git_revision,
    read_report,
    seal_report,
    validate_report,
)


class RuntimeArtifactBoundaryReviewTests(unittest.TestCase):
    def test_current_committed_boundary_passes_every_independent_check(self) -> None:
        report = seal_report(build_report(git_revision("HEAD")))
        self.assertEqual(len(report["checks"]), 14)
        self.assertTrue(all(check["passed"] for check in report["checks"]))
        self.assertEqual(
            report["status"], "pass-independent-automated-source-boundary-review"
        )

    def test_review_seal_and_every_overclaim_are_rejected(self) -> None:
        report = seal_report(build_report(git_revision("HEAD")))
        self.assertEqual(validate_report(report), [])
        for field in (
            "independent_human_review_performed",
            "independent_cryptographic_review_performed",
            "manual_fuzzing_executed",
            "release_approved",
        ):
            changed = copy.deepcopy(report)
            changed[field] = True
            self.assertNotEqual(validate_report(changed), [], field)

    def test_check_source_and_report_digest_mutations_fail(self) -> None:
        report = seal_report(build_report(git_revision("HEAD")))
        for mutate in (
            lambda value: value["checks"][0].update(passed=False),
            lambda value: value["sources"][0].update(sha256="f" * 64),
            lambda value: value.update(report_sha256="f" * 64),
        ):
            changed = copy.deepcopy(report)
            mutate(changed)
            self.assertNotEqual(validate_report(changed), [])

    def test_limitations_preserve_unavailable_review_boundaries(self) -> None:
        combined = " ".join(LIMITATIONS)
        self.assertIn("not an independent human or cryptographic review", combined)
        self.assertIn("Physical", combined)
        self.assertIn("Manual fuzzing", combined)
        self.assertIn("No release approval", combined)

    @unittest.skipUnless(REPORT_PATH.is_file(), "retained review follows source commit")
    def test_retained_review_matches_its_committed_source(self) -> None:
        self.assertEqual(validate_report(read_report()), [])


if __name__ == "__main__":
    unittest.main()
