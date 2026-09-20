from __future__ import annotations

import copy
import shutil
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from scripts.context_safety_registration import REGISTRATION, ROOT, load, validate


class ContextSafetyRegistrationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.value = load(REGISTRATION)
        self.tasks = (ROOT / "TASKS.md").read_text(encoding="utf-8")
        self.prd = (ROOT / "PRD.md").read_text(encoding="utf-8")
        self.registry = load(ROOT / "requirements" / "registry.json")

    def failures(self, value: dict) -> list[str]:
        return validate(value, self.tasks, self.prd, self.registry)

    def test_current_registration_is_valid_under_owner_delegation(self) -> None:
        self.assertEqual(self.failures(self.value), [])
        self.assertEqual(self.value["governance"]["status"], "accepted")

    def test_cycle_and_guarded_trial_inversion_fail(self) -> None:
        value = copy.deepcopy(self.value)
        value["task_dependencies"]["13.1.4.2"] = ["13.4.5.1"]
        value["task_dependencies"]["13.4.5.1"].append("13.1.4.2")
        value["task_dependencies"]["13.4.5.1"].sort()
        self.assertTrue(any("cycle" in item for item in self.failures(value)))

        value = copy.deepcopy(self.value)
        value["task_dependencies"]["13.4.5.1"].append("13.3.4.3")
        value["task_dependencies"]["13.4.5.1"].sort()
        failures = self.failures(value)
        self.assertTrue(any("entry" in item or "cycle" in item for item in failures))

        value = copy.deepcopy(self.value)
        value["task_dependencies"]["13.4.5.1"] = [
            "13.1.4",
            "13.1.5",
            "13.3.4.1",
        ]
        self.assertTrue(any("guarded native trial" in item for item in self.failures(value)))

    def test_case_owner_release_and_migration_refusal_fail_closed(self) -> None:
        value = copy.deepcopy(self.value)
        del value["acceptance_cases"]["CTX-FIT"]
        self.assertTrue(any("case set" in item for item in self.failures(value)))

        value = copy.deepcopy(self.value)
        value["acceptance_cases"]["CTX-FIT"]["owner"] = "999.9.9"
        self.assertTrue(any("owner" in item for item in self.failures(value)))

        value = copy.deepcopy(self.value)
        value["contract_migrations"][0]["missing_reason_code"] = "unknown.reason"
        self.assertTrue(any("refusal" in item for item in self.failures(value)))

    def test_governance_requires_exact_owner_delegation_and_corrected_identity(self) -> None:
        value = copy.deepcopy(self.value)
        value["governance"]["status"] = "blocked-owner-direction"
        self.assertTrue(any("owner-delegated" in item for item in self.failures(value)))

        value = copy.deepcopy(self.value)
        value["governance"]["resolved_decision_files"].pop()
        self.assertTrue(any("filenames" in item for item in self.failures(value)))

        value = copy.deepcopy(self.value)
        value["governance"]["decision_source_sha256"] = "0" * 64
        self.assertTrue(any("source hash" in item for item in self.failures(value)))

    def test_duplicate_decision_identity_and_mismatched_title_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            shutil.copytree(ROOT / "docs" / "decisions", root / "docs" / "decisions")
            duplicate = root / "docs" / "decisions" / "0053-shadow.md"
            duplicate.write_text("# Decision 0054: Incorrect Identity\n", encoding="utf-8")
            with patch("scripts.context_safety_registration.ROOT", root):
                failures = self.failures(self.value)
            self.assertTrue(any("duplicate accepted decision identity 0053" in item for item in failures))
            self.assertTrue(any("title/filename identity mismatch" in item for item in failures))


if __name__ == "__main__":
    unittest.main()
