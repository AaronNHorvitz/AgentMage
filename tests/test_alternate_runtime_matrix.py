"""Mutation tests for the non-activating Story 49.2 adapter matrix."""

import copy
import subprocess
import unittest
from unittest.mock import patch

from scripts import alternate_runtime_matrix as matrix


class AlternateRuntimeMatrixTests(unittest.TestCase):
    def test_current_matrix_is_complete_and_non_activating(self) -> None:
        self.assertEqual(matrix.validate(matrix.load()), [])

    def test_authority_semantic_identity_and_evidence_mutations_fail(self) -> None:
        value = matrix.load()
        mutations = (
            lambda changed: changed.update({"activation_authority": True}),
            lambda changed: changed.update({"enabled_route_count": 1}),
            lambda changed: changed.update({"silent_fallback": True}),
            lambda changed: changed["canonical_semantics"].pop(),
            lambda changed: changed["semantic_dispositions"]["unavailable"].pop("usage"),
            lambda changed: changed["candidates"].pop(),
            lambda changed: changed["candidates"][1].update({"exact_version": "latest"}),
            lambda changed: changed["candidates"][1].update({"support_claim": True}),
            lambda changed: changed["candidates"][1].update({"route_enabled": True}),
            lambda changed: changed["fake_parity_cases"].pop(),
            lambda changed: changed.update({"performance_status": "pass"}),
            lambda changed: changed["staleness_policy"].update({"version_change_invalidates_prior_evidence": False}),
        )
        for mutate in mutations:
            changed = copy.deepcopy(value)
            mutate(changed)
            self.assertTrue(matrix.validate(changed))

    def test_report_is_revision_bound_and_rejects_widening(self) -> None:
        revision = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
        with patch.object(
            matrix,
            "git_bytes",
            side_effect=lambda _revision, path: (matrix.ROOT / path).read_bytes(),
        ):
            value = matrix.expected_report(revision)
        with patch.object(matrix, "expected_report", return_value=value):
            self.assertEqual(matrix.validate_report(value), [])
            changed = copy.deepcopy(value)
            changed["supported_adapter_count"] = 1
            self.assertTrue(matrix.validate_report(changed))


if __name__ == "__main__":
    unittest.main()
