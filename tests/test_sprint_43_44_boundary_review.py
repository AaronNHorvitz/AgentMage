"""Mutation tests for Sprint 43/44 gate-owned reviews."""
import copy, subprocess, unittest
from unittest.mock import patch
from scripts import sprint_43_44_boundary_review as review

class Sprint4344BoundaryReviewTests(unittest.TestCase):
    def test_contracts_and_mutations(self) -> None:
        revision = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
        for sprint in (43, 44):
            value = review.expected(sprint, revision)
            self.assertEqual(value["status"], "PASS_LOCAL_BOUNDARY_REVIEW")
            with patch.object(review, "expected", return_value=value): self.assertEqual(review.validate(sprint, value), [])
            for key in ("checks", "security_requirement_ids", "source_sha256"):
                changed = copy.deepcopy(value)
                if key == "checks": changed[key][next(iter(changed[key]))] = False
                else: changed[key].pop(next(iter(changed[key])) if isinstance(changed[key], dict) else -1)
                with patch.object(review, "expected", return_value=value): self.assertTrue(review.validate(sprint, changed))

if __name__ == "__main__": unittest.main()
