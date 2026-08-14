import copy
import unittest

from scripts.validate_agent_failure_corpus import load_corpus, validate_corpus


class AgentFailureCorpusTests(unittest.TestCase):
    def payload(self):
        return load_corpus()

    def test_corpus_and_bound_evidence_are_current(self):
        self.assertEqual(validate_corpus(self.payload()), [])

    def test_every_case_rejects_false_success(self):
        for case in self.payload()["cases"]:
            self.assertIn("success", case["prohibited_outcomes"])
            self.assertNotIn(case["expected_disposition"], {"success", "verified_no_op"})

    def test_identity_category_and_disposition_mutations_fail(self):
        mutations = []
        changed = self.payload()
        changed["cases"][0]["case_id"] = "APF-999"
        mutations.append(changed)
        changed = self.payload()
        changed["cases"][1]["category"] = "restart"
        mutations.append(changed)
        changed = self.payload()
        changed["cases"][4]["expected_disposition"] = "success"
        mutations.append(changed)
        for mutation in mutations:
            self.assertTrue(validate_corpus(mutation, verify_files=False))

    def test_provenance_schema_and_false_success_mutations_fail(self):
        mutations = []
        changed = self.payload()
        changed["cases"][2]["evidence"]["sha256"] = "0" * 64
        mutations.append(changed)
        changed = self.payload()
        changed["cases"][3]["unexpected"] = True
        mutations.append(changed)
        changed = self.payload()
        changed["cases"][5]["prohibited_outcomes"].remove("success")
        mutations.append(changed)
        for mutation in mutations:
            self.assertTrue(validate_corpus(mutation))


if __name__ == "__main__":
    unittest.main()
