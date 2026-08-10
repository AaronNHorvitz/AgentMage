from __future__ import annotations

import copy
import json
import re
import tempfile
import unittest
from pathlib import Path

from scripts.adversarial_fixture_generator import (
    EXPECTED_BYPASS_CASES,
    EXPECTED_CATEGORIES,
    EXPECTED_INJECTION_INTENTS,
    EXPECTED_MODEL_CASES,
    EXPECTED_REPLAY_CASES,
    PROFILE_PATH,
    build_report,
    check_report,
    generate,
    materialize_specs,
    read_json,
    synthetic_canary,
    validate_profile,
    validate_report,
)


class AdversarialFixtureGeneratorTests(unittest.TestCase):
    def setUp(self) -> None:
        self.profile = read_json(PROFILE_PATH)
        self.specs = materialize_specs(self.profile)

    def test_checked_in_profile_and_report_are_current(self) -> None:
        self.assertEqual(validate_profile(self.profile), [])
        self.assertEqual(check_report(), [])

    def test_all_six_fixture_categories_are_nonempty(self) -> None:
        self.assertEqual({item.category for item in self.specs.values()}, set(EXPECTED_CATEGORIES))
        for category in EXPECTED_CATEGORIES:
            self.assertTrue(any(item.category == category for item in self.specs.values()))

    def test_canaries_are_deterministic_unique_and_obviously_synthetic(self) -> None:
        canaries = [
            synthetic_canary(self.profile["seed"], location)
            for location in self.profile["canary_locations"]
        ]
        self.assertEqual(len(canaries), len(set(canaries)))
        self.assertTrue(all(re.fullmatch(r"AM_SYNTHETIC_CANARY_[0-9A-F]{24}", item) for item in canaries))

    def test_report_retains_only_canary_hashes_and_counts(self) -> None:
        report = build_report()
        serialized = json.dumps(report, sort_keys=True)
        self.assertNotIn("AM_SYNTHETIC_CANARY_", serialized)
        self.assertFalse(report["raw_canary_values_retained"])
        self.assertEqual(report["preview"]["canary_count"], 4)

    def test_prompt_injection_intents_are_complete_and_inert(self) -> None:
        document = json.loads(
            self.specs["adversarial/prompt-injection/requests.json"].content
        )
        self.assertEqual(
            tuple(item["id"] for item in document["records"]),
            EXPECTED_INJECTION_INTENTS,
        )
        self.assertTrue(
            all(item["trust"] == "untrusted-fixture-content" for item in document["records"])
        )

    def test_conflict_preserves_trusted_and_untrusted_channels(self) -> None:
        document = json.loads(
            self.specs["adversarial/conflicts/instructions.json"].content
        )
        self.assertIn("trusted_user_intent", document)
        self.assertIn("untrusted_workspace_text", document)
        self.assertEqual(document["expected"]["side_effects"], [])

    def test_malformed_model_call_case_closure(self) -> None:
        paths = {
            Path(path).stem
            for path in self.specs
            if path.startswith("adversarial/model-calls/")
        }
        self.assertEqual(paths, set(EXPECTED_MODEL_CASES))
        truncated = self.specs["adversarial/model-calls/truncated-json.json"].content
        with self.assertRaises(json.JSONDecodeError):
            json.loads(truncated)

    def test_grant_replay_records_are_synthetic_and_unminted(self) -> None:
        document = json.loads(
            self.specs["adversarial/grant-replay/cases.json"].content
        )
        self.assertEqual(tuple(item["case"] for item in document["records"]), EXPECTED_REPLAY_CASES)
        self.assertTrue(all(item["grant_minted"] is False for item in document["records"]))

    def test_approval_bypass_records_never_record_approval(self) -> None:
        document = json.loads(
            self.specs["adversarial/approval-bypass/cases.json"].content
        )
        self.assertEqual(tuple(item["case"] for item in document["records"]), EXPECTED_BYPASS_CASES)
        self.assertTrue(all(item["approval_recorded"] is False for item in document["records"]))

    def test_fixture_bytes_do_not_match_common_real_credential_shapes(self) -> None:
        combined = b"\n".join(item.content for item in self.specs.values()).decode(
            "utf-8", errors="ignore"
        )
        patterns = (
            r"AKIA[0-9A-Z]{16}",
            r"gh[pousr]_[A-Za-z0-9]{20,}",
            r"-----BEGIN [A-Z ]*PRIVATE KEY-----",
            r"eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+",
        )
        self.assertFalse(any(re.search(pattern, combined) for pattern in patterns))

    def test_generation_is_repeatable_and_refuses_overwrite(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            first = Path(temporary) / "first"
            second = Path(temporary) / "second"
            self.assertEqual(generate(self.profile, first), generate(self.profile, second))
            with self.assertRaises(FileExistsError):
                generate(self.profile, first)
            self.assertTrue(
                all(path.stat().st_mode & 0o111 == 0 for path in first.rglob("*") if path.is_file())
            )

    def test_authority_claim_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.profile)
        mutated["authority_claim"] = "fixture-grant"
        self.assertTrue(validate_profile(mutated))

    def test_raw_canary_in_report_is_rejected(self) -> None:
        mutated = build_report()
        mutated["raw_canary_values_retained"] = True
        mutated["raw_canary"] = synthetic_canary(self.profile["seed"], "plain-text")
        self.assertTrue(validate_report(mutated))


if __name__ == "__main__":
    unittest.main()
