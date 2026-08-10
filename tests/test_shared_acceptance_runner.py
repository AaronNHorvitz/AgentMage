from __future__ import annotations

import copy
import json
import tempfile
import unittest
from pathlib import Path

from scripts.shared_acceptance_runner import (
    CancellationToken,
    CheckObservation,
    EXPECTED_CASES,
    PROFILE_PATH,
    build_report,
    canonical_json,
    check_report,
    default_handlers,
    read_json,
    run_acceptance,
    sha256_bytes,
    summary_for,
    validate_profile,
    validate_report,
    validate_run,
    write_run,
)


class SharedAcceptanceRunnerTests(unittest.TestCase):
    def setUp(self) -> None:
        self.profile = read_json(PROFILE_PATH)
        self.run = run_acceptance(self.profile)

    def test_checked_in_profile_canonical_run_and_report_are_current(self) -> None:
        self.assertEqual(validate_profile(self.profile), [])
        self.assertEqual(validate_run(self.run, self.profile), [])
        self.assertEqual(check_report(), [])

    def test_canonical_runner_executes_every_registered_check_in_order(self) -> None:
        self.assertEqual(
            [result["test_id"] for result in self.run["results"]],
            [case[0] for case in EXPECTED_CASES],
        )
        self.assertEqual(len(self.run["results"]), 8)
        self.assertTrue(all(result["status"] == "pass" for result in self.run["results"]))
        self.assertTrue(all(result["expectation_met"] for result in self.run["results"]))
        self.assertEqual(self.run["summary"]["overall_status"], "pass")
        self.assertEqual(self.run["summary"]["status_counts"]["pass"], 8)

    def test_canonical_run_is_byte_deterministic_and_uses_logical_timing(self) -> None:
        second = run_acceptance(self.profile)
        self.assertEqual(canonical_json(self.run), canonical_json(second))
        self.assertEqual(self.run["run_sha256"], second["run_sha256"])
        self.assertEqual(self.run["summary"]["total_logical_duration_ms"], 108)
        self.assertTrue(
            all(
                result["timing_kind"] == "synthetic-logical-not-wall-clock"
                for result in self.run["results"]
            )
        )

    def test_handler_registration_must_match_the_plan_exactly(self) -> None:
        missing = default_handlers()
        missing.pop("golden-integrity")
        extra = default_handlers()
        extra["undeclared-check"] = lambda _root: CheckObservation(True, "pass", {})
        with self.assertRaises(ValueError):
            run_acceptance(self.profile, handlers=missing)
        with self.assertRaises(ValueError):
            run_acceptance(self.profile, handlers=extra)

    def test_failed_check_is_retained_and_changes_overall_status(self) -> None:
        handlers = default_handlers()
        handlers["golden-integrity"] = lambda _root: CheckObservation(
            False, "synthetic expected failure", {"failure_count": 1}
        )
        run = run_acceptance(self.profile, handlers=handlers)
        target = next(result for result in run["results"] if result["check_id"] == "golden-integrity")
        self.assertEqual(target["status"], "fail")
        self.assertFalse(target["expectation_met"])
        self.assertEqual(run["summary"]["status_counts"]["fail"], 1)
        self.assertEqual(run["summary"]["overall_status"], "fail")
        self.assertEqual(validate_run(run, self.profile), [])

    def test_skipped_check_is_visible_and_never_converted_to_pass(self) -> None:
        handlers = default_handlers()
        handlers["document-structure-safety"] = lambda _root: CheckObservation(
            False, "synthetic dependency unavailable", {"dependency_available": False}, skipped=True
        )
        run = run_acceptance(self.profile, handlers=handlers)
        target = next(
            result for result in run["results"] if result["check_id"] == "document-structure-safety"
        )
        self.assertEqual(target["status"], "skipped")
        self.assertFalse(target["expectation_met"])
        self.assertEqual(run["summary"]["status_counts"]["skipped"], 1)

    def test_handler_exception_is_error_and_private_diagnostic_is_redacted(self) -> None:
        handlers = default_handlers()

        def raise_private_error(_root: Path) -> CheckObservation:
            raise RuntimeError(
                "/home/synthetic/private.txt AM_SYNTHETIC_SECRET_ACCEPTANCE_TEST"
            )

        handlers["corpus-integrity"] = raise_private_error
        run = run_acceptance(self.profile, handlers=handlers)
        target = run["results"][0]
        retained = target["diagnostic"]["retained_diagnostic"]
        self.assertEqual(target["status"], "error")
        self.assertNotIn("/home/", retained)
        self.assertNotIn("AM_SYNTHETIC_SECRET_", retained)
        self.assertIn("<REDACTED>", retained)
        self.assertEqual(validate_run(run, self.profile), [])

    def test_invalid_handler_metrics_become_a_visible_error(self) -> None:
        handlers = default_handlers()
        handlers["platform-redaction"] = lambda _root: CheckObservation(
            True, "invalid metric", {"private_path": "/home/synthetic/value"}
        )
        run = run_acceptance(self.profile, handlers=handlers)
        target = next(result for result in run["results"] if result["check_id"] == "platform-redaction")
        self.assertEqual(target["status"], "error")
        self.assertEqual(target["metrics"], {})

    def test_pre_cancelled_run_records_every_case_as_cancelled(self) -> None:
        token = CancellationToken()
        token.cancel()
        run = run_acceptance(self.profile, token=token)
        self.assertEqual(len(run["results"]), 8)
        self.assertTrue(all(result["status"] == "cancelled" for result in run["results"]))
        self.assertEqual(run["summary"]["status_counts"]["cancelled"], 8)
        self.assertEqual(run["summary"]["overall_status"], "fail")
        self.assertEqual(validate_run(run, self.profile), [])

    def test_diagnostics_are_bounded_hashed_and_raw_text_is_not_retained(self) -> None:
        handlers = default_handlers()
        handlers["fake-adapter-closure"] = lambda _root: CheckObservation(
            False, "X" * 1000, {"failure_count": 1}
        )
        run = run_acceptance(self.profile, handlers=handlers)
        target = next(result for result in run["results"] if result["check_id"] == "fake-adapter-closure")
        diagnostic = target["diagnostic"]
        self.assertTrue(diagnostic["truncated"])
        self.assertEqual(diagnostic["retained_bytes"], 256)
        self.assertEqual(
            diagnostic["retained_sha256"],
            sha256_bytes(diagnostic["retained_diagnostic"].encode("utf-8")),
        )
        self.assertNotIn("raw_diagnostic", diagnostic)

    def test_result_and_run_hashes_detect_mutation(self) -> None:
        mutated = copy.deepcopy(self.run)
        mutated["results"][0]["status"] = "fail"
        failures = validate_run(mutated, self.profile)
        self.assertTrue(any("run hash" in failure for failure in failures))
        self.assertTrue(any("result hash" in failure for failure in failures))
        self.assertTrue(any("expectation flag" in failure for failure in failures))

    def test_summary_omission_is_detected(self) -> None:
        mutated = copy.deepcopy(self.run)
        mutated["summary"]["status_counts"]["pass"] = 6
        self.assertTrue(any("summary" in failure for failure in validate_run(mutated, self.profile)))
        self.assertEqual(self.run["summary"], summary_for(self.run["results"]))

    def test_run_writer_is_atomic_repeatable_non_executable_and_no_overwrite(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            first = Path(temporary) / "first.json"
            second = Path(temporary) / "second.json"
            write_run(self.profile, first)
            write_run(self.profile, second)
            self.assertEqual(first.read_bytes(), second.read_bytes())
            self.assertEqual(first.read_bytes(), canonical_json(self.run))
            self.assertEqual(first.stat().st_mode & 0o111, 0)
            with self.assertRaises(FileExistsError):
                write_run(self.profile, first)

    def test_profile_cannot_add_checks_enable_network_or_unblock_macos(self) -> None:
        added = copy.deepcopy(self.profile)
        added["case_order"].append(copy.deepcopy(added["case_order"][0]))
        networked = copy.deepcopy(self.profile)
        networked["execution_contract"]["network"] = True
        unblocked = copy.deepcopy(self.profile)
        unblocked["macos_execution_status"] = "pass"
        self.assertTrue(validate_profile(added))
        self.assertTrue(validate_profile(networked))
        self.assertTrue(validate_profile(unblocked))

    def test_report_is_hash_only_and_does_not_claim_product_acceptance(self) -> None:
        report = build_report()
        serialized = json.dumps(report, sort_keys=True)
        self.assertNotIn("retained_diagnostic", serialized)
        self.assertFalse(report["raw_output_retained"])
        self.assertFalse(report["run_record_persisted"])
        self.assertEqual(report["product_acceptance_claim"], "none")
        self.assertEqual(report["macos_execution_status"], "blocked-macos")
        mutated = copy.deepcopy(report)
        mutated["product_acceptance_claim"] = "release-pass"
        self.assertTrue(validate_report(mutated))


if __name__ == "__main__":
    unittest.main()
