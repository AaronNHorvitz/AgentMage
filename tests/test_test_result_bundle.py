from __future__ import annotations

import copy
import json
import tempfile
import unittest
from pathlib import Path

from scripts.test_result_bundle import (
    EXPECTED_CASE_IDS,
    PROFILE_PATH,
    SYNTHETIC_CANARY,
    build_bundle,
    build_report,
    canonical_json,
    check_report,
    classify,
    flattened_results,
    read_json,
    sha256_bytes,
    shard_for,
    summarize,
    validate_bundle,
    validate_profile,
    validate_report,
    write_bundle,
)


class TestResultBundleTests(unittest.TestCase):
    def setUp(self) -> None:
        self.profile = read_json(PROFILE_PATH)
        self.bundle = build_bundle(self.profile)
        self.results = flattened_results(self.bundle)
        self.by_id = {result["test_id"]: result for result in self.results}

    def test_checked_in_profile_bundle_and_report_are_current(self) -> None:
        self.assertEqual(validate_profile(self.profile), [])
        self.assertEqual(validate_bundle(self.bundle, self.profile), [])
        self.assertEqual(check_report(), [])

    def test_sharding_is_stable_complete_and_non_overlapping(self) -> None:
        self.assertEqual(set(self.by_id), set(EXPECTED_CASE_IDS))
        self.assertEqual(len(self.results), len(EXPECTED_CASE_IDS))
        for result in self.results:
            self.assertEqual(result["shard"], shard_for(result["test_id"], 3))
        second = build_bundle(self.profile)
        self.assertEqual(
            [(shard["index"], shard["shard_sha256"]) for shard in self.bundle["shards"]],
            [(shard["index"], shard["shard_sha256"]) for shard in second["shards"]],
        )

    def test_retry_and_flaky_classifications_are_exact(self) -> None:
        expected = {
            "parser.normal": "pass",
            "parser.persistent-failure": "persistent-failure",
            "model.flaky": "flaky",
            "runtime.timeout-flaky": "flaky",
            "documents.skipped": "skipped",
            "known.quarantined-failure": "quarantined-failure",
            "worker.timeout-exhausted": "retry-exhausted",
            "output.oversized": "pass",
            "worker.cancelled": "cancelled",
        }
        self.assertEqual(
            {test_id: result["classification"] for test_id, result in self.by_id.items()},
            expected,
        )
        self.assertTrue(self.by_id["model.flaky"]["flaky"])
        self.assertTrue(self.by_id["runtime.timeout-flaky"]["flaky"])
        self.assertFalse(self.by_id["parser.normal"]["flaky"])

    def test_classify_never_turns_a_non_pass_terminal_state_into_pass(self) -> None:
        self.assertEqual(classify(["fail", "fail", "fail"], False), "persistent-failure")
        self.assertEqual(classify(["timeout", "timeout", "timeout"], False), "retry-exhausted")
        self.assertEqual(classify(["fail"], True), "quarantined-failure")
        self.assertEqual(classify(["skipped"], False), "skipped")
        self.assertEqual(classify(["cancelled"], False), "cancelled")

    def test_summary_reconciles_failure_skip_retry_flake_quarantine_and_truncation(self) -> None:
        summary = self.bundle["summary"]
        self.assertEqual(summary, summarize(self.results))
        self.assertEqual(summary["test_count"], 9)
        self.assertEqual(summary["retry_attempt_count"], 6)
        self.assertEqual(summary["retried_test_count"], 4)
        self.assertEqual(summary["flaky_test_count"], 2)
        self.assertEqual(summary["quarantined_test_count"], 1)
        self.assertEqual(summary["skipped_test_count"], 1)
        self.assertEqual(summary["truncated_attempt_count"], 1)
        self.assertEqual(summary["redacted_attempt_count"], 1)
        self.assertEqual(summary["non_pass_test_count"], 5)
        self.assertEqual(summary["overall_status"], "fail")

    def test_outputs_are_bounded_hashed_and_canary_redacted(self) -> None:
        for result in self.results:
            for attempt in result["attempts"]:
                output = attempt["output"]
                self.assertLessEqual(output["retained_bytes"], 64)
                self.assertEqual(
                    output["retained_sha256"],
                    sha256_bytes(output["retained_output"].encode("utf-8")),
                )
                self.assertNotIn("raw_output", output)
                self.assertIsNone(SYNTHETIC_CANARY.search(output["retained_output"]))
        oversized = self.by_id["output.oversized"]["attempts"][0]["output"]
        self.assertTrue(oversized["truncated"])
        self.assertEqual(oversized["retained_bytes"], 64)
        redacted = self.by_id["parser.persistent-failure"]["attempts"][0]["output"]
        self.assertEqual(redacted["redaction_count"], 1)
        self.assertIn("<SYNTHETIC_CANARY_REDACTED>", redacted["retained_output"])

    def test_result_shard_and_bundle_hashes_detect_tampering(self) -> None:
        mutated = copy.deepcopy(self.bundle)
        target = next(result for result in flattened_results(mutated) if result["test_id"] == "parser.normal")
        target["final_status"] = "fail"
        failures = validate_bundle(mutated, self.profile)
        self.assertTrue(any("bundle hash" in failure for failure in failures))
        self.assertTrue(any("shard hash" in failure for failure in failures))
        self.assertTrue(any("result hash" in failure for failure in failures))

    def test_summary_omission_or_pass_conversion_is_rejected(self) -> None:
        omitted = copy.deepcopy(self.bundle)
        omitted["summary"]["skipped_test_count"] = 0
        converted = copy.deepcopy(self.bundle)
        target = next(
            result
            for result in flattened_results(converted)
            if result["test_id"] == "parser.persistent-failure"
        )
        target["classification"] = "pass"
        self.assertTrue(any("summary" in failure for failure in validate_bundle(omitted, self.profile)))
        self.assertTrue(any("converted to pass" in failure for failure in validate_bundle(converted, self.profile)))

    def test_shard_reassignment_or_duplicate_result_is_rejected(self) -> None:
        reassigned = copy.deepcopy(self.bundle)
        source = next(shard for shard in reassigned["shards"] if shard["results"])
        result = source["results"].pop()
        destination = next(shard for shard in reassigned["shards"] if shard["index"] != source["index"])
        destination["results"].append(result)
        duplicated = copy.deepcopy(self.bundle)
        duplicated["shards"][0]["results"].append(copy.deepcopy(self.results[0]))
        self.assertTrue(any("shard assignment" in failure for failure in validate_bundle(reassigned, self.profile)))
        self.assertTrue(any("omitted or duplicated" in failure for failure in validate_bundle(duplicated, self.profile)))

    def test_bundle_writer_is_atomic_repeatable_non_executable_and_no_overwrite(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            first = Path(temporary) / "first.json"
            second = Path(temporary) / "second.json"
            write_bundle(self.profile, first)
            write_bundle(self.profile, second)
            self.assertEqual(first.read_bytes(), second.read_bytes())
            self.assertEqual(first.read_bytes(), canonical_json(self.bundle))
            self.assertEqual(first.stat().st_mode & 0o111, 0)
            with self.assertRaises(FileExistsError):
                write_bundle(self.profile, first)

    def test_terminal_retry_and_early_retry_exhaustion_are_rejected(self) -> None:
        terminal_retry = copy.deepcopy(self.profile)
        terminal_retry["cases"][0]["attempts"].append(
            {"duration_ms": 1, "output": "invalid retry", "status": "pass"}
        )
        early_stop = copy.deepcopy(self.profile)
        early_stop["cases"][1]["attempts"].pop()
        self.assertTrue(validate_profile(terminal_retry))
        self.assertTrue(validate_profile(early_stop))

    def test_profile_cannot_expand_output_or_suppress_non_pass_results(self) -> None:
        expanded = copy.deepcopy(self.profile)
        expanded["output_policy"]["max_retained_bytes_per_attempt"] = 4096
        suppressing = copy.deepcopy(self.profile)
        suppressing["side_effect_contract"]["suppresses_non_pass_results"] = True
        unblocked = copy.deepcopy(self.profile)
        unblocked["macos_execution_status"] = "pass"
        self.assertTrue(validate_profile(expanded))
        self.assertTrue(validate_profile(suppressing))
        self.assertTrue(validate_profile(unblocked))

    def test_platform_result_context_is_bound_by_identity_and_hash(self) -> None:
        context = self.bundle["context"]
        self.assertEqual(context["platform_profile_id"], "agentmage-platform-result-recorder-v1")
        self.assertTrue(context["platform_record_id"].startswith("am-platform-result-"))
        self.assertEqual(len(context["platform_profile_sha256"]), 64)
        self.assertEqual(len(context["platform_record_sha256"]), 64)

    def test_report_retains_no_output_or_canary_and_makes_bounded_claims(self) -> None:
        report = build_report()
        serialized = json.dumps(report, sort_keys=True)
        self.assertNotIn("retained_output", serialized)
        self.assertNotIn("AM_SYNTHETIC_CANARY_", serialized)
        self.assertFalse(report["raw_output_retained"])
        self.assertFalse(report["synthetic_canary_values_retained"])
        self.assertEqual(report["content_addressing_claim"], "tamper-evident-by-hash")
        self.assertEqual(report["filesystem_immutability_claim"], "none")
        mutated = copy.deepcopy(report)
        mutated["filesystem_immutability_claim"] = "immutable-filesystem"
        self.assertTrue(validate_report(mutated))


if __name__ == "__main__":
    unittest.main()
