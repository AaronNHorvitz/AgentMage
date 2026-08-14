from __future__ import annotations

import copy
import unittest

from scripts.model_activation import (
    ROOT,
    build_report,
    evaluate_activation,
)
from scripts.evidence_core import read_json_object


class ModelActivationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.policy = read_json_object(
            ROOT, "architecture/model-activation-policy.json"
        )
        cls.bindings = {name: "a" * 64 for name in cls.policy["required_binding_ids"]}
        cls.profile = {
            "profile_id": "future-admitted-profile",
            "disposition": "admitted",
            "enabled": True,
            "automatic_fallback": False,
            "supported_platforms": ["fedora-x86_64"],
            "bindings": cls.bindings,
        }

    def activate(self, profile=None, observed=None, platform="fedora-x86_64", offline=True):
        return evaluate_activation(
            self.policy,
            self.profile if profile is None else profile,
            self.bindings if observed is None else observed,
            platform,
            offline,
        )

    def test_current_baseline_exposes_no_enabled_model(self) -> None:
        report = build_report()
        self.assertEqual(report["enabled_profile_count"], 0)
        self.assertEqual(report["enabled_profiles"], [])
        self.assertTrue(all(not row["picker_visible"] for row in report["candidates"]))
        self.assertFalse(report["deterministic_provider"]["model_inference"])
        self.assertIsNone(report["deterministic_provider"]["model_id"])
        self.assertEqual(
            report["deterministic_provider"]["profile_discovery"],
            "signed-exact-admitted-only",
        )

    def test_rejected_and_blocked_profiles_are_denied(self) -> None:
        for disposition in ("rejected", "blocked"):
            profile = {**self.profile, "disposition": disposition, "enabled": False}
            result = self.activate(profile=profile)
            self.assertEqual(result["status"], "denied")
            self.assertIsNone(result["fallback_profile_id"])

    def test_complete_future_admission_can_enable_without_starting_inference(self) -> None:
        result = self.activate()
        self.assertEqual(result["status"], "enabled")
        self.assertFalse(result["inference_started"])

    def test_every_artifact_or_runtime_binding_mutation_is_denied(self) -> None:
        for name in self.policy["required_binding_ids"]:
            observed = dict(self.bindings)
            observed[name] = "b" * 64
            self.assertEqual(self.activate(observed=observed)["code"], "model.bindings.mismatch")

    def test_missing_and_malformed_bindings_are_denied(self) -> None:
        incomplete = copy.deepcopy(self.profile)
        incomplete["bindings"].pop("runtime")
        self.assertEqual(self.activate(profile=incomplete)["code"], "model.bindings.incomplete")
        malformed = copy.deepcopy(self.profile)
        malformed["bindings"]["runtime"] = "not-a-hash"
        self.assertEqual(self.activate(profile=malformed)["code"], "model.bindings.invalid")

    def test_unsupported_platform_is_denied(self) -> None:
        self.assertEqual(self.activate(platform="windows-x86_64")["code"], "model.platform.unsupported")

    def test_automatic_fallback_request_is_denied(self) -> None:
        profile = {**self.profile, "automatic_fallback": True}
        result = self.activate(profile=profile)
        self.assertEqual(result["code"], "model.fallback.prohibited")
        self.assertIsNone(result["fallback_profile_id"])

    def test_offline_unavailable_runtime_stops_visibly(self) -> None:
        result = self.activate(offline=False)
        self.assertEqual(result["code"], "model.runtime.offline_unavailable")
        self.assertFalse(result["inference_started"])

    def test_missing_profile_stops_without_substitution(self) -> None:
        result = evaluate_activation(
            self.policy, None, self.bindings, "fedora-x86_64", True
        )
        self.assertEqual(result["code"], "model.profile.missing")
        self.assertIsNone(result["fallback_profile_id"])


if __name__ == "__main__":
    unittest.main()
