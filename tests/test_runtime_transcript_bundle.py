import unittest

from scripts import runtime_transcript_bundle as bundle


class RuntimeTranscriptBundleTests(unittest.TestCase):
    def test_current_fixture_is_canonical_and_semantically_closed(self):
        self.assertEqual(bundle.validate_current(), [])

    def test_generation_is_deterministic(self):
        self.assertEqual(bundle.transcript_fixture(), bundle.transcript_fixture())

    def test_status_and_refusal_mutations_fail(self):
        status = bundle.transcript_fixture()
        status["cases"][0]["expected"]["plan_revision_delta"] = 1
        self.assertTrue(bundle.semantic_failures(status))

        refusal = bundle.transcript_fixture()
        refusal["cases"][3]["transcript"][2]["plan_revision"] = 4
        self.assertTrue(bundle.semantic_failures(refusal))

    def test_interruption_order_and_cancellation_mutations_fail(self):
        ordering = bundle.transcript_fixture()
        ordering["cases"][1]["transcript"][1:3] = reversed(
            ordering["cases"][1]["transcript"][1:3]
        )
        self.assertTrue(bundle.semantic_failures(ordering))

        descendants = bundle.transcript_fixture()
        descendants["cases"][2]["transcript"][1]["propagated_to"].clear()
        self.assertTrue(bundle.semantic_failures(descendants))

        signal = bundle.transcript_fixture()
        signal["cases"][1]["transcript"][1]["signal"]["reason"] = "shutdown"
        self.assertTrue(bundle.semantic_failures(signal))

    def test_final_response_and_authority_mutations_fail(self):
        completion = bundle.transcript_fixture()
        completion["cases"][6]["transcript"][0]["evidence"].clear()
        self.assertTrue(bundle.semantic_failures(completion))

        false_success = bundle.transcript_fixture()
        false_success["cases"][11]["transcript"][0][
            "claims_unfinished_work_succeeded"
        ] = True
        self.assertTrue(bundle.semantic_failures(false_success))

        authority = bundle.transcript_fixture()
        authority["contract_authority"] = "execute"
        self.assertTrue(bundle.semantic_failures(authority))


if __name__ == "__main__":
    unittest.main()
