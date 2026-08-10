from __future__ import annotations

import copy
import tempfile
import unittest
from pathlib import Path

from scripts.story_2_1_security_evidence import (
    COMPARISON_PATH,
    EXPECTED_REQUIREMENTS,
    MAP_PATH,
    PUBLIC_KEY_PATH,
    SIGNATURE_PATH,
    SIGNATURE_RECORD_PATH,
    build_comparison,
    build_map,
    build_signature_record,
    check_all,
    read_json,
    validate_comparison,
    validate_final_evidence_envelope,
    validate_map,
    validate_signature_record,
    verify_signature,
)


class Story21SecurityEvidenceTests(unittest.TestCase):
    def test_checked_comparison_signature_and_map_are_current(self) -> None:
        self.assertEqual(check_all(), [])
        self.assertEqual(read_json(COMPARISON_PATH), build_comparison())
        self.assertEqual(read_json(SIGNATURE_RECORD_PATH), build_signature_record())
        self.assertEqual(read_json(MAP_PATH), build_map())

    def test_detached_ed25519_signature_verifies(self) -> None:
        self.assertTrue(
            verify_signature(COMPARISON_PATH, PUBLIC_KEY_PATH, SIGNATURE_PATH)
        )
        record = build_signature_record()
        self.assertEqual(record["algorithm"], "Ed25519")
        self.assertFalse(record["public_key"]["private_key_retained"])
        self.assertTrue(record["public_key"]["test_evidence_only"])
        self.assertEqual(record["release_signature_claim"], "none")

    def test_comparison_or_signature_mutation_fails_verification(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            comparison = root / "comparison.json"
            public_key = root / "public.pem"
            signature = root / "comparison.sig"
            comparison.write_bytes(COMPARISON_PATH.read_bytes())
            public_key.write_bytes(PUBLIC_KEY_PATH.read_bytes())
            signature.write_bytes(SIGNATURE_PATH.read_bytes())

            mutated_comparison = bytearray(comparison.read_bytes())
            mutated_comparison[len(mutated_comparison) // 2] ^= 1
            comparison.write_bytes(bytes(mutated_comparison))
            self.assertFalse(verify_signature(comparison, public_key, signature))

            comparison.write_bytes(COMPARISON_PATH.read_bytes())
            mutated_signature = bytearray(signature.read_bytes())
            mutated_signature[0] ^= 1
            signature.write_bytes(bytes(mutated_signature))
            self.assertFalse(verify_signature(comparison, public_key, signature))

    def test_requirement_mapping_is_exact_and_does_not_overclaim(self) -> None:
        evidence_map = build_map()
        requirements = evidence_map["requirements"]
        self.assertEqual(
            [item["requirement_id"] for item in requirements],
            list(EXPECTED_REQUIREMENTS),
        )
        self.assertTrue(
            all(item["product_requirement_status"] == "not-complete" for item in requirements)
        )
        self.assertEqual(evidence_map["summary"]["product_requirements_complete"], 0)
        self.assertFalse(evidence_map["summary"]["story_gate_complete"])

    def test_retained_evidence_paths_and_hashes_are_unique(self) -> None:
        artifacts = build_map()["artifacts"]
        paths = [item["path"] for item in artifacts]
        self.assertEqual(len(paths), len(set(paths)))
        self.assertTrue(all(len(item["sha256"]) == 64 for item in artifacts))

    def test_final_signed_evidence_envelope_is_independently_clean(self) -> None:
        self.assertEqual(validate_final_evidence_envelope(), [])

    def test_comparison_rejects_raw_results_product_and_macos_claims(self) -> None:
        comparison = build_comparison()
        raw = copy.deepcopy(comparison)
        raw["raw_results_included"] = True
        claimed = copy.deepcopy(comparison)
        claimed["product_acceptance_claim"] = "pass"
        unblocked = copy.deepcopy(comparison)
        unblocked["macos_execution_status"] = "pass"
        self.assertTrue(validate_comparison(raw))
        self.assertTrue(validate_comparison(claimed))
        self.assertTrue(validate_comparison(unblocked))

    def test_signature_record_rejects_key_retention_or_release_claim(self) -> None:
        record = build_signature_record()
        retained = copy.deepcopy(record)
        retained["public_key"]["private_key_retained"] = True
        claimed = copy.deepcopy(record)
        claimed["release_signature_claim"] = "release"
        self.assertTrue(validate_signature_record(retained))
        self.assertTrue(validate_signature_record(claimed))

    def test_map_rejects_missing_requirement_completion_and_macos_substitution(self) -> None:
        evidence_map = build_map()
        missing = copy.deepcopy(evidence_map)
        missing["requirements"].pop()
        overclaimed = copy.deepcopy(evidence_map)
        overclaimed["requirements"][0]["product_requirement_status"] = "complete"
        substituted = copy.deepcopy(evidence_map)
        substituted["macos"]["evidence_substitution"] = "linux"
        self.assertTrue(validate_map(missing))
        self.assertTrue(validate_map(overclaimed))
        self.assertTrue(validate_map(substituted))


if __name__ == "__main__":
    unittest.main()
