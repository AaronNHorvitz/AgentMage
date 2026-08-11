import json
import unittest

from scripts import adversarial_grant_corpus as corpus


class AdversarialGrantCorpusTests(unittest.TestCase):
    def test_typed_corpus_is_reproducible_complete_and_fail_closed(self) -> None:
        coverage = corpus.check_corpus()
        self.assertEqual(coverage["case_count"], 560)
        self.assertEqual(coverage["mutation_class_count"], 14)
        self.assertEqual(coverage["cases_per_class"], 40)
        self.assertEqual(coverage["atomic_consumption_case_count"], 440)
        self.assertEqual(coverage["forged_candidate_case_count"], 120)
        self.assertEqual(coverage["admitted_attempt_count"], 0)

    def test_missing_case_and_admitted_attempt_are_rejected(self) -> None:
        retained = json.loads(corpus.CORPUS_PATH.read_text(encoding="utf-8"))
        retained["cases"].pop()
        retained["case_count"] -= 1
        with self.assertRaises(corpus.AdversarialGrantCorpusError):
            corpus.validate_corpus(json.dumps(retained).encode("utf-8"))

        retained = json.loads(corpus.CORPUS_PATH.read_text(encoding="utf-8"))
        retained["cases"][0]["admitted_attempts"] = 1
        with self.assertRaises(corpus.AdversarialGrantCorpusError):
            corpus.validate_corpus(json.dumps(retained).encode("utf-8"))

    def test_wrong_scope_status_and_duplicate_seed_are_rejected(self) -> None:
        for field, value in (
            ("expected_denial_scope", "grant"),
            ("resulting_status", "consumed"),
            ("seed", None),
        ):
            retained = json.loads(corpus.CORPUS_PATH.read_text(encoding="utf-8"))
            retained["cases"][0][field] = (
                retained["cases"][1]["seed"] if field == "seed" else value
            )
            with self.assertRaises(corpus.AdversarialGrantCorpusError):
                corpus.validate_corpus(json.dumps(retained).encode("utf-8"))


if __name__ == "__main__":
    unittest.main()
