from __future__ import annotations

import copy
import json
import unittest

from scripts.contract_evidence_index import (
    GROUPS,
    INDEX_PATH,
    PROTOCOLS,
    RAW_MARKERS,
    RAW_PATH,
    SECURITY_MAP_PATH,
    expected_index,
    expected_security_map,
    validate_index,
    validate_raw,
    validate_security_map,
)


class ContractEvidenceIndexTests(unittest.TestCase):
    def test_canonical_raw_map_and_index_pass(self) -> None:
        self.assertEqual(validate_raw(RAW_PATH.read_text(encoding="utf-8")), [])
        self.assertEqual(
            validate_security_map(json.loads(SECURITY_MAP_PATH.read_text(encoding="utf-8"))),
            [],
        )
        self.assertEqual(validate_index(json.loads(INDEX_PATH.read_text(encoding="utf-8"))), [])

    def test_raw_results_require_every_success_marker(self) -> None:
        raw = RAW_PATH.read_text(encoding="utf-8")
        for marker in RAW_MARKERS:
            with self.subTest(marker=marker):
                changed = raw.replace(marker, "removed-marker")
                self.assertIn(f"raw contract results missing marker: {marker}", validate_raw(changed))

    def test_raw_results_reject_failure_markers(self) -> None:
        raw = RAW_PATH.read_text(encoding="utf-8")
        for marker in ("Traceback (most recent call last)", "FAILED (", "not ok "):
            with self.subTest(marker=marker):
                self.assertIn(
                    f"raw contract results contain failure marker: {marker}",
                    validate_raw(raw + marker),
                )

    def test_security_mapping_has_exact_protocol_order(self) -> None:
        mapping = expected_security_map()
        self.assertEqual([item["protocol_id"] for item in mapping["protocols"]], PROTOCOLS)
        changed = copy.deepcopy(mapping)
        changed["protocols"].reverse()
        self.assertTrue(validate_security_map(changed))

    def test_security_protocol_cannot_be_marked_complete(self) -> None:
        for protocol_id in PROTOCOLS:
            with self.subTest(protocol=protocol_id):
                changed = expected_security_map()
                entry = next(
                    item for item in changed["protocols"] if item["protocol_id"] == protocol_id
                )
                entry["protocol_complete"] = True
                entry["status"] = "complete"
                self.assertTrue(validate_security_map(changed))

    def test_security_protocol_requires_remaining_work(self) -> None:
        changed = expected_security_map()
        changed["protocols"][0]["remaining"] = ""
        self.assertTrue(validate_security_map(changed))

    def test_security_evidence_hash_mutation_is_stale(self) -> None:
        changed = expected_security_map()
        changed["protocols"][0]["evidence"][0]["sha256"] = "0" * 64
        self.assertTrue(validate_security_map(changed))

    def test_index_retains_all_required_group_classes(self) -> None:
        index = expected_index()
        self.assertEqual([item["id"] for item in index["groups"]], list(GROUPS))
        changed = copy.deepcopy(index)
        changed["groups"].pop()
        self.assertTrue(validate_index(changed))

    def test_index_refuses_artifact_removal_or_hash_mutation(self) -> None:
        removed = expected_index()
        removed["groups"][0]["artifacts"].pop()
        self.assertTrue(validate_index(removed))
        changed = expected_index()
        changed["groups"][0]["artifacts"][0]["sha256"] = "0" * 64
        self.assertTrue(validate_index(changed))

    def test_raw_result_is_hash_bound_and_synthetic(self) -> None:
        index = expected_index()
        self.assertEqual(index["raw_result"]["data_class"], "synthetic-local-contract-tests-only")
        changed = copy.deepcopy(index)
        changed["raw_result"]["artifact"]["sha256"] = "0" * 64
        self.assertTrue(validate_index(changed))

    def test_product_truth_cannot_claim_protocol_platform_review_support_or_release(self) -> None:
        for key in (
            "protocol_complete",
            "native_platform_evidence",
            "external_review_complete",
            "platform_support_claimed",
            "release_readiness",
        ):
            with self.subTest(key=key):
                mapping = expected_security_map()
                mapping["product_truth"][key] = True
                self.assertTrue(validate_security_map(mapping))
                index = expected_index()
                index["product_truth"][key] = True
                self.assertTrue(validate_index(index))

    def test_synthetic_data_and_local_index_truth_remain_positive(self) -> None:
        for factory, validator in (
            (expected_security_map, validate_security_map),
            (expected_index, validate_index),
        ):
            for key in ("synthetic_data_only", "local_contract_evidence_retained"):
                with self.subTest(key=key):
                    changed = factory()
                    changed["product_truth"][key] = False
                    self.assertTrue(validator(changed))


if __name__ == "__main__":
    unittest.main()
