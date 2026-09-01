from __future__ import annotations

import copy
import json
import unittest

from scripts.runtime_feature_activation import MANIFEST, UNAVAILABLE, validate_manifest


class RuntimeFeatureActivationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))

    def test_current_manifest_is_closed(self) -> None:
        self.assertEqual(validate_manifest(self.manifest), [])

    def test_unavailable_features_cannot_register_or_enable(self) -> None:
        for feature_id in UNAVAILABLE:
            with self.subTest(feature_id=feature_id):
                changed = copy.deepcopy(self.manifest)
                item = next(item for item in changed["features"] if item["id"] == feature_id)
                item["available"] = True
                item["default_enabled"] = True
                item["registration"] = "invented"
                self.assertTrue(validate_manifest(changed))

    def test_unknown_reordered_or_weakened_inventory_fails(self) -> None:
        changed = copy.deepcopy(self.manifest)
        changed["features"].reverse()
        self.assertTrue(validate_manifest(changed))
        changed = copy.deepcopy(self.manifest)
        changed["features"].append(copy.deepcopy(changed["features"][0]))
        self.assertTrue(validate_manifest(changed))
        changed = copy.deepcopy(self.manifest)
        changed["disabled_invariants"].remove("no_client_registration")
        self.assertTrue(validate_manifest(changed))


if __name__ == "__main__":
    unittest.main()
