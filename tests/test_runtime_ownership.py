from __future__ import annotations

import copy
import unittest

from scripts.runtime_ownership import (
    EXPECTED_LAYERS,
    SINGLETON_ROLES,
    load_inventory,
    load_ownership,
    validate_ownership,
)


class RuntimeOwnershipTests(unittest.TestCase):
    def setUp(self) -> None:
        self.record = load_ownership()
        self.inventory = load_inventory()

    def _validate(self, record: dict) -> list[str]:
        return validate_ownership(record, self.inventory)

    def _role(self, record: dict, role_id: str) -> dict:
        return next(item for item in record["singleton_roles"] if item["id"] == role_id)

    def _layer(self, record: dict, layer_id: str) -> dict:
        return next(item for item in record["layers"] if item["id"] == layer_id)

    def test_canonical_record_passes(self) -> None:
        self.assertEqual(self._validate(self.record), [])

    def test_all_six_singleton_roles_are_recorded_once(self) -> None:
        self.assertEqual(
            [role["id"] for role in self.record["singleton_roles"]], SINGLETON_ROLES
        )
        self.assertEqual([layer["id"] for layer in self.record["layers"]], EXPECTED_LAYERS)

    def test_a_second_owner_is_rejected_for_every_singleton_role(self) -> None:
        """A duplicate context manager, store, loop, policy engine, dispatcher, or
        verifier must be rejected."""
        for role_id in SINGLETON_ROLES:
            with self.subTest(role=role_id):
                mutated = copy.deepcopy(self.record)
                self._role(mutated, role_id)["owners"].append("capability-knowledge")
                self.assertIn(
                    f"role {role_id} must have exactly one owner, found 2",
                    self._validate(mutated),
                )

    def test_no_singleton_role_may_move_outside_the_kernel(self) -> None:
        outside = {
            "capability": "capability-knowledge",
            "platform": "platform-linux",
            "shell": "shell-vscode",
        }
        for role_id in SINGLETON_ROLES:
            for layer_id, module_id in outside.items():
                with self.subTest(role=role_id, layer=layer_id):
                    mutated = copy.deepcopy(self.record)
                    self._role(mutated, role_id)["owners"] = [module_id]
                    self.assertIn(
                        f"role {role_id} cannot be owned by the {layer_id} layer",
                        self._validate(mutated),
                    )

    def test_a_role_cannot_be_left_unowned_or_dropped(self) -> None:
        for role_id in SINGLETON_ROLES:
            with self.subTest(role=role_id):
                emptied = copy.deepcopy(self.record)
                self._role(emptied, role_id)["owners"] = []
                self.assertIn(
                    f"role {role_id} must have exactly one owner, found 0",
                    self._validate(emptied),
                )
                dropped = copy.deepcopy(self.record)
                dropped["singleton_roles"] = [
                    item for item in dropped["singleton_roles"] if item["id"] != role_id
                ]
                self.assertTrue(self._validate(dropped))

    def test_a_duplicate_role_entry_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.record)
        mutated["singleton_roles"].append(
            {"id": "runtime-loop", "owners": ["kernel-engine"]}
        )
        self.assertIn("duplicate singleton role: runtime-loop", self._validate(mutated))

    def test_a_new_role_cannot_be_invented(self) -> None:
        mutated = copy.deepcopy(self.record)
        mutated["singleton_roles"].append(
            {"id": "second-runtime-loop", "owners": ["kernel-engine"]}
        )
        self.assertTrue(self._validate(mutated))

    def test_no_runtime_module_may_be_claimed_by_two_layers(self) -> None:
        mutated = copy.deepcopy(self.record)
        shell = self._layer(mutated, "shell")
        shell["modules"] = sorted([*shell["modules"], "kernel-engine"])
        shell["languages"] = ["rust", "typescript"]
        self.assertIn(
            "module kernel-engine is claimed by more than one layer: kernel, shell",
            self._validate(mutated),
        )

    def test_the_kernel_alone_may_own_singleton_roles(self) -> None:
        for layer_id in EXPECTED_LAYERS:
            with self.subTest(layer=layer_id):
                mutated = copy.deepcopy(self.record)
                layer = self._layer(mutated, layer_id)
                layer["may_own_singleton_roles"] = layer_id != "kernel"
                self.assertIn(
                    f"layer {layer_id} may_own_singleton_roles must be "
                    f"{layer_id == 'kernel'}",
                    self._validate(mutated),
                )

    def test_an_unimplemented_layer_cannot_claim_modules(self) -> None:
        mutated = copy.deepcopy(self.record)
        mcp = self._layer(mutated, "mcp")
        mcp["modules"] = ["kernel-engine"]
        mcp["languages"] = ["rust"]
        self.assertIn(
            "unimplemented layer mcp must declare no modules", self._validate(mutated)
        )

    def test_ownership_cannot_name_a_module_that_does_not_exist(self) -> None:
        role = copy.deepcopy(self.record)
        self._role(role, "policy-engine")["owners"] = ["kernel-shadow-engine"]
        self.assertIn(
            "role policy-engine names unknown owner kernel-shadow-engine",
            self._validate(role),
        )
        layer = copy.deepcopy(self.record)
        kernel = self._layer(layer, "kernel")
        kernel["modules"] = sorted([*kernel["modules"], "kernel-shadow-engine"])
        self.assertIn(
            "layer kernel names unknown module kernel-shadow-engine",
            self._validate(layer),
        )

    def test_every_inventory_module_needs_an_ownership_decision(self) -> None:
        mutated = copy.deepcopy(self.record)
        mutated["unassigned_modules"] = [
            item for item in mutated["unassigned_modules"] if item != "release-xtask"
        ]
        self.assertIn(
            "modules missing an ownership decision: release-xtask",
            self._validate(mutated),
        )

    def test_a_module_cannot_be_owned_and_unassigned_at_once(self) -> None:
        mutated = copy.deepcopy(self.record)
        mutated["unassigned_modules"] = sorted(
            [*mutated["unassigned_modules"], "kernel-engine"]
        )
        self.assertIn(
            "modules both owned and unassigned: kernel-engine", self._validate(mutated)
        )

    def test_layer_languages_must_match_the_inventory(self) -> None:
        mutated = copy.deepcopy(self.record)
        self._layer(mutated, "shell")["languages"] = ["rust"]
        self.assertIn(
            "layer shell languages must equal its modules' languages: rust, typescript",
            self._validate(mutated),
        )

    def test_layers_cannot_be_widened_by_unknown_keys(self) -> None:
        mutated = copy.deepcopy(self.record)
        self._layer(mutated, "capability")["owns_runtime_loop"] = True
        self.assertIn(
            "layer capability has unknown keys: owns_runtime_loop",
            self._validate(mutated),
        )


if __name__ == "__main__":
    unittest.main()
