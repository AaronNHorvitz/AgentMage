from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    "benchmark_contract", ROOT / "scripts/benchmark_contract.py"
)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def exact_tuple() -> dict[str, object]:
    return {
        "artifact_sha256": "a" * 64,
        "tokenizer_sha256": "b" * 64,
        "template_sha256": "c" * 64,
        "codec_sha256": "d" * 64,
        "runtime_sha256": "e" * 64,
        "sampler_sha256": "f" * 64,
        "context_tokens": 8192,
        "inference_slots": 1,
        "platform": "fedora-x86_64",
        "hardware_sha256": "1" * 64,
        "driver_sha256": "2" * 64,
        "tool_catalog_sha256": "3" * 64,
        "grader_sha256": "4" * 64,
        "corpus_sha256": "5" * 64,
    }


class BenchmarkContractTests(unittest.TestCase):
    def test_identical_tuple_is_comparable(self) -> None:
        result = MODULE.compare_tuples(exact_tuple(), exact_tuple())
        self.assertTrue(result.comparable)
        self.assertEqual(result.differing_fields, ())
        self.assertEqual(result.label, "COMPARABLE")

    def test_every_single_tuple_change_remains_visible(self) -> None:
        for field in MODULE.TUPLE_FIELDS:
            changed = exact_tuple()
            changed[field] = (
                changed[field] + 1 if isinstance(changed[field], int) else f"changed-{field}"
            )
            result = MODULE.compare_tuples(exact_tuple(), changed)
            self.assertFalse(result.comparable, field)
            self.assertEqual(result.differing_fields, (field,), field)
            self.assertEqual(result.label, "INCOMPARABLE-TUPLE-DRIFT")

    def test_pass_statistics_cover_all_success_failure_and_mixed_cases(self) -> None:
        all_pass = MODULE.repeated_trial_statistics([True] * 5, 3)
        self.assertEqual(all_pass["pass_at_one"], 1.0)
        self.assertEqual(all_pass["pass_at_k"], 1.0)
        self.assertEqual(all_pass["pass_to_the_k"], 1.0)
        self.assertEqual(all_pass["variance"], 0.0)

        all_fail = MODULE.repeated_trial_statistics([False] * 5, 3)
        self.assertEqual(all_fail["pass_at_one"], 0.0)
        self.assertEqual(all_fail["pass_at_k"], 0.0)
        self.assertEqual(all_fail["pass_to_the_k"], 0.0)

        mixed = MODULE.repeated_trial_statistics([True, False, True, False, True], 2)
        self.assertEqual(mixed["trial_count"], 5)
        self.assertGreater(mixed["variance"], 0.0)
        self.assertLessEqual(mixed["confidence_interval"][0], mixed["pass_at_one"])
        self.assertGreaterEqual(mixed["confidence_interval"][1], mixed["pass_at_one"])

    def test_measured_and_blocked_records_cannot_borrow_each_others_shape(self) -> None:
        measured = {
            "disposition": "FAILED",
            "tuple": exact_tuple(),
            "statistics": {"trial_count": 5},
            "limitations": ["exact tuple only"],
        }
        MODULE.validate_benchmark_record(measured)
        measured["tuple"]["driver_sha256"] = None
        with self.assertRaises(ValueError):
            MODULE.validate_benchmark_record(measured)

        blocked = {
            "disposition": "BLOCKED-HARDWARE",
            "tuple": {field: None for field in MODULE.TUPLE_FIELDS},
            "statistics": {"trial_count": None, "pass_at_one": None},
            "limitations": ["hardware preflight did not pass"],
        }
        MODULE.validate_benchmark_record(blocked)
        blocked["statistics"]["pass_at_one"] = 0.0
        with self.assertRaises(ValueError):
            MODULE.validate_benchmark_record(blocked)

    def test_claim_lint_blocks_broad_hidden_and_unsupported_claims(self) -> None:
        allowed = {
            "kind": "observed_repeatability",
            "scope": "exact_recorded_tuple",
            "supported_by_exact_result": True,
        }
        self.assertEqual(MODULE.lint_claims([allowed]), [])
        for kind in sorted(MODULE.PROHIBITED_CLAIMS):
            claim = dict(allowed, kind=kind)
            self.assertEqual(MODULE.lint_claims([claim]), ["claim-0-unsupported"])
        self.assertEqual(
            MODULE.lint_claims([dict(allowed, supported_by_exact_result=False)]),
            ["claim-0-unsupported"],
        )


if __name__ == "__main__":
    unittest.main()
