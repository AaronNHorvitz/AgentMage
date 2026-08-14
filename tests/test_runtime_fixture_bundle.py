"""Mutation and reproducibility tests for Task 12.1.2.2 runtime fixtures."""

import copy
import json
import unittest

from scripts import runtime_fixture_bundle as fixtures


class RuntimeFixtureBundleTests(unittest.TestCase):
    def expected(self) -> dict:
        return fixtures.expected_fixtures()

    def test_current_files_are_canonical_and_semantically_closed(self) -> None:
        self.assertEqual(fixtures.validate_current(), [])
        self.assertEqual(fixtures.semantic_failures(self.expected()), [])

    def test_generation_is_deterministic(self) -> None:
        first = self.expected()
        second = self.expected()
        self.assertEqual(first, second)
        for family, path in fixtures.FIXTURE_PATHS.items():
            self.assertEqual(json.loads(path.read_text()), first[family])

    def test_planning_mutations_fail(self) -> None:
        for mutation in ("case", "second-running", "disposition", "private", "network"):
            changed = copy.deepcopy(self.expected())
            if mutation == "case":
                changed["planning"]["cases"][0]["case_id"] = "PLN-99"
            elif mutation == "second-running":
                changed["planning"]["cases"][1]["steps"][1]["state"] = "running"
            elif mutation == "disposition":
                changed["planning"]["cases"][2]["expected"]["admitted"] = True
            elif mutation == "private":
                changed["planning"]["private_user_data_used"] = True
            else:
                changed["planning"]["external_network_used"] = True
            self.assertTrue(fixtures.semantic_failures(changed))

    def test_reasoning_authority_evidence_and_private_thought_mutations_fail(self) -> None:
        for mutation in ("authority", "verification", "private-thought", "mode-authority", "mode-evidence"):
            changed = copy.deepcopy(self.expected())
            if mutation == "authority":
                changed["reasoning"]["cases"][0]["authority"] = "model-authorized"
            elif mutation == "verification":
                changed["reasoning"]["cases"][1]["independent_verification_required"] = False
            elif mutation == "private-thought":
                changed["reasoning"]["cases"][0]["private_chain_of_thought_retained"] = True
            elif mutation == "mode-authority":
                changed["reasoning"]["mode_changes_authority"] = True
            else:
                changed["reasoning"]["mode_changes_evidence_standard"] = True
            self.assertTrue(fixtures.semantic_failures(changed))

    def test_completion_case_and_disposition_mutations_fail(self) -> None:
        for mutation in ("case", "unsupported-success", "valid-failure", "private", "network"):
            changed = copy.deepcopy(self.expected())
            if mutation == "case":
                changed["completion"]["cases"][4]["case_id"] = "CMP-99"
            elif mutation == "unsupported-success":
                changed["completion"]["cases"][3]["expected"]["verified"] = True
            elif mutation == "valid-failure":
                changed["completion"]["cases"][0]["expected"]["verified"] = False
            elif mutation == "private":
                changed["completion"]["private_user_data_used"] = True
            else:
                changed["completion"]["external_network_used"] = True
            self.assertTrue(fixtures.semantic_failures(changed))


if __name__ == "__main__":
    unittest.main()
