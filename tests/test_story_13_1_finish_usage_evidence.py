from __future__ import annotations

import copy
import subprocess
import unittest
from unittest.mock import patch

from scripts import story_13_1_finish_usage_evidence as evidence


def report() -> dict[str, object]:
    raw = "\n".join(evidence.MARKERS).encode()
    with patch.object(evidence, "git_file", return_value=b"committed-source"):
        return evidence.expected_report("a" * 40, raw)


class Story131FinishUsageEvidenceTests(unittest.TestCase):
    def test_symbolic_revision_resolves_to_exact_commit(self) -> None:
        self.assertRegex(evidence.resolve_revision("HEAD"), r"^[0-9a-f]{40}$")
        with patch.object(subprocess, "run") as run:
            run.return_value.returncode = 1
            run.return_value.stdout = ""
            with self.assertRaisesRegex(ValueError, "revision is unavailable"):
                evidence.resolve_revision("missing")

    def test_every_execution_marker_is_required(self) -> None:
        valid = "\n".join(evidence.MARKERS)
        self.assertEqual(evidence.validate_raw(valid), [])
        for marker in evidence.MARKERS:
            self.assertTrue(evidence.validate_raw(valid.replace(marker, "")), marker)

    def test_matrix_is_complete_and_preserves_claim_limits(self) -> None:
        value = report()
        self.assertEqual(set(value["ctx_finish"]["finish_matrix"]), set(evidence.FINISH_MATRIX))
        self.assertEqual(set(value["ctx_finish"]["usage_facts"]), set(evidence.USAGE_FACTS))
        self.assertFalse(value["ctx_finish"]["incomplete_proposal_dispatch"])
        self.assertFalse(value["ctx_finish"]["hidden_overflow_retry"])
        self.assertFalse(value["claims"]["native_model_trial_executed"])
        self.assertFalse(value["claims"]["model_profile_enabled"])
        self.assertEqual(value["claims"]["release_claim"], "none")

    def test_missing_case_and_overclaim_mutations_fail(self) -> None:
        raw = "\n".join(evidence.MARKERS).encode()
        baseline = report()
        mutations = (
            lambda value: value["ctx_finish"]["finish_matrix"].pop(),
            lambda value: value["ctx_finish"]["usage_facts"].pop(),
            lambda value: value["ctx_finish"].update({"incomplete_proposal_dispatch": True}),
            lambda value: value["ctx_finish"].update({"hidden_overflow_retry": True}),
            lambda value: value["claims"].update({"native_model_trial_executed": True}),
            lambda value: value["claims"].update({"model_profile_enabled": True}),
            lambda value: value["claims"].update({"release_claim": "ready"}),
        )
        for mutate in mutations:
            changed = copy.deepcopy(baseline)
            mutate(changed)
            with patch.object(evidence, "git_file", return_value=b"committed-source"):
                self.assertTrue(evidence.validate_report(changed, "a" * 40, raw))


if __name__ == "__main__":
    unittest.main()
