from __future__ import annotations

import copy
import unittest

from scripts.seeded_fuzz_failures import (
    DETECTORS,
    REPORT_PATH,
    SEED_CORPUS_PATH,
    SEED_SPECS,
    campaign,
    check_artifacts,
    derived_secret,
    materialize_seed,
    minimize_payload,
    read_json,
    validate_report,
    validate_seed_corpus,
)


class SeededFuzzFailuresTests(unittest.TestCase):
    def test_checked_seed_corpus_report_and_regressions_are_current(self) -> None:
        self.assertEqual(check_artifacts(), [])
        expected, regressions = campaign()
        self.assertEqual(read_json(REPORT_PATH), expected)
        self.assertEqual(len(regressions), 6)

    def test_all_required_failure_classes_are_detected_and_non_pass(self) -> None:
        report, _ = campaign()
        expected = [item[1] for item in SEED_SPECS]
        self.assertEqual(
            [item["run"]["result_status"] for item in report["results"]], expected
        )
        self.assertTrue(all(item["run"]["result_status"] != "pass" for item in report["results"]))

    def test_minimization_is_deterministic_bounded_and_preserves_failure(self) -> None:
        corpus = read_json(SEED_CORPUS_PATH)
        for seed in corpus["seeds"]:
            payload = materialize_seed(seed)
            first = minimize_payload(payload, seed["failure_class"])
            second = minimize_payload(payload, seed["failure_class"])
            self.assertEqual(first, second)
            self.assertLessEqual(len(first), len(payload))
            self.assertTrue(DETECTORS[seed["failure_class"]](first))

    def test_results_have_ownership_blocking_disposition_and_gate_failure(self) -> None:
        report, _ = campaign()
        self.assertTrue(
            all(item["ownership"]["owner"] for item in report["results"])
        )
        self.assertTrue(
            all(
                item["ownership"]["disposition"] == "quarantined-blocking"
                for item in report["results"]
            )
        )
        self.assertTrue(all(item["status"] == "block" for item in report["gate_results"]))

    def test_secret_canary_value_is_not_persisted_in_report_or_regressions(self) -> None:
        report, regressions = campaign()
        secret = derived_secret("seed-secret-001")
        self.assertNotIn(secret, REPORT_PATH.read_bytes())
        self.assertNotIn(secret, str(report).encode("utf-8"))
        self.assertTrue(all(secret not in content for content in regressions.values()))

    def test_seed_hash_private_data_and_secret_payload_mutations_fail_closed(self) -> None:
        corpus = read_json(SEED_CORPUS_PATH)
        hash_mutation = copy.deepcopy(corpus)
        hash_mutation["seeds"][0]["seed_sha256"] = "0" * 64
        private = copy.deepcopy(corpus)
        private["seeds"][0]["private_user_data"] = True
        secret = copy.deepcopy(corpus)
        secret["seeds"][4]["payload_base64"] = "QU1fU1lOVEhFVElDX1NFQ1JFVA=="
        for changed in (hash_mutation, private, secret):
            self.assertTrue(validate_seed_corpus(changed))

    def test_pass_conversion_minimization_data_and_macos_overclaims_fail_closed(self) -> None:
        report, _ = campaign()
        converted = copy.deepcopy(report)
        converted["results"][0]["run"]["result_status"] = "pass"
        unminimized = copy.deepcopy(report)
        unminimized["results"][0]["minimized_reproducer"]["status"] = "unminimized"
        exposed = copy.deepcopy(report)
        exposed["secret_canary_values_recorded"] = True
        macos = copy.deepcopy(report)
        macos["macos_execution_status"] = "pass"
        for changed in (converted, unminimized, exposed, macos):
            self.assertTrue(validate_report(changed))


if __name__ == "__main__":
    unittest.main()
