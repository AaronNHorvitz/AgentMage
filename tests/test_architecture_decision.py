from __future__ import annotations

import copy
import unittest

from scripts.architecture_decision import (
    REQUEST_REFERENCE_EXACT_VALUES,
    REQUEST_REFERENCE_KINDS,
    REQUEST_REFERENCE_PROHIBITED_FLAGS,
    load_matrix,
    validate_matrix,
)


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

    def test_ambient_workspace_read_stays_prohibited_after_the_narrowing(self) -> None:
        """The narrowed permission must not reintroduce a general workspace read."""
        contract = self.matrix["vscode_contract"]
        self.assertIn("workspace-read", contract["extension_prohibited_authority"])
        self.assertIn("workspace-write", contract["extension_prohibited_authority"])
        self.assertIn("request-bound-reference-resolution", contract["extension_authority"])

    def test_every_ambient_capability_flag_must_remain_false(self) -> None:
        for flag in REQUEST_REFERENCE_PROHIBITED_FLAGS:
            with self.subTest(flag=flag):
                mutated = copy.deepcopy(self.matrix)
                mutated["vscode_contract"]["request_reference_contract"][flag] = True
                self.assertIn(
                    f"request reference {flag} must remain false",
                    validate_matrix(mutated),
                )

    def test_request_reference_scope_cannot_widen(self) -> None:
        for key, expected in REQUEST_REFERENCE_EXACT_VALUES.items():
            with self.subTest(key=key):
                mutated = copy.deepcopy(self.matrix)
                mutated["vscode_contract"]["request_reference_contract"][key] = "anything-else"
                self.assertIn(
                    f"request reference {key} must equal {expected}",
                    validate_matrix(mutated),
                )

    def test_request_reference_kinds_are_a_closed_set(self) -> None:
        for mutation in (
            REQUEST_REFERENCE_KINDS + ["workspace-glob"],
            REQUEST_REFERENCE_KINDS[:-1],
            ["ambient-workspace-file"],
            [],
        ):
            with self.subTest(mutation=tuple(mutation)):
                mutated = copy.deepcopy(self.matrix)
                mutated["vscode_contract"]["request_reference_contract"][
                    "permitted_reference_kinds"
                ] = mutation
                self.assertIn(
                    "request reference kinds must be the exact closed current-request set",
                    validate_matrix(mutated),
                )

    def test_request_reference_contract_cannot_be_removed_or_widened_by_new_keys(self) -> None:
        removed = copy.deepcopy(self.matrix)
        del removed["vscode_contract"]["request_reference_contract"]
        self.assertIn(
            "vscode_contract.request_reference_contract must be an object",
            validate_matrix(removed),
        )
        widened = copy.deepcopy(self.matrix)
        widened["vscode_contract"]["request_reference_contract"]["workspace_glob"] = "**/*"
        self.assertIn(
            "unknown request reference contract keys: workspace_glob",
            validate_matrix(widened),
        )

    def test_extension_cannot_drop_request_bound_reference_authority(self) -> None:
        mutated = copy.deepcopy(self.matrix)
        mutated["vscode_contract"]["extension_authority"].remove(
            "request-bound-reference-resolution"
        )
        self.assertIn(
            "the VS Code extension must declare request-bound-reference-resolution authority",
            validate_matrix(mutated),
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
