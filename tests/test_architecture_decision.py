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
