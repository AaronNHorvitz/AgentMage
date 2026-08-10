from __future__ import annotations

import copy
import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from scripts.document_fixture_generator import materialize_specs as materialize_document_specs
from scripts.expected_output_manifests import (
    EXPECTED_PROHIBITED_SIDE_EFFECTS,
    EXPECTED_STATES,
    PROFILE_PATH,
    ZERO_HASH,
    build_manifest_records,
    build_report,
    canonical_json,
    check_report,
    generate,
    manifest_set_identity,
    materialize_specs,
    read_json,
    sha256_bytes,
    source_range,
    validate_manifest_records,
    validate_profile,
    validate_records,
    validate_report,
)


class ExpectedOutputManifestTests(unittest.TestCase):
    def setUp(self) -> None:
        self.profile = read_json(PROFILE_PATH)
        self.records = build_manifest_records(self.profile)

    def test_checked_in_profile_report_and_records_are_current(self) -> None:
        self.assertEqual(validate_profile(self.profile), [])
        self.assertEqual(validate_manifest_records(self.profile), [])
        self.assertEqual(check_report(), [])

    def test_all_four_evidence_states_have_one_expected_manifest(self) -> None:
        self.assertEqual(
            tuple(record["expected_output"]["evidence"]["state"] for record in self.records),
            EXPECTED_STATES,
        )
        specs = materialize_specs(self.profile)
        self.assertEqual(len(specs), 4)
        self.assertEqual(
            {json.loads(item.content)["manifest_id"] for item in specs.values()},
            {record["manifest_id"] for record in self.records},
        )

    def test_source_ranges_resolve_to_exact_fixture_bytes(self) -> None:
        document_profile = read_json(Path("fixtures/document-fixture-profile.json"))
        documents = materialize_document_specs(document_profile)
        for case, record in zip(self.profile["cases"], self.records):
            citations = record["expected_output"]["evidence"]["citations"]
            if case["evidence_state"] == "Unknown/Blocked":
                self.assertEqual(citations, [])
                continue
            citation = citations[0]
            content = documents[citation["path"]].content
            expected = source_range(content, case["source"]["line_start"], case["source"]["line_end"])
            self.assertEqual(citation["range"], expected)
            self.assertEqual(citation["content_sha256"], sha256_bytes(content))
            excerpt = content[expected["byte_start"] : expected["byte_end"]]
            self.assertEqual(hashlib.sha256(excerpt).hexdigest(), expected["range_sha256"])

    def test_receipts_form_a_deterministic_hash_chain(self) -> None:
        previous = ZERO_HASH
        for record in self.records:
            receipt = copy.deepcopy(record["expected_receipt"])
            recorded_hash = receipt.pop("receipt_sha256")
            self.assertEqual(receipt["previous_receipt_sha256"], previous)
            self.assertEqual(sha256_bytes(canonical_json(receipt)), recorded_hash)
            self.assertEqual(
                receipt["output_sha256"],
                sha256_bytes(canonical_json(record["expected_output"])),
            )
            previous = recorded_hash

    def test_state_specific_provenance_is_complete(self) -> None:
        by_state = {
            record["expected_output"]["evidence"]["state"]: record["expected_output"]["evidence"]
            for record in self.records
        }
        self.assertTrue(by_state["Observed"]["citations"])
        self.assertIsNone(by_state["Observed"]["derivation"])
        self.assertEqual(by_state["Derived"]["derivation"]["method_id"], "count-json-array-v1")
        self.assertEqual(by_state["Inferred"]["inference"]["model_id"], "fake-model-v1")
        self.assertEqual(by_state["Inferred"]["inference"]["runtime_id"], "fake-inference-runtime-v1")
        self.assertEqual(by_state["Unknown/Blocked"]["blocked_reason"], "out-of-scope")

    def test_every_manifest_declares_closed_zero_side_effect_expectations(self) -> None:
        for record in self.records:
            self.assertEqual(record["expected_side_effects"], [])
            prohibited = record["prohibited_side_effects"]
            self.assertEqual(
                tuple(item["id"] for item in prohibited),
                EXPECTED_PROHIBITED_SIDE_EFFECTS,
            )
            self.assertTrue(all(item["expected_count"] == 0 for item in prohibited))

    def test_generation_is_repeatable_non_executable_and_refuses_overwrite(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            first = Path(temporary) / "first"
            second = Path(temporary) / "second"
            self.assertEqual(generate(self.profile, first), generate(self.profile, second))
            first_files = {
                path.relative_to(first): path.read_bytes()
                for path in first.rglob("*")
                if path.is_file()
            }
            second_files = {
                path.relative_to(second): path.read_bytes()
                for path in second.rglob("*")
                if path.is_file()
            }
            self.assertEqual(first_files, second_files)
            self.assertTrue(
                all(path.stat().st_mode & 0o111 == 0 for path in first.rglob("*") if path.is_file())
            )
            with self.assertRaises(FileExistsError):
                generate(self.profile, first)

    def test_manifest_set_identity_changes_on_manifest_mutation(self) -> None:
        original = materialize_specs(self.profile)
        mutated = copy.deepcopy(original)
        path = next(iter(mutated))
        manifest = mutated[path]
        mutated[path] = type(manifest)(manifest.case_id, manifest.content + b" ")
        self.assertNotEqual(manifest_set_identity(original), manifest_set_identity(mutated))

    def test_receipt_corruption_is_rejected(self) -> None:
        corrupted = copy.deepcopy(self.records)
        corrupted[1]["expected_receipt"]["output_sha256"] = "f" * 64
        failures = validate_records(self.profile, corrupted)
        self.assertTrue(any("receipt hash mismatch" in failure for failure in failures))
        self.assertTrue(any("output hash mismatch" in failure for failure in failures))

    def test_source_hash_or_range_corruption_is_rejected(self) -> None:
        corrupted = copy.deepcopy(self.records)
        citation = corrupted[0]["expected_output"]["evidence"]["citations"][0]
        citation["content_sha256"] = "f" * 64
        failures = validate_records(self.profile, corrupted)
        self.assertTrue(any("source binding mismatch" in failure for failure in failures))

    def test_weakened_side_effect_closure_and_authority_claim_are_rejected(self) -> None:
        weakened = copy.deepcopy(self.profile)
        weakened["prohibited_side_effects"].remove("network-access")
        claimed = copy.deepcopy(self.profile)
        claimed["authority_claim"] = "synthetic-grant"
        self.assertTrue(validate_profile(weakened))
        self.assertTrue(validate_profile(claimed))

    def test_invalid_source_range_and_unknown_source_are_rejected(self) -> None:
        invalid_range = copy.deepcopy(self.profile)
        invalid_range["cases"][0]["source"]["line_start"] = 0
        unknown_source = copy.deepcopy(self.profile)
        unknown_source["cases"][0]["source"]["path"] = "documents/text/missing.txt"
        self.assertTrue(validate_profile(invalid_range))
        with self.assertRaises(ValueError):
            build_manifest_records(unknown_source)

    def test_report_contains_hashes_not_manifest_payloads_and_reserves_goldens(self) -> None:
        report = build_report()
        self.assertFalse(report["manifest_preview"]["persisted"])
        self.assertEqual(report["manifest_preview"]["manifest_count"], 4)
        self.assertNotIn("statement", json.dumps(report, sort_keys=True))
        mutated = copy.deepcopy(report)
        mutated["versioned_golden_manifest_status"] = "complete"
        self.assertTrue(validate_report(mutated))


if __name__ == "__main__":
    unittest.main()
