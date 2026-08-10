from __future__ import annotations

import copy
import unittest

from scripts.dependency_rules import load_rules, validate_rules
from scripts.module_inventory import load_inventory


class DependencyRuleTests(unittest.TestCase):
    def setUp(self) -> None:
        self.rules = load_rules()
        self.inventory = load_inventory()

    def policy(self, rules: dict, module_id: str) -> dict:
        return next(item for item in rules["module_rules"] if item["id"] == module_id)

    def test_canonical_dependency_graph_passes(self) -> None:
        self.assertEqual(validate_rules(self.rules, self.inventory), [])

    def test_kernel_contracts_cannot_import_any_product_module(self) -> None:
        for target in (
            "kernel-engine",
            "platform-linux",
            "capability-read-only",
            "shell-host",
            "shell-vscode",
        ):
            with self.subTest(target=target):
                mutated = copy.deepcopy(self.rules)
                policy = self.policy(mutated, "kernel-contracts")
                policy["allowed_imports"].append(target)
                policy["declared_imports"].append(target)
                self.assertTrue(validate_rules(mutated, self.inventory))

    def test_kernel_engine_reverse_edges_are_rejected_separately(self) -> None:
        for target in (
            "platform-linux",
            "platform-macos",
            "capability-read-only",
            "shell-host",
            "shell-vscode",
        ):
            with self.subTest(target=target):
                mutated = copy.deepcopy(self.rules)
                policy = self.policy(mutated, "kernel-engine")
                policy["allowed_imports"].append(target)
                policy["declared_imports"].append(target)
                failures = validate_rules(mutated, self.inventory)
                self.assertTrue(failures)

    def test_platform_and_capability_cross_dependencies_are_rejected(self) -> None:
        for source, target in (
            ("platform-linux", "capability-read-only"),
            ("platform-macos", "shell-host"),
            ("capability-read-only", "platform-linux"),
            ("capability-read-only", "shell-vscode"),
        ):
            with self.subTest(source=source, target=target):
                mutated = copy.deepcopy(self.rules)
                policy = self.policy(mutated, source)
                policy["allowed_imports"].append(target)
                policy["declared_imports"].append(target)
                self.assertTrue(validate_rules(mutated, self.inventory))

    def test_import_outside_allowlist_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.rules)
        policy = self.policy(mutated, "kernel-engine")
        policy["declared_imports"].append("shell-host")
        failures = validate_rules(mutated, self.inventory)
        self.assertTrue(any("outside its allowlist" in item for item in failures))

    def test_compile_cycle_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.rules)
        policy = self.policy(mutated, "kernel-contracts")
        policy["allowed_imports"].append("kernel-engine")
        policy["declared_imports"].append("kernel-engine")
        failures = validate_rules(mutated, self.inventory)
        self.assertTrue(any("compile dependency cycle" in item for item in failures))

    def test_linux_package_cannot_consume_macos_adapter(self) -> None:
        mutated = copy.deepcopy(self.rules)
        policy = self.policy(mutated, "packaging-linux")
        policy["assembly_inputs"].append("platform-macos")
        self.assertTrue(validate_rules(mutated, self.inventory))

    def test_rules_and_inventory_must_cover_the_same_modules(self) -> None:
        mutated = copy.deepcopy(self.inventory)
        mutated["modules"].pop()
        failures = validate_rules(self.rules, mutated)
        self.assertTrue(any("identical module ids" in item for item in failures))


if __name__ == "__main__":
    unittest.main()
