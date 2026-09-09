from __future__ import annotations

import copy
import unittest

from scripts.context_safety_registration import REGISTRATION, ROOT, load, validate


class ContextSafetyRegistrationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.value = load(REGISTRATION)
        self.tasks = (ROOT / "TASKS.md").read_text(encoding="utf-8")
        self.prd = (ROOT / "PRD.md").read_text(encoding="utf-8")
        self.registry = load(ROOT / "requirements" / "registry.json")

    def failures(self, value: dict) -> list[str]:
        return validate(value, self.tasks, self.prd, self.registry)

    def test_current_registration_is_valid_and_truthfully_blocked(self) -> None:
        self.assertEqual(self.failures(self.value), [])
        self.assertEqual(self.value["governance"]["status"], "blocked-owner-direction")

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

    def test_governance_collision_cannot_be_silently_resolved(self) -> None:
        value = copy.deepcopy(self.value)
        value["governance"]["status"] = "accepted"
        self.assertTrue(any("owner-direction" in item for item in self.failures(value)))

        value = copy.deepcopy(self.value)
        value["governance"]["colliding_decision_files"].pop()
        self.assertTrue(any("filenames" in item for item in self.failures(value)))


if __name__ == "__main__":
    unittest.main()
