from __future__ import annotations

import copy
import unittest

from scripts.architecture_decision import load_matrix, validate_matrix


class ArchitectureDecisionTests(unittest.TestCase):
    def setUp(self) -> None:
        self.matrix = load_matrix()

    def test_canonical_matrix_passes(self) -> None:
        self.assertEqual(validate_matrix(self.matrix), [])

    def test_each_required_component_language_is_enforced(self) -> None:
        for component in self.matrix["product_components"]:
            with self.subTest(component=component["id"]):
                mutated = copy.deepcopy(self.matrix)
                selected = next(
                    item
                    for item in mutated["product_components"]
                    if item["id"] == component["id"]
                )
                selected["language"] = "python"
                self.assertTrue(validate_matrix(mutated))

    def test_python_cannot_become_an_end_user_dependency(self) -> None:
        mutated = copy.deepcopy(self.matrix)
        python = next(
            item for item in mutated["non_product_tooling"] if item["language"] == "python"
        )
        python["end_user_dependency"] = True
        self.assertIn(
            "Python must remain non-product tooling with no end-user dependency",
            validate_matrix(mutated),
        )

    def test_extension_cannot_gain_product_authority(self) -> None:
        for authority in self.matrix["vscode_contract"]["extension_prohibited_authority"]:
            with self.subTest(authority=authority):
                mutated = copy.deepcopy(self.matrix)
                mutated["vscode_contract"]["extension_prohibited_authority"].remove(authority)
                self.assertTrue(validate_matrix(mutated))

    def test_ambient_workspace_read_prohibition_is_retained(self) -> None:
        self.assertIn(
            "ambient-workspace-read",
            self.matrix["vscode_contract"]["extension_prohibited_authority"],
        )
        mutated = copy.deepcopy(self.matrix)
        mutated["vscode_contract"]["reference_resolution_contract"][
            "ambient_workspace_read_prohibited"
        ] = False
        self.assertIn(
            "ambient extension workspace reads must remain prohibited",
            validate_matrix(mutated),
        )

    def test_generic_read_authority_cannot_return(self) -> None:
        for authority in (
            "workspace-read",
            "workspace-enumeration",
            "workspace-index",
            "filesystem-read",
        ):
            with self.subTest(authority=authority):
                mutated = copy.deepcopy(self.matrix)
                mutated["vscode_contract"]["extension_authority"].append(authority)
                failures = validate_matrix(mutated)
                self.assertTrue(
                    any("cannot regain ambient read authority" in item for item in failures)
                )

    def test_current_request_reference_authority_cannot_be_dropped(self) -> None:
        mutated = copy.deepcopy(self.matrix)
        mutated["vscode_contract"]["extension_authority"].remove(
            "current-request-reference-resolution"
        )
        self.assertIn(
            "VS Code extension authority set is incomplete or changed",
            validate_matrix(mutated),
        )

    def test_reference_resolution_contract_is_required(self) -> None:
        mutated = copy.deepcopy(self.matrix)
        del mutated["vscode_contract"]["reference_resolution_contract"]
        self.assertIn(
            "reference_resolution_contract must be an object",
            validate_matrix(mutated),
        )

    def test_reference_scope_cannot_widen_past_the_current_request(self) -> None:
        for field, value, expected in (
            ("permitted_scope", "workspace", "only current-request references may be resolvable"),
            (
                "delivery_surface",
                "language-model-chat-provider",
                "references must be delivered to the AgentMage Chat Participant",
            ),
            (
                "requires_explicit_user_delivery",
                False,
                "reference resolution requires explicit delivery to the current request",
            ),
            (
                "requires_stable_api",
                False,
                "reference resolution must use stable VS Code APIs",
            ),
            (
                "resolved_byte_authority",
                "vscode-extension",
                "the Rust host must own every resolved reference byte",
            ),
            (
                "unresolved_reference_disposition",
                "silently_dropped",
                "an unresolved reference must remain visibly unavailable",
            ),
        ):
            with self.subTest(field=field):
                mutated = copy.deepcopy(self.matrix)
                mutated["vscode_contract"]["reference_resolution_contract"][field] = value
                self.assertIn(expected, validate_matrix(mutated))

    def test_each_prohibited_resolution_behavior_is_enforced(self) -> None:
        contract = self.matrix["vscode_contract"]["reference_resolution_contract"]
        for behavior in contract["prohibited_resolution_behavior"]:
            with self.subTest(behavior=behavior):
                mutated = copy.deepcopy(self.matrix)
                mutated["vscode_contract"]["reference_resolution_contract"][
                    "prohibited_resolution_behavior"
                ].remove(behavior)
                failures = validate_matrix(mutated)
                self.assertTrue(
                    any("missing prohibited behavior" in item for item in failures)
                )

    def test_granted_authority_cannot_also_be_prohibited(self) -> None:
        mutated = copy.deepcopy(self.matrix)
        mutated["vscode_contract"]["extension_prohibited_authority"].append(
            "current-request-reference-resolution"
        )
        failures = validate_matrix(mutated)
        self.assertTrue(
            any("both granted and prohibited" in item for item in failures)
        )

    def test_proposed_vscode_api_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.matrix)
        mutated["vscode_contract"]["api_channel"] = "proposed"
        mutated["vscode_contract"]["proposed_api_allowed"] = True
        self.assertTrue(validate_matrix(mutated))

    def test_macos_cannot_be_recorded_as_implemented_or_verified(self) -> None:
        mutated = copy.deepcopy(self.matrix)
        macos = next(
            item
            for item in mutated["platform_targets"]
            if item["id"] == "macos-arm64"
        )
        macos["implementation_status"] = "implemented"
        macos["verification_status"] = "pass"
        self.assertTrue(validate_matrix(mutated))

    def test_shipped_boolean_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.matrix)
        mutated["product_components"][0]["shipped"] = True
        self.assertTrue(validate_matrix(mutated))

    def test_windows_target_and_component_are_required(self) -> None:
        for collection, record_id in (
            ("product_components", "windows-platform-adapter"),
            ("platform_targets", "windows-x86_64"),
        ):
            with self.subTest(collection=collection):
                mutated = copy.deepcopy(self.matrix)
                mutated[collection] = [
                    item for item in mutated[collection] if item["id"] != record_id
                ]
                self.assertTrue(validate_matrix(mutated))

    def test_status_reference_is_required(self) -> None:
        mutated = copy.deepcopy(self.matrix)
        mutated["product_components"][0]["status_ref"] = "stale"
        self.assertTrue(validate_matrix(mutated))

    def test_end_user_toolchain_assumptions_are_rejected(self) -> None:
        mutated = copy.deepcopy(self.matrix)
        mutated["build_contract"]["end_user_ambient_toolchains"] = ["cargo"]
        self.assertIn(
            "end users must not require ambient development toolchains",
            validate_matrix(mutated),
        )


if __name__ == "__main__":
    unittest.main()
