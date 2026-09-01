from __future__ import annotations

import copy
import unittest
from pathlib import Path

from scripts.vscode_api_surfaces import (
    EXPECTED_CHANNELS,
    GUARANTEED_SUPPORTED_PATH_SURFACES,
    load_surfaces,
    validate_surfaces,
)


class VscodeApiSurfaceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.record = load_surfaces()

    def _surface(self, record: dict, surface_id: str) -> dict:
        return next(item for item in record["surfaces"] if item["id"] == surface_id)

    def test_canonical_record_passes(self) -> None:
        self.assertEqual(validate_surfaces(self.record), [])

    def test_all_four_channels_are_recorded(self) -> None:
        self.assertEqual([item["id"] for item in self.record["channels"]], EXPECTED_CHANNELS)

    def test_only_stable_may_carry_the_supported_path(self) -> None:
        for channel in self.record["channels"]:
            with self.subTest(channel=channel["id"]):
                mutated = copy.deepcopy(self.record)
                target = next(
                    item for item in mutated["channels"] if item["id"] == channel["id"]
                )
                target["permitted_for_supported_path"] = channel["id"] != "stable"
                self.assertTrue(validate_surfaces(mutated))

    def test_a_guarantee_cannot_rest_on_an_experimental_channel(self) -> None:
        for surface_id in sorted(GUARANTEED_SUPPORTED_PATH_SURFACES):
            for channel in ("preview", "proposed", "private"):
                with self.subTest(surface=surface_id, channel=channel):
                    mutated = copy.deepcopy(self.record)
                    self._surface(mutated, surface_id)["channel"] = channel
                    failures = validate_surfaces(mutated)
                    self.assertIn(
                        f"surface {surface_id} cannot be guaranteed on a non-stable channel",
                        failures,
                    )
                    self.assertIn(
                        f"surface {surface_id} cannot carry the supported path on its channel",
                        failures,
                    )

    def test_the_guaranteed_participant_path_cannot_shrink_or_widen(self) -> None:
        shrunk = copy.deepcopy(self.record)
        shrunk["surfaces"] = [
            item
            for item in shrunk["surfaces"]
            if item["id"] != "chat-request-references"
        ]
        self.assertIn(
            "the guaranteed stable participant path is incomplete or widened",
            validate_surfaces(shrunk),
        )
        widened = copy.deepcopy(self.record)
        widened["surfaces"].append(
            {
                "id": "workspace-file-system",
                "channel": "stable",
                "support": "guaranteed",
                "supported_path_member": True,
                "degradation": "none",
                "absence_disclosed": True,
            }
        )
        self.assertIn(
            "the guaranteed stable participant path is incomplete or widened",
            validate_surfaces(widened),
        )

    def test_label_provider_stays_best_effort_and_off_the_supported_path(self) -> None:
        promoted = copy.deepcopy(self.record)
        label = self._surface(promoted, "label-provider")
        label["support"] = "guaranteed"
        failures = validate_surfaces(promoted)
        self.assertIn(
            "the best-effort compatibility set is incomplete or widened", failures
        )
        self.assertIn(
            "surface label-provider cannot be guaranteed on a non-stable channel", failures
        )

        on_path = copy.deepcopy(self.record)
        self._surface(on_path, "label-provider")["supported_path_member"] = True
        self.assertIn(
            "best-effort surface label-provider must not be on the supported path",
            validate_surfaces(on_path),
        )

    def test_best_effort_compatibility_must_declare_its_degradation(self) -> None:
        for degradation in ("none", ""):
            with self.subTest(degradation=degradation):
                mutated = copy.deepcopy(self.record)
                self._surface(mutated, "label-provider")["degradation"] = degradation
                self.assertIn(
                    "best-effort surface label-provider must declare its degradation",
                    validate_surfaces(mutated),
                )

    def test_every_surface_must_disclose_its_absence(self) -> None:
        for surface in self.record["surfaces"]:
            with self.subTest(surface=surface["id"]):
                mutated = copy.deepcopy(self.record)
                self._surface(mutated, surface["id"])["absence_disclosed"] = False
                self.assertIn(
                    f"surface {surface['id']} must disclose its absence",
                    validate_surfaces(mutated),
                )

    def test_experiment_controls_are_a_closed_complete_set(self) -> None:
        for mutation in (
            ["feature-flag", "version-guard", "fallback"],
            ["feature-flag", "version-guard", "fallback", "separate-non-support-claim", "extra"],
            [],
        ):
            with self.subTest(mutation=tuple(mutation)):
                mutated = copy.deepcopy(self.record)
                mutated["experiment_controls"] = mutation
                self.assertTrue(validate_surfaces(mutated))

    def test_native_compatibility_limits_cannot_be_hidden_or_widened(self) -> None:
        for key, value in (
            ("stable_input_parts", ["text", "image"]),
            ("external_tool_proposals", "supported"),
            ("usage_disclosure", "exact-token-counts"),
            ("persistent_lifecycle", "native-chat"),
            ("transition_command", "missing"),
            ("provider_claim", "agent-host"),
        ):
            with self.subTest(key=key):
                mutated = copy.deepcopy(self.record)
                mutated["native_compatibility"][key] = value
                self.assertIn(
                    "native_compatibility must retain the exact stable text-only capability and limitation matrix",
                    validate_surfaces(mutated),
                )

    def test_production_provider_uses_the_closed_stable_compatibility_projection(self) -> None:
        root = Path(__file__).resolve().parents[1]
        source = (root / "shells/vscode/src/extension.ts").read_text(encoding="utf-8")
        for marker in (
            "vscode.chat.createChatParticipant",
            "vscode.lm.registerLanguageModelChatProvider",
            "projectNativeProviderRequest(",
            "nativeProviderRouteDisclosure(",
            "nativeProviderUsageDisclosure(",
            "vscode.LanguageModelDataPart.json",
            "capabilities: { imageInput: false, toolCalling: false }",
            "agentmage.openVerifiedChat",
        ):
            self.assertIn(marker, source)
        self.assertNotIn("lastUserParts", source)

    def test_surfaces_cannot_be_widened_by_unknown_keys(self) -> None:
        mutated = copy.deepcopy(self.record)
        self._surface(mutated, "chat-participant")["ambient_workspace_scan"] = True
        self.assertIn(
            "surface chat-participant has unknown keys: ambient_workspace_scan",
            validate_surfaces(mutated),
        )

    def test_decision_and_supported_path_identities_are_pinned(self) -> None:
        for key, value in (
            ("schema_version", 1),
            ("decision_id", "ADR-0004"),
            ("status", "proposed"),
            ("supported_path", "language-model-chat-provider"),
        ):
            with self.subTest(key=key):
                mutated = copy.deepcopy(self.record)
                mutated[key] = value
                self.assertTrue(validate_surfaces(mutated))


if __name__ == "__main__":
    unittest.main()
