from __future__ import annotations

import copy
import tempfile
import unittest
from pathlib import Path

from scripts.module_inventory import ROOT, load_inventory, validate_inventory


class ModuleInventoryTests(unittest.TestCase):
    def setUp(self) -> None:
        self.inventory = load_inventory()

    def test_canonical_inventory_matches_repository(self) -> None:
        self.assertEqual(validate_inventory(self.inventory), [])

    def test_missing_module_record_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.inventory)
        mutated["modules"].pop()
        self.assertTrue(validate_inventory(mutated))

    def test_duplicate_module_identity_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.inventory)
        mutated["modules"].append(copy.deepcopy(mutated["modules"][0]))
        failures = validate_inventory(mutated)
        self.assertTrue(any("duplicate module id" in failure for failure in failures))

    def test_path_traversal_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.inventory)
        mutated["modules"][0]["path"] = "../outside"
        self.assertTrue(validate_inventory(mutated))

    def test_language_or_build_system_drift_is_rejected(self) -> None:
        for field, value in (("language", "python"), ("build_system", "pip")):
            with self.subTest(field=field):
                mutated = copy.deepcopy(self.inventory)
                mutated["modules"][0][field] = value
                self.assertTrue(validate_inventory(mutated))

    def test_missing_directories_and_markers_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            empty_root = Path(temporary)
            failures = validate_inventory(self.inventory, empty_root)
        self.assertTrue(any("missing category directory" in item for item in failures))
        self.assertTrue(any("missing module directory" in item for item in failures))

    def test_existing_paths_are_bounded_to_repository(self) -> None:
        for module in self.inventory["modules"]:
            resolved = (ROOT / module["path"]).resolve()
            resolved.relative_to(ROOT)


if __name__ == "__main__":
    unittest.main()
