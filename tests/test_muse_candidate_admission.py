from __future__ import annotations

import copy
import unittest

from scripts import muse_candidate_admission as admission


class MuseCandidateAdmissionTests(unittest.TestCase):
    def test_exact_disabled_records_validate(self) -> None:
        self.assertEqual(admission.validate(), [])

    def test_artifact_template_and_context_drift_fail(self) -> None:
        for mutate in (
            lambda record: record["source_artifacts"]["gguf"].update({"sha256": "0" * 64}),
            lambda record: record["source_artifacts"]["chat_template"].update({"sha256": "0" * 64}),
            lambda record: record["context_contract"].update({"first_evaluation_tokens": 32768}),
        ):
            record = copy.deepcopy(admission.load(admission.SOURCE))
            mutate(record)
            self.assertTrue(admission.validate_source(record))

    def test_activation_fallback_and_runtime_substitution_fail(self) -> None:
        for mutate in (
            lambda record: record["decision"].update({"status": "PASS"}),
            lambda record: record["decision"].update({"fallback": True}),
            lambda record: record["candidate_runtime"].update({"source_commit": "0" * 40}),
            lambda record: record["preserved_runtime"].update({"compatible": True}),
        ):
            record = copy.deepcopy(admission.load(admission.RUNTIME))
            mutate(record)
            self.assertTrue(admission.validate_runtime(record))

    def test_historical_source_policy_and_kernel_family_branch_drift_fail(self) -> None:
        for mutate in (
            lambda record: record["policy"].update({"sha256": "0" * 64}),
            lambda record: record["codec_contract"].update({"kernel_family_branch_allowed": True}),
            lambda record: record["ownership_and_origin"].update({"developer": "unknown"}),
        ):
            record = copy.deepcopy(admission.load(admission.SOURCE))
            mutate(record)
            self.assertTrue(admission.validate_source(record))

    def test_current_policy_binding_is_separate_and_current(self) -> None:
        self.assertEqual(admission.validate_current_policy(), [])
        original = admission.POLICY
        try:
            admission.POLICY = admission.CATALOG
            self.assertIn(
                "current catalog policy binding is stale",
                admission.validate_current_policy(),
            )
        finally:
            admission.POLICY = original


if __name__ == "__main__":
    unittest.main()
