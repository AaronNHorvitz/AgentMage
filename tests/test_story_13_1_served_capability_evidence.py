from __future__ import annotations

import copy
import subprocess
import unittest
from unittest.mock import patch

from scripts import story_13_1_served_capability_evidence as evidence


def report() -> dict[str, object]:
    raw = "\n".join(evidence.MARKERS).encode()
    with patch.object(evidence, "git_file", return_value=b"committed-source"):
        return evidence.expected_report("a" * 40, raw)


class Story131ServedCapabilityEvidenceTests(unittest.TestCase):
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

    def test_local_matrix_preserves_native_and_release_limits(self) -> None:
        value = report()
        self.assertTrue(value["ctx_served"]["zero_generation_dispatch_on_refusal"])
        self.assertFalse(value["claims"]["native_boundary_campaign_executed"])
        self.assertFalse(value["claims"]["model_profile_enabled"])
        self.assertEqual(value["claims"]["release_claim"], "none")
        self.assertEqual(
            set(value["ctx_served"]["deterministic_adapter_matrix"]),
            set(evidence.REJECTION_MATRIX),
        )

    def test_missing_case_and_overclaim_mutations_fail(self) -> None:
        raw = "\n".join(evidence.MARKERS).encode()
        baseline = report()
        mutations = (
            lambda value: value["ctx_served"]["deterministic_adapter_matrix"].pop(),
            lambda value: value["ctx_served"].update({"zero_generation_dispatch_on_refusal": False}),
            lambda value: value["claims"].update({"native_boundary_campaign_executed": True}),
            lambda value: value["claims"].update({"model_profile_enabled": True}),
            lambda value: value["claims"].update({"platform_qualified": True}),
            lambda value: value["claims"].update({"release_claim": "ready"}),
        )
        for mutate in mutations:
            changed = copy.deepcopy(baseline)
            mutate(changed)
            with patch.object(evidence, "git_file", return_value=b"committed-source"):
                self.assertTrue(evidence.validate_report(changed, "a" * 40, raw))


if __name__ == "__main__":
    unittest.main()
