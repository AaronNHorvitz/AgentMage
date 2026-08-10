from __future__ import annotations

import copy
import unittest

from scripts.fuzz_result_contract import (
    EXAMPLE_PATH,
    REPORT_PATH,
    REQUIRED_CONTRACT_FIELDS,
    build_example,
    build_report,
    check_artifacts,
    read_json,
    validate_example,
    validate_report,
)


class FuzzResultContractTests(unittest.TestCase):
    def test_checked_example_and_report_are_current(self) -> None:
        self.assertEqual(check_artifacts(), [])
        self.assertEqual(read_json(EXAMPLE_PATH), build_example())
        self.assertEqual(read_json(REPORT_PATH), build_report())

    def test_example_contains_every_required_result_dimension(self) -> None:
        report = build_report()
        self.assertEqual(report["required_contract_fields"], list(REQUIRED_CONTRACT_FIELDS))
        self.assertFalse(report["additional_properties_permitted"])
        self.assertTrue(report["non_pass_failure_required"])

    def test_non_pass_failure_minimization_and_ownership_reconcile(self) -> None:
        example = build_example()
        self.assertEqual(example["run"]["result_status"], example["failure"]["failure_class"])
        self.assertEqual(example["minimized_reproducer"]["status"], "minimized")
        self.assertTrue(example["minimized_reproducer"]["failure_signature_preserved"])
        self.assertEqual(example["ownership"]["disposition"], "fix-required")
        self.assertEqual(example["ownership"]["severity"], "high")

    def test_failure_status_hash_and_minimization_mutations_fail_closed(self) -> None:
        example = build_example()
        missing = copy.deepcopy(example)
        missing["failure"] = None
        mismatch = copy.deepcopy(example)
        mismatch["failure"]["failure_class"] = "crash"
        hash_mutation = copy.deepcopy(example)
        hash_mutation["record_sha256"] = "0" * 64
        minimization = copy.deepcopy(example)
        minimization["minimized_reproducer"]["failure_signature_preserved"] = False
        for changed in (missing, mismatch, hash_mutation, minimization):
            self.assertTrue(validate_example(changed))

    def test_sanitizer_redaction_network_product_and_macos_mutations_fail_closed(self) -> None:
        example = build_example()
        sanitizer = copy.deepcopy(example)
        sanitizer["sanitizer"]["finding_count"] = 0
        secret = copy.deepcopy(example)
        secret["redaction"]["secret_canary_values_recorded"] = True
        networked = copy.deepcopy(example)
        networked["network_used"] = True
        claimed = copy.deepcopy(example)
        claimed["product_fuzz_claim"] = "pass"
        macos = copy.deepcopy(example)
        macos["macos_execution_status"] = "pass"
        for changed in (sanitizer, secret, networked, claimed, macos):
            self.assertTrue(validate_example(changed))

    def test_report_rejects_weakened_contract_or_product_claim(self) -> None:
        report = build_report()
        extensible = copy.deepcopy(report)
        extensible["additional_properties_permitted"] = True
        claimed = copy.deepcopy(report)
        claimed["product_fuzz_claim"] = "pass"
        self.assertTrue(validate_report(extensible))
        self.assertTrue(validate_report(claimed))


if __name__ == "__main__":
    unittest.main()
